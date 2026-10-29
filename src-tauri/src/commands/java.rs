use crate::core::models::JavaInfo;
use crate::services;

#[tauri::command]
pub async fn scan_java() -> Result<Vec<JavaInfo>, String> {
    Ok(services::java::scan().await)
}
