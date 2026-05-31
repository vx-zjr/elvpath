pub mod catalog;
pub mod commands;
pub mod detection;
pub mod execution;
pub mod models;
pub mod planner;
pub mod typegen;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::scan_environment,
            commands::refresh_catalog_versions,
            commands::build_install_plan,
            commands::execute_install_step,
            commands::rollback_config_change,
            commands::save_network_profile
        ])
        .run(tauri::generate_context!())
        .expect("error while running ElvPath");
}
