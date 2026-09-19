mod memory;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path};

use crate::core::iox;

pub(crate) use memory::{collapse_memory_markers, render_managed_memory, strip_managed_banner};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct Agent {
    #[serde(default)]
    pub(crate) enabled: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub(crate) scope: String,
}

#[derive(Clone, Debug)]
pub(crate) struct Config {
    pub(crate) agents: BTreeMap<String, Agent>,
    pub(crate) memory_banner: bool,
    raw: toml::Value,
    existed: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            agents: BTreeMap::new(),
            memory_banner: true,
            raw: toml::Value::Table(toml::map::Map::new()),
            existed: false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Memory {
    pub(crate) body: String,
    pub(crate) fragments: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct McpServer {
    pub(crate) id: String,
    pub(crate) name: String,
    /// 用户自定义的功能简介，持久化到 TOML [server].description 字段
    pub(crate) description: String,
    pub(crate) transport: String,
    pub(crate) command: String,
    pub(crate) url: String,
    pub(crate) args: Vec<String>,
    pub(crate) recipe: String,
    pub(crate) runner: String,
    pub(crate) package: String,
    pub(crate) env: BTreeMap<String, String>,
    pub(crate) headers: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Canonical {
    pub(crate) config: Config,
    pub(crate) memory: Memory,
    pub(crate) mcp_servers: Vec<McpServer>,
    pub(crate) skills: usize,
    pub(crate) hooks: usize,
    pub(crate) subagents: usize,
}

pub(crate) fn load(home: &Path) -> Result<Canonical, String> {
    if !home.exists() {
        return Ok(Canonical::default());
    }
    let config = load_config(home)?;
    let memory = load_memory(home)?;
    let mcp_servers = load_mcp_servers(home)?;
    Ok(Canonical {
        config,
        memory,
        mcp_servers,
        skills: count_skill_directories(&home.join("skills"))?,
        hooks: count_files_with_extension(&home.join("hooks"), "toml")?,
        subagents: count_files_with_extension(&home.join("subagents"), "md")?,
    })
}

pub(crate) fn write_config(home: &Path, config: &Config) -> Result<(), String> {
    let mut raw = config.raw.clone();
    let root = raw
        .as_table_mut()
        .ok_or_else(|| "agentsync.toml root must be a table".to_string())?;
    let agents = root
        .entry("agents")
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()))
        .as_table_mut()
        .ok_or_else(|| "agentsync.toml [agents] must be a table".to_string())?;
    for (name, agent) in &config.agents {
        if !agents.contains_key(name) {
            let value = toml::Value::try_from(agent)
                .map_err(|error| format!("serialize Agent {name}: {error}"))?;
            agents.insert(name.clone(), value);
        }
    }
    let mut body =
        toml::to_string(&raw).map_err(|error| format!("serialize agentsync.toml: {error}"))?;
    if !body.ends_with('\n') {
        body.push('\n');
    }
    iox::atomic_write(&home.join("agentsync.toml"), body.as_bytes())
        .map_err(|error| format!("write agentsync.toml: {error}"))
}

pub(crate) fn write_memory(home: &Path, memory: &Memory) -> Result<(), String> {
    validate_reserved_markers(memory)?;
    let fragments_dir = home.join("memory/fragments");
    if memory.fragments.is_empty() && contains_regular_file(&fragments_dir)? {
        return Err("refusing to overwrite memory/AGENTS.md: canonical memory is composed of fragments/ and the value to write carries none".to_string());
    }
    iox::atomic_write(&home.join("memory/AGENTS.md"), memory.body.as_bytes())
        .map_err(|error| format!("write memory/AGENTS.md: {error}"))?;
    for (name, content) in &memory.fragments {
        validate_fragment_name(name)?;
        iox::atomic_write(&fragments_dir.join(name), content.as_bytes())
            .map_err(|error| format!("write memory fragment {name}: {error}"))?;
    }
    Ok(())
}

impl Config {
    pub(crate) fn existed(&self) -> bool {
        self.existed
    }

