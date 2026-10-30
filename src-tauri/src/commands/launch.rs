use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::State;

use crate::core::models::VersionJson;
use crate::services::install::version_json;
use crate::services::instance;
use crate::services::launch::{account, args, classpath, process, LaunchOptions};
use crate::services::{java as java_service, version};
use crate::state::AppState;

/// 启动结果
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchReport {
    pub pid: u32,
    pub version: String,
    pub main_class: String,
    pub java: String,
    pub java_major: u32,
    pub account: String,
    pub uuid: String,
    pub classpath_count: usize,
    pub memory_mb: u32,
    pub command_preview: String,
    pub game_dir: String,
    /// 缺失的 classpath 文件（非空说明没装全）
    pub missing: Vec<String>,
}

#[tauri::command]
pub async fn launch_version(
    state: State<'_, AppState>,
    id: String,
    account_name: String,
    memory_mb: Option<u32>,
) -> Result<LaunchReport, String> {
    do_launch(&state, &id, &account_name, memory_mb.unwrap_or(4096))
        .await
        .map_err(|e| e.to_string())
}

async fn do_launch(
    _state: &AppState,
    id: &str,
    name: &str,
    memory_mb: u32,
) -> Result<LaunchReport, anyhow::Error> {
    account::validate_name(name).map_err(|e| anyhow::anyhow!(e))?;

    let mc_dir = version::default_mc_dir();
    let json_path = mc_dir
        .join("versions")
        .join(id)
        .join(format!("{id}.json"));

    if !json_path.is_file() {
        return Err(anyhow::anyhow!("{id} 还没装好，先点安装"));
    }

    let text = std::fs::read_to_string(&json_path)?;
    let vj: VersionJson =
        serde_json::from_str(&text).map_err(|e| anyhow::anyhow!("版本定义解析失败: {e}"))?;
    let rv = version_json::from_merged(&vj)?;

    /* ① 完整性自检 */
    let missing = classpath::missing_files(&mc_dir, &rv);
    if !missing.is_empty() {
        return Err(anyhow::anyhow!(
            "还有 {} 个库文件没下载（例: {}），请先重新安装",
            missing.len(),
            missing.first().cloned().unwrap_or_default()
        ));
    }

    /* ② 读实例设置：实例里的配置优先于全局默认 */
    let inst = instance::load(&mc_dir, id);
    let memory_mb = inst.memory_mb.unwrap_or(memory_mb);
    let width = inst.width.unwrap_or(854);
    let height = inst.height.unwrap_or(480);
    let work_dir = inst
        .game_dir
        .as_ref()
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| mc_dir.clone());

    /* ③ 选 Java：实例指定优先 → 否则挑满足要求的最低版本 */
    let required = rv.java_major.unwrap_or(8);
    let javas = java_service::scan().await;
    if javas.is_empty() {
        return Err(anyhow::anyhow!(
            "系统里没找到 Java，这个版本需要 Java {required}"
        ));
    }

    let java = inst
        .java_path
        .as_ref()
        .filter(|p| Path::new(p).is_file())
        .and_then(|p| javas.iter().find(|j| &j.path == p).cloned())
        .or_else(|| {
            javas
                .iter()
                .filter(|j| j.major >= required)
                .min_by_key(|j| j.major)
                .cloned()
        })
        .or_else(|| javas.iter().max_by_key(|j| j.major).cloned())
        .ok_or_else(|| anyhow::anyhow!("没有可用的 Java"))?;

    /* ④ classpath */
    let cp_paths = classpath::build(&mc_dir, &rv);
    let cp_count = cp_paths.len();
    let cp = classpath::join(&cp_paths);

    /* ⑤ natives 目录 */
    let natives_dir = mc_dir.join("versions").join(id).join("natives");

    /* ⑥ 组装参数：优先用当前登录的正版账户，没有就离线 */
    let acct = match crate::services::auth::store::active() {
        Some(a) if a.is_msa() => {
            if a.is_expired() {
                return Err(anyhow::anyhow!(
                    "账户 {} 的登录已过期，去「账户」页点一下续期",
                    a.name
                ));
            }
            tracing::info!("使用正版账户启动: {}", a.name);
            account::Account::from_stored(&a)
        }
        _ => {
            tracing::info!("没有登录账户，使用离线模式: {name}");
            account::Account::offline(name)
        }
    };
    let opt = LaunchOptions {
        java: PathBuf::from(&java.path),
        mc_dir: mc_dir.clone(),
        natives_dir,
        classpath: cp,
        account: acct.clone(),
        memory_mb,
        width,
        height,
        extra_jvm: inst.extra_jvm.clone(),
    };

    let preview = args::preview_command(&rv, &opt);
    let jvm_args = args::build_jvm_args(&rv, &opt);
    let game_args = args::build_game_args(&rv, &opt);

    /* ⑦ 启动 */
    let log_file = mc_dir.join("logs").join(format!("game-{id}.log"));
    let gp = process::spawn(
        &opt.java,
        &jvm_args,
        &rv.main_class,
        &game_args,
        &work_dir,
        Some(log_file),
    )
    .await?;

    // 记一下这次启动时间
    let _ = instance::touch(&mc_dir, id);

    let pid = gp.pid;
    tracing::info!(
        "游戏已启动 pid={pid} | {} | Java {} | 内存 {} MB | classpath {cp_count} 项",
        rv.id,
        java.version,
        memory_mb
    );

    // 后台等它退出，记一下退出码
    let version_id = rv.id.clone();
    tauri::async_runtime::spawn(async move {
        let code = process::wait(gp.child).await;
        tracing::info!("游戏进程结束: {version_id} pid={pid} 退出码={code:?}");
    });

    Ok(LaunchReport {
        pid,
        version: rv.id.clone(),
        main_class: rv.main_class.clone(),
        java: java.path.clone(),
        java_major: java.major,
        account: acct.name.clone(),
        uuid: acct.uuid_dashed(),
        classpath_count: cp_count,
        memory_mb,
        command_preview: preview,
        game_dir: mc_dir.display().to_string(),
        missing: Vec::new(),
    })
}
