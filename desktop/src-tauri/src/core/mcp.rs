use crate::core::source::McpServer;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RecipeDescriptor {
    pub(crate) id: String,
    pub(crate) runner: String,
    pub(crate) package: String,
    pub(crate) args: Vec<String>,
    pub(crate) status: String,
    pub(crate) required_secrets: Vec<String>,
}

pub(crate) fn recipe_for(server: &McpServer) -> RecipeDescriptor {
    let recipe_id = if !server.recipe.is_empty() {
        server.recipe.as_str()
    } else if matches!(server.id.as_str(), "f2c-mcp" | "chrome-devtools") {
        server.id.as_str()
    } else {
        ""
    };
    let builtin = match recipe_id {
        "f2c-mcp" => Some(("npx", "@f2c/mcp", Vec::new(), vec!["personalToken"])),
        "chrome-devtools" => Some((
            "npx",
            "chrome-devtools-mcp@latest",
            vec![
                "--no-usage-statistics".to_string(),
                "--no-performance-crux".to_string(),
            ],
            Vec::new(),
        )),
        _ => None,
    };
    if let Some((runner, package, default_args, required_secrets)) = builtin {
        return RecipeDescriptor {
            id: recipe_id.to_string(),
            runner: if server.runner.is_empty() {
                runner.to_string()
            } else {
                server.runner.clone()
            },
            package: if server.package.is_empty() {
                package.to_string()
            } else {
                server.package.clone()
            },
            args: if server.args.is_empty() {
                default_args
            } else {
                server.args.clone()
            },
            status: if required_secrets.iter().all(|key| {
                server
                    .env
                    .get(*key)
                    .is_some_and(|value| is_secret_ref(value))
            }) {
                "ready".to_string()
            } else if required_secrets
                .iter()
                .any(|key| server.env.contains_key(*key))
            {
                "plaintext-secret".to_string()
            } else {
                "missing-secret".to_string()
            },
            required_secrets: required_secrets
                .into_iter()
                .map(ToString::to_string)
                .collect(),
        };
    }
    RecipeDescriptor {
        id: server.recipe.clone(),
        runner: server.runner.clone(),
        package: server.package.clone(),
        args: server.args.clone(),
        status: if server.recipe.is_empty() {
            if server.command.is_empty() && server.url.is_empty() {
                "missing-recipe"
            } else {
                "legacy"
            }
        } else {
            "unknown-recipe"
        }
        .to_string(),
        required_secrets: Vec::new(),
    }
}

pub(crate) fn credential_state(server: &McpServer) -> &'static str {
    let secret_values = server
        .env
        .iter()
        .chain(server.headers.iter())
        .filter(|(key, _)| is_secret_key(key));
    let mut has_secret = false;
    for (_, value) in secret_values {
        has_secret = true;
        if !is_secret_ref(value) {
            return "plaintext-blocked";
        }
    }
    if has_secret {
        "configured"
    } else {
        "not-required"
    }
}

pub(crate) fn is_secret_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    [
        "token",
        "key",
        "secret",
        "password",
        "authorization",
        "credential",
    ]
    .iter()
    .any(|part| key.contains(part))
}

pub(crate) fn is_secret_ref(value: &str) -> bool {
    value.starts_with("${secret:") || value.starts_with("${env:")
}

/// 判断 command/url 是否为 `safe_endpoint()` 生成的展示占位符字符串。
/// 这类字符串只能用于 UI 展示，绝不能写入运行时配置或用于执行。
pub(crate) fn is_display_placeholder(value: &str) -> bool {
    value == "本地命令（已脱敏）" || value == "远程端点（已脱敏）"
}

pub(crate) fn sanitize_imported_credentials(server: &mut McpServer) {
    for (key, value) in server.env.iter_mut().chain(server.headers.iter_mut()) {
        if is_secret_key(key) && !value.is_empty() && !is_secret_ref(value) {
            *value = format!("${{env:{key}}}");
        }
    }
}

pub(crate) fn validate_global_promotion(server: &McpServer) -> Result<(), String> {
    if credential_state(server) == "plaintext-blocked" {
        return Err(format!(
            "refusing to promote MCP {}: secret-like values must use ${{secret:…}} or ${{env:…}} references",
            server.id
        ));
    }
    let recipe = recipe_for(server);
    if matches!(
        recipe.status.as_str(),
        "missing-recipe" | "unknown-recipe" | "legacy" | "missing-secret" | "plaintext-secret"
    ) {
        return Err(format!(
            "refusing to promote MCP {}: recipe status is {}",
            server.id, recipe.status
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        credential_state, recipe_for, sanitize_imported_credentials, validate_global_promotion,
    };
    use crate::core::source::McpServer;
    use std::collections::BTreeMap;

    fn server(id: &str) -> McpServer {
        McpServer {
            id: id.to_string(),
            name: id.to_string(),
            ..McpServer::default()
        }
    }

    #[test]
    fn f2c_recipe_requires_a_secret_reference() {
        let mut item = server("f2c-mcp");
        assert_eq!(recipe_for(&item).status, "missing-secret");
        item.env = BTreeMap::from([(
            "personalToken".to_string(),
            "${secret:F2C_TOKEN}".to_string(),
        )]);
        assert_eq!(recipe_for(&item).package, "@f2c/mcp");
        assert_eq!(recipe_for(&item).status, "ready");
        assert!(validate_global_promotion(&item).is_ok());
    }

    #[test]
    fn tokenless_chrome_recipe_is_ready_without_credentials() {
        let item = server("chrome-devtools");
        let recipe = recipe_for(&item);
        assert_eq!(recipe.status, "ready");
        assert!(recipe.required_secrets.is_empty());
        assert!(validate_global_promotion(&item).is_ok());
    }

    #[test]
    fn plaintext_secret_is_blocked_from_promotion() {
        let mut item = server("f2c-mcp");
        item.env = BTreeMap::from([("personalToken".to_string(), "figd_private".to_string())]);
        assert_eq!(credential_state(&item), "plaintext-blocked");
        assert!(validate_global_promotion(&item).is_err());
    }

    #[test]
    fn sanitizes_plaintext_credentials_before_native_import() {
        let mut item = server("f2c-mcp");
        item.env = BTreeMap::from([
            ("PATH".to_string(), "/usr/bin".to_string()),
            ("personalToken".to_string(), "figd_private".to_string()),
        ]);
        item.headers =
            BTreeMap::from([("Authorization".to_string(), "Bearer private".to_string())]);

        sanitize_imported_credentials(&mut item);

        assert_eq!(item.env.get("PATH"), Some(&"/usr/bin".to_string()));
        assert_eq!(
            item.env.get("personalToken"),
            Some(&"${env:personalToken}".to_string())
        );
        assert_eq!(
            item.headers.get("Authorization"),
            Some(&"${env:Authorization}".to_string())
        );
        assert_eq!(credential_state(&item), "configured");
        assert!(validate_global_promotion(&item).is_ok());
    }

    #[test]
    fn legacy_mcp_without_recipe_is_blocked_from_promotion() {
        let mut item = server("legacy-server");
        item.command = "legacy-server".to_string();
        assert_eq!(recipe_for(&item).status, "legacy");
        assert!(validate_global_promotion(&item).is_err());
    }
}