    pub(crate) fn insert_enabled_agent(&mut self, name: &str) -> bool {
        if self.agents.contains_key(name) {
            return false;
        }
        self.agents.insert(
            name.to_string(),
            Agent {
                enabled: true,
                scope: String::new(),
            },
        );
        true
    }
}

fn load_config(home: &Path) -> Result<Config, String> {
    let path = home.join("agentsync.toml");
    let body = match fs::read_to_string(&path) {
        Ok(body) => body,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Config::default()),
        Err(error) => return Err(format!("read {}: {error}", path.display())),
    };
    let raw: toml::Value = body
        .parse()
        .map_err(|error| format!("parse {}: {error}", path.display()))?;
    let root = raw
        .as_table()
        .ok_or_else(|| format!("{} root must be a table", path.display()))?;
    let mut agents = BTreeMap::new();
    if let Some(agent_table) = root.get("agents").and_then(toml::Value::as_table) {
        for (name, value) in agent_table {
            let agent: Agent = value
                .clone()
                .try_into()
                .map_err(|error| format!("parse Agent {name}: {error}"))?;
            agents.insert(name.clone(), agent);
        }
    }
    let memory_banner = root
        .get("memory")
        .and_then(toml::Value::as_table)
        .and_then(|memory| memory.get("banner"))
        .and_then(toml::Value::as_bool)
        .unwrap_or(true);
    Ok(Config {
        agents,
        memory_banner,
        raw,
        existed: true,
    })
}

fn load_memory(home: &Path) -> Result<Memory, String> {
    let memory_path = home.join("memory/AGENTS.md");
    let body = match fs::read_to_string(&memory_path) {
        Ok(body) => body,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(format!("read {}: {error}", memory_path.display())),
    };
    let fragments_root = home.join("memory/fragments");
    let mut fragments = BTreeMap::new();
    collect_text_files(&fragments_root, &fragments_root, &mut fragments)?;
    let memory = Memory { body, fragments };
    validate_reserved_markers(&memory)?;
    Ok(memory)
}

fn load_mcp_servers(home: &Path) -> Result<Vec<McpServer>, String> {
    let root = home.join("mcp");
    let entries = match fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("read {}: {error}", root.display())),
    };
    let mut paths = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "toml")
        })
        .collect::<Vec<_>>();
    paths.sort();
    let mut servers = Vec::with_capacity(paths.len());
    for path in paths {
        let body = fs::read_to_string(&path)
            .map_err(|error| format!("read {}: {error}", path.display()))?;
        let raw: toml::Value = body
            .parse()
            .map_err(|error| format!("parse {}: {error}", path.display()))?;
        let server = raw
            .get("server")
            .and_then(toml::Value::as_table)
            .ok_or_else(|| format!("{} is missing [server]", path.display()))?;
        let id = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string();
        servers.push(McpServer {
            name: table_string(server, "name").if_empty_then(&id),
            id,
            description: table_string(server, "description"),
            transport: table_string(server, "type"),
            command: table_string(server, "command"),
            url: table_string(server, "url"),
            args: string_array(server.get("args")),
            recipe: table_string(server, "recipe"),
            runner: table_string(server, "runner"),
            package: table_string(server, "package"),
            env: string_table(server.get("env")),
            headers: string_table(server.get("headers")),
        });
    }
    Ok(servers)
}

fn table_string(table: &toml::map::Map<String, toml::Value>, key: &str) -> String {
    table
        .get(key)
        .and_then(toml::Value::as_str)
        .unwrap_or_default()
        .to_string()
}

trait StringFallback {
    fn if_empty_then(self, fallback: &str) -> String;
}

impl StringFallback for String {
    fn if_empty_then(self, fallback: &str) -> String {
        if self.is_empty() {
            fallback.to_string()
        } else {
            self
        }
    }
}

