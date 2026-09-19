use super::Memory;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

const FRAGMENT_TOKEN: &str = "agentsync:fragment";
const MANAGED_START: &str = "<!-- agentsync:managed memory-banner -->";
const MANAGED_END: &str = "<!-- /agentsync:managed memory-banner -->";
const MAX_IMPORT_DEPTH: usize = 16;

#[derive(Debug)]
struct Frame {
    name: String,
    lines: Vec<String>,
}

pub(crate) fn expand_memory_imports(body: &str, fragments: &BTreeMap<String, String>) -> String {
    let markers = !body.contains(FRAGMENT_TOKEN)
        && fragments
            .values()
            .all(|content| !content.contains(FRAGMENT_TOKEN));
    expand(body, fragments, &mut BTreeSet::new(), 0, markers)
}

pub(crate) fn render_managed_memory(
    body: &str,
    fragments: &BTreeMap<String, String>,
    destination_name: &str,
    banner: bool,
) -> String {
    let expanded = expand_memory_imports(body, fragments);
    if !banner || destination_name.is_empty() || expanded.contains("agentsync:managed") {
        return expanded;
    }
    format!("{}\n\n{}", managed_banner(destination_name), expanded)
}

pub(crate) fn strip_managed_banner(body: &str) -> String {
    let Some(start) = body.find(MANAGED_START) else {
        return body.to_string();
    };
    let Some(relative_end) = body[start..].find(MANAGED_END) else {
        return body.to_string();
    };
    let end = start + relative_end + MANAGED_END.len();
    let block = &body[start..end];
    if !is_managed_banner(block) {
        return body.to_string();
    }
    let mut suffix = &body[end..];
    if let Some(rest) = suffix.strip_prefix("\r\n\r\n") {
        suffix = rest;
    } else if let Some(rest) = suffix.strip_prefix("\n\n") {
        suffix = rest;
    } else if let Some(rest) = suffix.strip_prefix("\r\n") {
        suffix = rest;
    } else if let Some(rest) = suffix.strip_prefix('\n') {
        suffix = rest;
    }
    format!("{}{}", &body[..start], suffix)
}

pub(crate) fn collapse_memory_markers(body: &str) -> Result<Option<Memory>, String> {
    if !body.contains(FRAGMENT_TOKEN) {
        return Ok(None);
    }
    let mut body_lines = Vec::new();
    let mut stack: Vec<Frame> = Vec::new();
    let mut fragments = BTreeMap::new();
    for line in body.split('\n') {
        if let Some(name) = marker_name(line, "<!-- agentsync:fragment ", " -->") {
            validate_fragment_name(name)?;
            emit_line(
                &mut body_lines,
                &mut stack,
                format!("@import ./fragments/{name}"),
            );
            stack.push(Frame {
                name: name.to_string(),
                lines: Vec::new(),
            });
            continue;
        }
        if let Some(name) = marker_name(line, "<!-- /agentsync:fragment ", " -->") {
            let Some(frame) = stack.pop() else {
                return Err(format!("unbalanced memory fragment marker for {name:?}"));
            };
            if frame.name != name {
                return Err(format!("unbalanced memory fragment marker for {name:?}"));
            }
            let content = format!("{}\n", frame.lines.join("\n").trim_end_matches('\n'));
            if let Some(previous) = fragments.insert(name.to_string(), content.clone()) {
                if previous != content {
                    return Err(format!(
                        "memory fragment {name:?} appears multiple times with differing content"
                    ));
                }
            }
            continue;
        }
        emit_line(&mut body_lines, &mut stack, line.to_string());
    }
    if let Some(frame) = stack.last() {
        return Err(format!(
            "unterminated memory fragment marker for {:?}",
            frame.name
        ));
    }
    let mut collapsed_body = body_lines.join("\n");
    if !collapsed_body.is_empty() && !collapsed_body.ends_with('\n') {
        collapsed_body.push('\n');
    }
    Ok(Some(Memory {
        body: collapsed_body,
        fragments,
    }))
}

