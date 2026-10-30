use serde::Serialize;
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

/* ─────────────── 日志与校验 ─────────────── */

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogView {
    pub path: String,
    pub exists: bool,
    pub size: u64,
    /// 最近 N 行
    pub lines: Vec<String>,
    /// 是不是只给了尾部
    pub truncated: bool,
}

/// 读实例的最新游戏日志（从文件尾部读，不怕日志很大）
#[tauri::command]
pub async fn read_instance_log(
    id: String,
    lines: Option<usize>,
) -> Result<LogView, String> {
    use std::io::{Read, Seek, SeekFrom};

    let mc_dir = version::default_mc_dir();
    let path = mc_dir.join("logs").join(format!("game-{id}.log"));
    let want = lines.unwrap_or(400).clamp(20, 5000);

    if !path.is_file() {
        return Ok(LogView {
            path: path.display().to_string(),
            exists: false,
            size: 0,
            lines: Vec::new(),
            truncated: false,
        });
    }

    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

    // 最多读尾部 2MB —— 够 400 行，也不会因为几 GB 的日志卡死
    const MAX_TAIL: u64 = 2 * 1024 * 1024;
    let mut file = std::fs::File::open(&path).map_err(|e| e.to_string())?;
    let start = size.saturating_sub(MAX_TAIL);
    file.seek(SeekFrom::Start(start)).map_err(|e| e.to_string())?;

    let mut buf = String::new();
    file.read_to_string(&mut buf).map_err(|e| e.to_string())?;

    // 如果从中间切进去，第一行可能是半截，丢掉
    if start > 0 {
        if let Some(pos) = buf.find('\n') {
            buf = buf[pos + 1..].to_string();
        }
    }

    let all: Vec<&str> = buf.lines().collect();
    let truncated = all.len() > want || start > 0;
    let tail: Vec<String> = all
        .iter()
        .rev()
        .take(want)
        .rev()
        .map(|s| s.to_string())
        .collect();

    Ok(LogView {
        path: path.display().to_string(),
        exists: true,
        size,
        lines: tail,
        truncated,
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyReport {
    /// classpath 应有条目数
    pub total: usize,
    /// 缺哪些（相对路径）
    pub missing: Vec<String>,
    pub ok: bool,
    /// 客户端 jar 在不在
    pub has_jar: bool,
    /// native 目录文件数
    pub native_files: usize,
    pub checked_ms: u64,
}

/// 校验实例文件完整性（只查不缺，不下载）
#[tauri::command]
pub async fn verify_instance(id: String) -> Result<VerifyReport, String> {
    let started = std::time::Instant::now();
    let mc_dir = version::default_mc_dir();

    let json_path = mc_dir.join("versions").join(&id).join(format!("{id}.json"));
    if !json_path.is_file() {
        return Err(format!("{id} 的版本定义不存在，先安装"));
    }

    let text = std::fs::read_to_string(&json_path).map_err(|e| e.to_string())?;
    let vj: crate::core::models::VersionJson =
        serde_json::from_str(&text).map_err(|e| format!("版本定义解析失败: {e}"))?;
    let rv = crate::services::install::version_json::from_merged(&vj)
        .map_err(|e| e.to_string())?;

    let total = crate::services::launch::classpath::build(&mc_dir, &rv).len();
    let missing = crate::services::launch::classpath::missing_files(&mc_dir, &rv);

    let natives_dir = mc_dir.join("versions").join(&id).join("natives");
    let native_files = std::fs::read_dir(&natives_dir)
        .map(|d| d.flatten().filter(|e| e.path().is_file()).count())
        .unwrap_or(0);

    Ok(VerifyReport {
        total,
        ok: missing.is_empty(),
        missing,
        has_jar: mc_dir
            .join("versions")
            .join(&id)
            .join(format!("{id}.jar"))
            .is_file(),
        native_files,
        checked_ms: started.elapsed().as_millis() as u64,
    })
}