fn string_array(value: Option<&toml::Value>) -> Vec<String> {
    value
        .and_then(toml::Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(toml::Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn write_mcp_server(home: &Path, server: &McpServer) -> Result<(), String> {
    let id = server.id.trim();
    if id.is_empty() || id == "." || id == ".." || id.contains('/') || id.contains('\\') {
        return Err("MCP id is not a safe canonical filename".to_string());
    }
    let mut table = toml::map::Map::new();
    table.insert("name".to_string(), toml::Value::String(server.name.clone()));
    if !server.description.is_empty() {
        table.insert(
            "description".to_string(),
            toml::Value::String(server.description.clone()),
        );
    }
    if !server.transport.is_empty() {
        table.insert(
            "type".to_string(),
            toml::Value::String(server.transport.clone()),
        );
    }
    for (key, value) in [
        ("command", &server.command),
        ("url", &server.url),
        ("recipe", &server.recipe),
        ("runner", &server.runner),
        ("package", &server.package),
    ] {
        if !value.is_empty() {
            table.insert(key.to_string(), toml::Value::String(value.clone()));
        }
    }
    if !server.args.is_empty() {
        table.insert(
            "args".to_string(),
            toml::Value::Array(
                server
                    .args
                    .iter()
                    .cloned()
                    .map(toml::Value::String)
                    .collect(),
            ),
        );
    }
    if !server.env.is_empty() {
        table.insert("env".to_string(), string_table_value(&server.env));
    }
    if !server.headers.is_empty() {
        table.insert("headers".to_string(), string_table_value(&server.headers));
    }
    let mut root = toml::map::Map::new();
    root.insert("server".to_string(), toml::Value::Table(table));
    let mut body = toml::to_string(&toml::Value::Table(root))
        .map_err(|error| format!("serialize MCP {}: {error}", server.id))?;
    if !body.ends_with('\n') {
        body.push('\n');
    }
    iox::atomic_write(
        &home.join("mcp").join(format!("{id}.toml")),
        body.as_bytes(),
    )
    .map_err(|error| format!("write MCP {}: {error}", server.id))
}

fn string_table_value(values: &BTreeMap<String, String>) -> toml::Value {
    toml::Value::Table(
        values
            .iter()
            .map(|(key, value)| (key.clone(), toml::Value::String(value.clone())))
            .collect(),
    )
}

fn string_table(value: Option<&toml::Value>) -> BTreeMap<String, String> {
    value
        .and_then(toml::Value::as_table)
        .map(|table| {
            table
                .iter()
                .filter_map(|(key, value)| {
                    value.as_str().map(|value| (key.clone(), value.to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn count_skill_directories(root: &Path) -> Result<usize, String> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(format!("read {}: {error}", root.display())),
    };
    Ok(entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().join("SKILL.md").is_file())
        .count())
}

fn count_files_with_extension(root: &Path, extension: &str) -> Result<usize, String> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(format!("read {}: {error}", root.display())),
    };
    Ok(entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.path().is_file()
                && entry
                    .path()
                    .extension()
                    .is_some_and(|candidate| candidate == extension)
        })
        .count())
}

fn collect_text_files(
    root: &Path,
    directory: &Path,
    output: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("read {}: {error}", directory.display())),
    };
    for entry in entries {
        let entry = entry.map_err(|error| format!("read {}: {error}", directory.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| format!("inspect {}: {error}", path.display()))?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            collect_text_files(root, &path, output)?;
        } else if file_type.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|error| format!("resolve fragment path: {error}"))?
                .to_string_lossy()
                .replace('\\', "/");
            let content = fs::read_to_string(&path)
                .map_err(|error| format!("read {}: {error}", path.display()))?;
            output.insert(relative, content);
        }
    }
    Ok(())
}

