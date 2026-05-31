use crate::catalog::load_embedded_catalog;
use crate::detection::{scan_catalog, SystemCommandRunner};
use crate::execution;
use crate::models::{
    CatalogRefreshResult, ConfigChangeTemplate, DetectionResult, ExecutionRequest, ExecutionResult,
    InstallPlan, NetworkProfile, OperatingSystem, PlanSelection, RollbackResult,
};
use crate::planner;
use serde::Serialize;
use tauri::Emitter;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExecutionLogEvent {
    step_id: String,
    line: String,
}

#[tauri::command]
pub fn scan_environment() -> Result<Vec<DetectionResult>, String> {
    let catalog = load_embedded_catalog().map_err(|error| error.to_string())?;
    let runner = SystemCommandRunner;
    Ok(scan_catalog(
        &catalog.items,
        OperatingSystem::current(),
        &runner,
    ))
}

#[tauri::command]
pub fn refresh_catalog_versions() -> Result<CatalogRefreshResult, String> {
    let catalog = load_embedded_catalog().map_err(|error| error.to_string())?;
    Ok(CatalogRefreshResult {
        catalog,
        refreshed_online: false,
        message: "Using embedded catalog; online refresh hooks are ready for official APIs."
            .to_string(),
    })
}

#[tauri::command]
pub fn build_install_plan(
    selections: Vec<PlanSelection>,
    network_profile: NetworkProfile,
) -> Result<InstallPlan, String> {
    let catalog = load_embedded_catalog().map_err(|error| error.to_string())?;
    planner::build_install_plan(
        &catalog,
        &selections,
        &network_profile,
        OperatingSystem::current(),
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn execute_install_step(
    app: tauri::AppHandle,
    request: ExecutionRequest,
) -> Result<ExecutionResult, String> {
    let step_id = request.step.id.clone();
    let runner = execution::SystemProcessRunner;
    execution::execute_install_step_with_runner_and_observer(
        request,
        OperatingSystem::current(),
        &runner,
        &mut |line| {
            let _ = app.emit(
                "elvpath-install-log",
                ExecutionLogEvent {
                    step_id: step_id.clone(),
                    line: line.to_string(),
                },
            );
        },
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn rollback_config_change(change: ConfigChangeTemplate) -> Result<RollbackResult, String> {
    Ok(execution::rollback_config_change(change))
}

#[tauri::command]
pub fn save_network_profile(profile: NetworkProfile) -> Result<NetworkProfile, String> {
    Ok(profile)
}
