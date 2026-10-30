use tauri::State;

use crate::services::instance::{self, InstanceConfig, InstanceInfo};
use crate::services::version;
use crate::state::AppState;

#[tauri::command]
pub async fn list_instances(_state: State<'_, AppState>) -> Result<Vec<InstanceInfo>, String> {
    let mc_dir = version::default_mc_dir();
    Ok(instance::list(&mc_dir))
}

#[tauri::command]
pub async fn get_instance(id: String) -> Result<InstanceConfig, String> {
    let mc_dir = version::default_mc_dir();
    Ok(instance::load(&mc_dir, &id))
}

#[tauri::command]
pub async fn update_instance(id: String, config: InstanceConfig) -> Result<(), String> {
    let mc_dir = version::default_mc_dir();
    instance::save(&mc_dir, &id, &config).map_err(|e| e.to_string())?;
    tracing::info!("实例设置已保存: {id}");
    Ok(())
}

/// 删除实例，返回释放的字节数
#[tauri::command]
pub async fn delete_instance(id: String) -> Result<u64, String> {
    let mc_dir = version::default_mc_dir();
    let freed = instance::remove(&mc_dir, &id).map_err(|e| e.to_string())?;
    tracing::info!("实例已删除: {id}，释放 {:.1} MB", freed as f64 / 1024.0 / 1024.0);
    Ok(freed)
}

/// 复制实例（另存为副本）
#[tauri::command]
pub async fn duplicate_instance(from: String, to: String) -> Result<(), String> {
    if to.trim().is_empty() {
        return Err("新名字不能为空".to_string());
    }
    let mc_dir = version::default_mc_dir();
    instance::duplicate(&mc_dir, &from, &to).map_err(|e| e.to_string())?;
    tracing::info!("实例已复制: {from} → {to}");
    Ok(())
}

/// 在资源管理器里打开实例目录
#[tauri::command]
pub async fn open_instance_folder(id: String) -> Result<String, String> {
    let mc_dir = version::default_mc_dir();
    let dir = mc_dir.join("versions").join(&id);
    if !dir.is_dir() {
        return Err("目录不存在".to_string());
    }

    #[cfg(windows)]
    {
        std::process::Command::new("explorer")
            .arg(&dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(&dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(dir.display().to_string())
}

/// 打开游戏根目录
#[tauri::command]
pub async fn open_game_folder() -> Result<String, String> {
    let mc_dir = version::default_mc_dir();
    #[cfg(windows)]
    {
        std::process::Command::new("explorer")
            .arg(&mc_dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(mc_dir.display().to_string())
}
