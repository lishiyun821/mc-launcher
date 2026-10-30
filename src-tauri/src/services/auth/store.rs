//! 账户存储（`<data>/accounts.json`）
//!
//! ⚠️ 令牌以明文存在用户自己的数据目录里 —— 和 PCL / HMCL 的做法一致。
//! 这个文件**绝不能**进日志、进 git、进任何上报。开源项目里它是本地隐私数据，
//! 位置在 `%APPDATA%`（或本启动器的数据目录），系统默认只有当前用户能读。
//! 以后可以加 Windows DPAPI 加密，接口不用改。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// 一个已保存的账户
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredAccount {
    /// 无横线 UUID，作为唯一键
    pub id: String,
    /// 游戏内名字
    pub name: String,
    /// 带横线 UUID
    pub uuid: String,
    /// Minecraft access token（启动时用）
    pub access_token: String,
    /// Minecraft 令牌过期时间（unix 秒）
    pub expires_at: i64,
    /// 微软 refresh token（续期用）
    #[serde(default)]
    pub refresh_token: String,
    /// XSTS 里的 xuid（正版服务器要用）
    #[serde(default)]
    pub xuid: Option<String>,
    /// 皮肤地址
    #[serde(default)]
    pub skin_url: Option<String>,
    /// 披风地址
    #[serde(default)]
    pub cape_url: Option<String>,
    /// msa = 正版，offline = 离线
    #[serde(default = "default_kind")]
    pub kind: String,
    /// 保存时间
    #[serde(default)]
    pub added_at: i64,
}

fn default_kind() -> String {
    "msa".to_string()
}

impl StoredAccount {
    /// 令牌是否过期（留 5 分钟余量）
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        self.expires_at - 300 <= now
    }

    pub fn is_msa(&self) -> bool {
        self.kind != "offline"
    }
}

/// 整个文件
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AccountsFile {
    pub accounts: Vec<StoredAccount>,
    /// 当前选中的账户 id
    pub active: Option<String>,
}

fn path() -> PathBuf {
    crate::infra::paths::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("accounts.json")
}

/// 读（文件不存在 / 坏了 → 空）
pub fn load() -> AccountsFile {
    load_from(&path())
}

pub fn load_from(p: &Path) -> AccountsFile {
    std::fs::read_to_string(p)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// 原子写
pub fn save(file: &AccountsFile) -> Result<(), anyhow::Error> {
    save_to(&path(), file)
}

pub fn save_to(p: &Path, file: &AccountsFile) -> Result<(), anyhow::Error> {
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(file)?;
    let tmp = p.with_extension("json.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, p)?;
    Ok(())
}

/// 加或更新一个账户（同 UUID 覆盖），并设为当前账户
pub fn upsert(account: StoredAccount) -> Result<AccountsFile, anyhow::Error> {
    let mut file = load();

    if let Some(existing) = file.accounts.iter_mut().find(|a| a.id == account.id) {
        *existing = account.clone();
    } else {
        file.accounts.push(account.clone());
    }

    file.active = Some(account.id);
    save(&file)?;
    Ok(file)
}

/// 删除账户
pub fn remove(id: &str) -> Result<AccountsFile, anyhow::Error> {
    let mut file = load();
    file.accounts.retain(|a| a.id != id);

    if file.active.as_deref() == Some(id) {
        file.active = file.accounts.first().map(|a| a.id.clone());
    }

    save(&file)?;
    Ok(file)
}

/// 切换当前账户
pub fn set_active(id: &str) -> Result<AccountsFile, anyhow::Error> {
    let mut file = load();
    if file.accounts.iter().any(|a| a.id == id) {
        file.active = Some(id.to_string());
        save(&file)?;
    }
    Ok(file)
}

/// 取当前账户
pub fn active() -> Option<StoredAccount> {
    let file = load();
    let id = file.active.clone()?;
    file.accounts.into_iter().find(|a| a.id == id)
}

/// 更新某个账户的令牌（续期后写回）
pub fn update_token(
    id: &str,
    access_token: String,
    expires_in: u64,
    refresh_token: Option<String>,
) -> Result<(), anyhow::Error> {
    let mut file = load();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    if let Some(a) = file.accounts.iter_mut().find(|a| a.id == id) {
        a.access_token = access_token;
        a.expires_at = now + expires_in as i64;
        if let Some(r) = refresh_token {
            a.refresh_token = r;
        }
        save(&file)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("mcl_acc_{name}"));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d.join("accounts.json")
    }

    fn acc(id: &str, name: &str) -> StoredAccount {
        StoredAccount {
            id: id.to_string(),
            name: name.to_string(),
            uuid: format!("{id}-dashed"),
            access_token: "tok".to_string(),
            expires_at: 9999999999,
            refresh_token: "rt".to_string(),
            xuid: Some("123".to_string()),
            skin_url: None,
            cape_url: None,
            kind: "msa".to_string(),
            added_at: 0,
        }
    }

    #[test]
    fn save_and_load_roundtrip() {
        let p = tmp("a");
        let mut f = AccountsFile::default();
        f.accounts.push(acc("aaa", "Steve"));
        f.active = Some("aaa".to_string());
        save_to(&p, &f).unwrap();

        let back = load_from(&p);
        assert_eq!(back.accounts.len(), 1);
        assert_eq!(back.accounts[0].name, "Steve");
        assert_eq!(back.active.as_deref(), Some("aaa"));
        assert!(back.accounts[0].is_msa());
        assert!(!back.accounts[0].is_expired());
    }

    #[test]
    fn expired_detection() {
        let mut a = acc("bbb", "Alex");
        a.expires_at = 1000; // 早就过期
        assert!(a.is_expired());
    }

    #[test]
    fn missing_file_is_empty() {
        let p = std::env::temp_dir().join("mcl_acc_nonexistent_xyz.json");
        let _ = std::fs::remove_file(&p);
        let f = load_from(&p);
        assert!(f.accounts.is_empty());
        assert!(f.active.is_none());
    }
}
