use tauri::State;

use crate::core::models::VersionManifest;
use crate::services;
use crate::state::AppState;

/// 拉取版本清单（forceRefresh=true 时绕过缓存）
#[tauri::command]
pub async fn fetch_version_manifest(
    state: State<'_, AppState>,
    force_refresh: bool,
) -> Result<VersionManifest, String> {
    services::version::fetch_manifest(&state.downloader, force_refresh)
        .await
        .map_err(|e| e.to_string())
}

/// 列出本地已安装的版本
#[tauri::command]
pub fn list_local_versions() -> Vec<String> {
    services::version::list_local_versions()
}
