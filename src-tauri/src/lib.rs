pub mod commands;
pub mod core;
pub mod infra;
pub mod services;
pub mod state;

use state::AppState;

use tauri::Manager;

pub fn run() {
    // 日志初始化（必须在最早）
    infra::logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let state = AppState::new(handle)?;
            app.manage(state);
            tracing::info!("MC Launcher 启动完成");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // app
            commands::app::app_version,
            commands::app::get_config,
            commands::app::set_config,
            // version
            commands::version::fetch_version_manifest,
            commands::version::list_local_versions,
            // install
            commands::install::install_version,
            // task
            commands::task::list_tasks,
            commands::task::cancel_task,
            // java
            commands::java::scan_java,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
