use tauri::State;

use crate::infra::task::TaskInfo;
use crate::state::AppState;

#[tauri::command]
pub async fn list_tasks(state: State<'_, AppState>) -> Result<Vec<TaskInfo>, String> {
    Ok(state.tasks.list().await)
}

#[tauri::command]
pub async fn cancel_task(state: State<'_, AppState>, task_id: String) -> Result<(), String> {
    state.tasks.cancel(&task_id).await;
    Ok(())
}
