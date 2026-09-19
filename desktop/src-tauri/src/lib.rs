use serde::Serialize;

mod core;

use core::desktop::{
    ApplyPreview, Core, ImportNativeRuleRequest, McpPromotionResult, NativeMcpPromotionRequest,
    ProjectMcpPromotionRequest, ProjectPathRequest, ProjectRuleImport, PushMcpToAgentRequest,
    PushMcpToAgentResult, RuleDocument, RuleProposal, RuleRequest, RuleSyncResult, RuleWorkspace,
    SaveMcpRequest, SaveMcpResult, SaveRuleRequest, SyncRulesRequest, VerifyMcpInAgentRequest,
    VerifyMcpInAgentResult, WorkspaceSnapshot,
};

#[cfg(any(target_os = "macos", windows))]
use tauri::{
    menu::{Menu, MenuItem, MenuItemKind, PredefinedMenuItem},
    Emitter,
};

#[cfg(windows)]
use tauri::menu::HELP_SUBMENU_ID;

#[cfg(any(target_os = "macos", windows))]
const CHECK_FOR_UPDATES_MENU_ID: &str = "check-for-updates";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreHealth {
    pub status: &'static str,
    pub mode: &'static str,
}

#[tauri::command]
fn core_health() -> CoreHealth {
    CoreHealth {
        status: "ready",
        mode: "embedded",
    }
}

#[tauri::command]
fn get_workspace_snapshot() -> Result<WorkspaceSnapshot, String> {
    Core::from_env()?.read_snapshot()
}

#[tauri::command]
fn preview_apply() -> Result<ApplyPreview, String> {
    Core::from_env()?.preview_apply()
}

#[tauri::command]
fn rules_get(request: RuleRequest) -> Result<RuleWorkspace, String> {
    Core::from_env()?.get_rules(request)
}

#[tauri::command]
fn rules_preview_native(agent: String, path: String) -> Result<String, String> {
    Core::from_env()?.preview_native_rule(&agent, &path)
}

#[tauri::command]
fn rules_save(request: SaveRuleRequest) -> Result<RuleDocument, String> {
    Core::from_env()?.save_rule(request)
}

#[tauri::command]
fn rules_sync(request: SyncRulesRequest) -> Result<RuleSyncResult, String> {
    Core::from_env()?.sync_rules(request)
}

#[tauri::command]
fn rules_import_native(request: ImportNativeRuleRequest) -> Result<RuleDocument, String> {
    Core::from_env()?.import_native_rule(request)
}

#[tauri::command]
fn project_import(request: ProjectPathRequest) -> Result<ProjectRuleImport, String> {
    Core::from_env()?.import_project(&request.path)
}

#[tauri::command]
fn project_analyze_rules(request: ProjectPathRequest) -> Result<RuleProposal, String> {
    Core::from_env()?.analyze_project_rules(&request.path)
}

#[tauri::command]
fn mcp_promote_project(request: ProjectMcpPromotionRequest) -> Result<McpPromotionResult, String> {
    Core::from_env()?.promote_project_mcp(request)
}

#[tauri::command]
fn mcp_promote_native(request: NativeMcpPromotionRequest) -> Result<McpPromotionResult, String> {
    Core::from_env()?.promote_native_mcp(request)
}

#[tauri::command]
fn mcp_import_native_force(
    request: NativeMcpPromotionRequest,
) -> Result<McpPromotionResult, String> {
    Core::from_env()?.promote_native_mcp_force(request)
}

#[tauri::command]
fn mcp_save(request: SaveMcpRequest) -> Result<SaveMcpResult, String> {
    Core::from_env()?.save_global_mcp(request)
}

#[tauri::command]
fn mcp_push_to_agent(request: PushMcpToAgentRequest) -> Result<PushMcpToAgentResult, String> {
    Core::from_env()?.push_mcp_to_agent(request)
}

#[tauri::command]
fn mcp_verify_in_agent(request: VerifyMcpInAgentRequest) -> Result<VerifyMcpInAgentResult, String> {
    Core::from_env()?.verify_mcp_in_agent(request)
}

pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(any(target_os = "macos", windows))]
    let builder = if updater_enabled() {
        builder
            .plugin(tauri_plugin_updater::Builder::new().build())
            .plugin(tauri_plugin_process::init())
            .menu(desktop_menu)
            .on_menu_event(|app, event| {
                if event.id().as_ref() == CHECK_FOR_UPDATES_MENU_ID {
                    let _ = app.emit(CHECK_FOR_UPDATES_MENU_ID, ());
                }
            })
    } else {
        builder
    };

    builder
        .invoke_handler(tauri::generate_handler![
            core_health,
            get_workspace_snapshot,
            preview_apply,
            rules_get,
            rules_preview_native,
            rules_save,
            rules_sync,
            rules_import_native,
            project_import,
            project_analyze_rules,
            mcp_promote_project,
            mcp_promote_native,
            mcp_import_native_force,
            mcp_save,
            mcp_push_to_agent,
            mcp_verify_in_agent
        ])
        .run(tauri::generate_context!())
        .expect("error while running agent-assistant");
}

#[cfg(any(target_os = "macos", windows))]
fn updater_enabled() -> bool {
    option_env!("AGENT_ASSISTANT_UPDATER_ENABLED") == Some("true")
}

#[cfg(any(target_os = "macos", windows))]
fn desktop_menu(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let menu = Menu::default(app)?;
    let check_for_updates = MenuItem::with_id(
        app,
        CHECK_FOR_UPDATES_MENU_ID,
        "检查更新…",
        true,
        None::<&str>,
    )?;
    let separator = PredefinedMenuItem::separator(app)?;

    #[cfg(target_os = "macos")]
    if let Some(MenuItemKind::Submenu(app_menu)) = menu.items()?.into_iter().next() {
        app_menu.insert_items(&[&check_for_updates, &separator], 2)?;
    }

    #[cfg(windows)]
    if let Some(MenuItemKind::Submenu(help_menu)) = menu.get(HELP_SUBMENU_ID) {
        help_menu.insert_items(&[&check_for_updates, &separator], 0)?;
    }

    Ok(menu)
}

#[cfg(test)]
mod tests {
    use super::core_health;

    #[test]
    fn core_is_embedded_in_the_tauri_process() {
        let health = core_health();
        assert_eq!(health.status, "ready");
        assert_eq!(health.mode, "embedded");
    }
}
