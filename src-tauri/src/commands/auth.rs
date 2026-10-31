use serde::Serialize;
use tauri::State;

use crate::services::auth::{microsoft, store};
use crate::state::AppState;

/// 给前端的账户信息 —— **不含任何令牌**
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInfo {
    pub id: String,
    pub name: String,
    pub uuid: String,
    /// msa / offline
    pub kind: String,
    pub skin_url: Option<String>,
    pub cape_url: Option<String>,
    pub expires_at: i64,
    pub expired: bool,
    pub active: bool,
}

fn to_info(a: &store::StoredAccount, active_id: Option<&str>) -> AccountInfo {
    AccountInfo {
        id: a.id.clone(),
        name: a.name.clone(),
        uuid: a.uuid.clone(),
        kind: a.kind.clone(),
        skin_url: a.skin_url.clone(),
        cape_url: a.cape_url.clone(),
        expires_at: a.expires_at,
        expired: a.is_expired(),
        active: active_id == Some(a.id.as_str()),
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountsView {
    pub accounts: Vec<AccountInfo>,
    pub active: Option<String>,
}

/// 列账户（不含令牌）
#[tauri::command]
pub async fn list_accounts() -> Result<AccountsView, String> {
    let file = store::load();
    Ok(AccountsView {
        accounts: file
            .accounts
            .iter()
            .map(|a| to_info(a, file.active.as_deref()))
            .collect(),
        active: file.active.clone(),
    })
}

/// 添加一个离线账户（不起正版验证，单人游戏够用）
#[tauri::command]
pub async fn add_offline_account(name: String) -> Result<AccountInfo, String> {
    let name = name.trim().to_string();

    if name.len() < 3 || name.len() > 16 {
        return Err("用户名需要 3 到 16 个字符".to_string());
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err("用户名只能包含字母、数字和下划线".to_string());
    }

    // 离线 UUID 由名字哈希得来，和原版行为一致
    let raw = crate::services::launch::account::offline_uuid(&name);
    if raw.len() != 32 {
        return Err("生成 UUID 失败".to_string());
    }
    let uuid = format!(
        "{}-{}-{}-{}-{}",
        &raw[0..8],
        &raw[8..12],
        &raw[12..16],
        &raw[16..20],
        &raw[20..32]
    );

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let stored = store::StoredAccount {
        id: raw.clone(),
        name,
        uuid,
        access_token: String::new(),
        expires_at: 0,
        refresh_token: String::new(),
        xuid: None,
        skin_url: None,
        cape_url: None,
        kind: "offline".to_string(),
        added_at: now,
    };

    let file = store::upsert(stored).map_err(|e| e.to_string())?;
    let acc = file
        .accounts
        .iter()
        .find(|a| a.id == raw)
        .ok_or_else(|| "保存后没找到账户".to_string())?;

    Ok(to_info(acc, file.active.as_deref()))
}

/* ─────────────────────────── 登录 ─────────────────────────── */

/// 第一步：申请登录码（前端拿 userCode 让用户去浏览器输）
#[tauri::command]
pub async fn ms_login_start(state: State<'_, AppState>) -> Result<microsoft::DeviceCode, String> {
    microsoft::request_device_code(&state.downloader.http())
        .await
        .map_err(|e| e.to_string())
}

/// 第二步：轮询结果
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum LoginState {
    /// 还没完成
    Pending,
    /// 完成，账户已保存
    Success { account: AccountInfo },
    /// 失败（附人话原因）
    Failed { message: String },
}

#[tauri::command]
pub async fn ms_login_poll(
    state: State<'_, AppState>,
    device_code: String,
) -> Result<LoginState, String> {
    let polled = microsoft::poll_token(&state.downloader.http(), &device_code)
        .await
        .map_err(|e| e.to_string())?;

    let ms = match polled {
        microsoft::PollResult::Pending => return Ok(LoginState::Pending),
        microsoft::PollResult::SlowDown => return Ok(LoginState::Pending),
        microsoft::PollResult::Failed(msg) => {
            return Ok(LoginState::Failed { message: msg })
        }
        microsoft::PollResult::Success(ms) => ms,
    };

    // 从 MS 令牌一路走到玩家档案
    let outcome = match microsoft::login_with_ms_token(&state.downloader.http(), ms).await {
        Ok(o) => o,
        Err(e) => {
            return Ok(LoginState::Failed {
                message: e.to_string(),
            })
        }
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let skin = outcome
        .profile
        .skins
        .iter()
        .find(|s| s.state.eq_ignore_ascii_case("ACTIVE"))
        .or_else(|| outcome.profile.skins.first())
        .map(|s| s.url.clone());

    let cape = outcome
        .profile
        .capes
        .iter()
        .find(|c| c.state.eq_ignore_ascii_case("ACTIVE"))
        .map(|c| c.url.clone());

    let account = store::StoredAccount {
        id: outcome.profile.id.clone(),
        name: outcome.profile.name.clone(),
        uuid: dashed(&outcome.profile.id),
        access_token: outcome.minecraft_token.access_token.clone(),
        expires_at: now + outcome.minecraft_token.expires_in as i64,
        refresh_token: outcome.ms.refresh_token.clone(),
        xuid: if outcome.xuid.is_empty() {
            None
        } else {
            Some(outcome.xuid.clone())
        },
        skin_url: skin,
        cape_url: cape,
        kind: "msa".to_string(),
        added_at: now,
    };

    let saved = store::upsert(account).map_err(|e| e.to_string())?;

    tracing::info!(
        "微软登录成功: {} ({}) 共 {} 个账户",
        outcome.profile.name,
        outcome.profile.id,
        saved.accounts.len()
    );

    let active_id = saved.active.clone();
    let info = saved
        .accounts
        .iter()
        .find(|a| Some(a.id.as_str()) == active_id.as_deref())
        .map(|a| to_info(a, active_id.as_deref()))
        .ok_or_else(|| "账户保存后读取失败".to_string())?;

    Ok(LoginState::Success { account: info })
}

/* ─────────────────────────── 管理 ─────────────────────────── */

#[tauri::command]
pub async fn remove_account(id: String) -> Result<AccountsView, String> {
    let file = store::remove(&id).map_err(|e| e.to_string())?;
    Ok(AccountsView {
        accounts: file
            .accounts
            .iter()
            .map(|a| to_info(a, file.active.as_deref()))
            .collect(),
        active: file.active,
    })
}

#[tauri::command]
pub async fn set_active_account(id: String) -> Result<AccountsView, String> {
    let file = store::set_active(&id).map_err(|e| e.to_string())?;
    Ok(AccountsView {
        accounts: file
            .accounts
            .iter()
            .map(|a| to_info(a, file.active.as_deref()))
            .collect(),
        active: file.active,
    })
}

/// 用 refresh_token 续期 Minecraft 令牌
#[tauri::command]
pub async fn refresh_account(state: State<'_, AppState>, id: String) -> Result<AccountInfo, String> {
    let file = store::load();
    let acc = file
        .accounts
        .iter()
        .find(|a| a.id == id)
        .cloned()
        .ok_or_else(|| "账户不存在".to_string())?;

    if !acc.is_msa() || acc.refresh_token.is_empty() {
        return Err("这不是微软账户，不需要续期".to_string());
    }

    let ms = microsoft::refresh_ms_token(&state.downloader.http(), &acc.refresh_token)
        .await
        .map_err(|e| e.to_string())?;

    let outcome = microsoft::login_with_ms_token(&state.downloader.http(), ms)
        .await
        .map_err(|e| e.to_string())?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    store::update_token(
        &id,
        outcome.minecraft_token.access_token.clone(),
        outcome.minecraft_token.expires_in,
        Some(outcome.ms.refresh_token.clone()),
    )
    .map_err(|e| e.to_string())?;

    let file = store::load();
    let updated = file
        .accounts
        .iter()
        .find(|a| a.id == id)
        .map(|a| to_info(a, file.active.as_deref()))
        .ok_or_else(|| "续期后读取失败".to_string())?;

    tracing::info!("账户令牌已续期: {} (至 {})", updated.name, now + outcome.minecraft_token.expires_in as i64);
    Ok(updated)
}

fn dashed(raw: &str) -> String {
    let s: String = raw.chars().filter(|c| *c != '-').collect();
    if s.len() != 32 {
        return raw.to_string();
    }
    format!(
        "{}-{}-{}-{}-{}",
        &s[0..8],
        &s[8..12],
        &s[12..16],
        &s[16..20],
        &s[20..32]
    )
}