fn expand(
    body: &str,
    fragments: &BTreeMap<String, String>,
    visiting: &mut BTreeSet<String>,
    depth: usize,
    markers: bool,
) -> String {
    if depth >= MAX_IMPORT_DEPTH {
        return body.to_string();
    }
    let mut output = String::new();
    for line in body.split_inclusive('\n') {
        let has_newline = line.ends_with('\n');
        let content = line.strip_suffix('\n').unwrap_or(line);
        let trimmed = content.trim();
        let name = trimmed
            .strip_prefix("@import ./fragments/")
            .filter(|name| !name.is_empty() && !name.chars().any(char::is_whitespace));
        let Some(name) = name else {
            output.push_str(line);
            continue;
        };
        let Some(fragment) = fragments.get(name) else {
            output.push_str(line);
            continue;
        };
        if visiting.contains(name) {
            output.push_str(line);
            continue;
        }
        visiting.insert(name.to_string());
        let expanded = expand(fragment, fragments, visiting, depth + 1, markers);
        visiting.remove(name);
        let expanded = expanded.trim_end_matches('\n');
        if markers {
            output.push_str(&format!(
                "<!-- agentsync:fragment {name} -->\n{expanded}\n<!-- /agentsync:fragment {name} -->"
            ));
        } else {
            output.push_str(expanded);
        }
        if has_newline {
            output.push('\n');
        }
    }
    output
}

fn managed_banner(destination_name: &str) -> String {
    format!(
        "{MANAGED_START}\n> **Managed by [agentsync](https://agentsync.cc) — do not edit `{destination_name}` directly.**\n> To change it, edit `.agentsync/memory/AGENTS.md` (or the relevant\n> `.agentsync/memory/fragments/*.md` fragment) and run `agentsync apply`.\n> Direct edits here are reported as drift and overwritten on the next apply.\n{MANAGED_END}"
    )
}

fn is_managed_banner(block: &str) -> bool {
    let lines: Vec<_> = block.lines().collect();
    lines.len() == 6
        && lines[0] == MANAGED_START
        && lines[1].starts_with("> **Managed by [agentsync](https://agentsync.cc) — do not edit `")
        && lines[1].ends_with("` directly.**")
        && lines[2] == "> To change it, edit `.agentsync/memory/AGENTS.md` (or the relevant"
        && lines[3] == "> `.agentsync/memory/fragments/*.md` fragment) and run `agentsync apply`."
        && lines[4]
            == "> Direct edits here are reported as drift and overwritten on the next apply."
        && lines[5] == MANAGED_END
}

fn marker_name<'a>(line: &'a str, prefix: &str, suffix: &str) -> Option<&'a str> {
    line.strip_prefix(prefix)?.strip_suffix(suffix)
}

fn validate_fragment_name(name: &str) -> Result<(), String> {
    let path = Path::new(name);
    if name.is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!("fragment name {name:?} escapes memory/fragments"));
    }
    Ok(())
}

fn emit_line(body: &mut Vec<String>, stack: &mut [Frame], line: String) {
    if let Some(frame) = stack.last_mut() {
        frame.lines.push(line);
    } else {
        body.push(line);
    }
}

#[cfg(test)]
mod tests {
    use super::{collapse_memory_markers, render_managed_memory, strip_managed_banner};
    use std::collections::BTreeMap;

    #[test]
    fn managed_fragment_render_is_reversible() {
        let body = "# Root\n\n@import ./fragments/style.md\n";
        let fragments =
            BTreeMap::from([("style.md".to_string(), "Use current styles.\n".to_string())]);
        let rendered = render_managed_memory(body, &fragments, "AGENTS.md", true);
        assert!(rendered.contains("Managed by [agentsync]"));
        assert!(rendered.contains("<!-- agentsync:fragment style.md -->"));

        let stripped = strip_managed_banner(&rendered);
        let collapsed = collapse_memory_markers(&stripped).unwrap().unwrap();
        assert_eq!(collapsed.body, body);
        assert_eq!(collapsed.fragments, fragments);
    }

    #[test]
    fn malformed_fragment_marker_is_rejected() {
        let error = collapse_memory_markers(
            "<!-- agentsync:fragment ../escape.md -->\nsecret\n<!-- /agentsync:fragment ../escape.md -->\n",
        )
        .unwrap_err();
        assert!(error.contains("escapes"));
    }
}
