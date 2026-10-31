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
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
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
            // launch
            commands::launch::launch_version,
            // instance
            commands::instance::list_instances,
            commands::instance::get_instance,
            commands::instance::update_instance,
            commands::instance::delete_instance,
            commands::instance::duplicate_instance,
            commands::instance::open_instance_folder,
            commands::instance::open_game_folder,
            commands::instance::read_instance_log,
            commands::instance::verify_instance,
            // auth
            commands::auth::list_accounts,
            commands::auth::ms_login_start,
            commands::auth::ms_login_poll,
            commands::auth::remove_account,
            commands::auth::set_active_account,
            commands::auth::refresh_account,
            // task
            commands::task::list_tasks,
            commands::task::cancel_task,
            // java
            commands::update::check_update,
            commands::java::scan_java,
            commands::java::list_java_runtimes,
            commands::java::java_remote_info,
            commands::java::install_java,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