fn contains_regular_file(root: &Path) -> Result<bool, String> {
    if !root.exists() {
        return Ok(false);
    }
    let mut files = BTreeMap::new();
    collect_text_files(root, root, &mut files)?;
    Ok(!files.is_empty())
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

fn validate_reserved_markers(memory: &Memory) -> Result<(), String> {
    if memory.body.contains("agentsync:managed") {
        return Err(
            "memory/AGENTS.md contains the reserved marker \"agentsync:managed\"".to_string(),
        );
    }
    for (name, content) in &memory.fragments {
        validate_fragment_name(name)?;
        if content.contains("agentsync:managed") {
            return Err(format!(
                "memory fragment {name:?} contains the reserved marker \"agentsync:managed\""
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{load, write_config, write_mcp_server, write_memory, McpServer, Memory};
    use crate::core::test_support::TestDir;
    use std::collections::BTreeMap;
    use std::fs;

    #[test]
    fn empty_home_loads_as_empty_canonical() {
        let dir = TestDir::new("source-empty");
        let canonical = load(&dir.path().join("missing")).unwrap();
        assert!(canonical.config.agents.is_empty());
        assert!(canonical.mcp_servers.is_empty());
        assert_eq!(canonical.memory, Memory::default());
    }

    #[test]
    fn config_update_preserves_unknown_fields() {
        let dir = TestDir::new("source-config-extra");
        let config_path = dir.path().join("agentsync.toml");
        fs::write(
            &config_path,
            "custom = \"keep\"\n\n[agents.codex]\nenabled = true\nfuture_key = \"keep agent data\"\n\n[future]\nanswer = 42\n",
        )
        .unwrap();

        let mut canonical = load(dir.path()).unwrap();
        assert!(canonical.config.existed());
        assert!(canonical.config.insert_enabled_agent("gemini"));
        write_config(dir.path(), &canonical.config).unwrap();

        let parsed: toml::Value = fs::read_to_string(config_path).unwrap().parse().unwrap();
        assert_eq!(parsed["custom"].as_str(), Some("keep"));
        assert_eq!(parsed["future"]["answer"].as_integer(), Some(42));
        assert_eq!(
            parsed["agents"]["codex"]["future_key"].as_str(),
            Some("keep agent data")
        );
        assert_eq!(parsed["agents"]["gemini"]["enabled"].as_bool(), Some(true));
    }

    #[test]
    fn memory_and_nested_fragments_round_trip() {
        let dir = TestDir::new("source-memory");
        let memory = Memory {
            body: "# Root\n\n@import ./fragments/style.md\n".to_string(),
            fragments: BTreeMap::from([
                (
                    "style.md".to_string(),
                    "Use existing styles.\n@import ./fragments/nested/more.md\n".to_string(),
                ),
                ("nested/more.md".to_string(), "Keep spacing.\n".to_string()),
            ]),
        };
        write_memory(dir.path(), &memory).unwrap();
        assert_eq!(load(dir.path()).unwrap().memory, memory);
    }

    #[test]
    fn flattened_write_cannot_orphan_existing_fragments() {
        let dir = TestDir::new("source-memory-guard");
        let memory = Memory {
            body: "@import ./fragments/one.md\n".to_string(),
            fragments: BTreeMap::from([("one.md".to_string(), "One.\n".to_string())]),
        };
        write_memory(dir.path(), &memory).unwrap();
        let error = write_memory(
            dir.path(),
            &Memory {
                body: "One.\n".to_string(),
                fragments: BTreeMap::new(),
            },
        )
        .unwrap_err();
        assert!(error.contains("composed of fragments"));
    }

    #[test]
    fn source_counts_desktop_components_and_reads_mcp_refs() {
        let dir = TestDir::new("source-components");
        fs::create_dir_all(dir.path().join("mcp")).unwrap();
        fs::create_dir_all(dir.path().join("skills/one")).unwrap();
        fs::create_dir_all(dir.path().join("hooks")).unwrap();
        fs::create_dir_all(dir.path().join("subagents")).unwrap();
        fs::write(
            dir.path().join("mcp/remote.toml"),
            "[server]\ntype = \"http\"\nurl = \"https://example.test/mcp?token=hidden\"\n[server.headers]\nAuthorization = \"${env:MCP_TOKEN}\"\n",
        )
        .unwrap();
        fs::write(dir.path().join("skills/one/SKILL.md"), "# One\n").unwrap();
        fs::write(
            dir.path().join("hooks/after.toml"),
            "[hook]\ncommand = \"ok\"\n",
        )
        .unwrap();
        fs::write(dir.path().join("subagents/reviewer.md"), "# Review\n").unwrap();

        let canonical = load(dir.path()).unwrap();
        assert_eq!(canonical.skills, 1);
        assert_eq!(canonical.hooks, 1);
        assert_eq!(canonical.subagents, 1);
        assert_eq!(canonical.mcp_servers.len(), 1);
        assert_eq!(
            canonical.mcp_servers[0].headers["Authorization"],
            "${env:MCP_TOKEN}"
        );
        assert_eq!(canonical.mcp_servers[0].name, "remote");
    }

    #[test]
    fn writes_global_mcp_recipe_without_plaintext_secret_values() {
        let dir = TestDir::new("source-write-mcp");
        let server = McpServer {
            id: "chrome-devtools".to_string(),
            name: "chrome-devtools".to_string(),
            recipe: "chrome-devtools".to_string(),
            runner: "npx".to_string(),
            package: "chrome-devtools-mcp@latest".to_string(),
            args: vec!["--no-usage-statistics".to_string()],
            ..McpServer::default()
        };
        write_mcp_server(dir.path(), &server).unwrap();
        let loaded = load(dir.path()).unwrap();
        assert_eq!(loaded.mcp_servers[0], server);
    }
}
