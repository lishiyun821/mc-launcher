use tauri::State;

use crate::core::models::AppConfig;
use crate::state::AppState;

#[tauri::command]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    let cfg = state.config.read().await.clone();
    Ok(cfg)
}

#[tauri::command]
pub async fn set_config(state: State<'_, AppState>, config: AppConfig) -> Result<(), String> {
    config.save().map_err(|e| e.to_string())?;
    let mut guard = state.config.write().await;
    *guard = config;
    tracing::info!("配置已保存");
    Ok(())
}
