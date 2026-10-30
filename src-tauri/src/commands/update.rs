//! 启动器自身更新检查
//!
//! 只查 GitHub Releases，不做自动下载安装（那需要 Tauri updater 的签名密钥
//! 和 CI 产物，等有构建流水线再接）。这里负责：有新版本就告诉用户，给个链接。

use serde::Serialize;

/// 仓库坐标
const REPO: &str = "lishiyun821/mc-launcher";

/// 当前版本（跟 tauri.conf.json / Cargo.toml 对齐）
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current: String,
    /// 最新版本号（没有 release 时为 None）
    pub latest: Option<String>,
    pub has_update: bool,
    /// 打开发布页
    pub url: String,
    pub notes: Option<String>,
    pub published_at: Option<String>,
    /// 检查失败的原因（网络 / 没发布过等），成功时为 None
    pub error: Option<String>,
}

impl UpdateInfo {
    fn base() -> Self {
        Self {
            current: CURRENT.to_string(),
            latest: None,
            has_update: false,
            url: format!("https://github.com/{REPO}/releases"),
            notes: None,
            published_at: None,
            error: None,
        }
    }
}

/// 把 `v1.2.3` / `1.2.3-beta` 拆成可比较的数字段
fn parse_ver(s: &str) -> Vec<u32> {
    s.trim()
        .trim_start_matches(['v', 'V'])
        .split(|c: char| !c.is_ascii_digit())
        .filter(|x| !x.is_empty())
        .filter_map(|x| x.parse::<u32>().ok())
        .collect()
}

/// latest 是否比 current 新
fn is_newer(latest: &str, current: &str) -> bool {
    let a = parse_ver(latest);
    let b = parse_ver(current);
    if a.is_empty() {
        return false;
    }
    for i in 0..a.len().max(b.len()) {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        if x != y {
            return x > y;
        }
    }
    false
}

pub async fn check(http: &reqwest::Client) -> UpdateInfo {
    let mut info = UpdateInfo::base();

    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let res = match http
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            info.error = Some(format!("连不上 GitHub：{e}"));
            return info;
        }
    };

    if res.status() == reqwest::StatusCode::NOT_FOUND {
        info.error = Some("还没有发布过版本".to_string());
        return info;
    }

    if !res.status().is_success() {
        info.error = Some(format!("GitHub 返回 HTTP {}", res.status().as_u16()));
        return info;
    }

    let body: serde_json::Value = match res.json().await {
        Ok(v) => v,
        Err(e) => {
            info.error = Some(format!("响应解析失败：{e}"));
            return info;
        }
    };

    let tag = body
        .get("tag_name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if tag.is_empty() {
        info.error = Some("发布信息里没有版本号".to_string());
        return info;
    }

    let ver = tag.trim_start_matches(['v', 'V']).to_string();

    info.has_update = is_newer(&ver, CURRENT);
    info.latest = Some(ver);
    info.url = body
        .get("html_url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("https://github.com/{REPO}/releases"));
    info.notes = body
        .get("body")
        .and_then(|v| v.as_str())
        .map(|s| s.chars().take(2000).collect());
    info.published_at = body
        .get("published_at")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    info
}

/// IPC 入口
#[tauri::command]
pub async fn check_update(
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<UpdateInfo, String> {
    Ok(check(state.downloader.http()).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_versions() {
        assert_eq!(parse_ver("v1.2.3"), vec![1, 2, 3]);
        assert_eq!(parse_ver("1.0.0-beta.2"), vec![1, 0, 0, 2]);
        assert_eq!(parse_ver("26.3"), vec![26, 3]);
    }

    #[test]
    fn newer_detection() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
        assert!(is_newer("0.1.1", "0.1"));
    }

    #[test]
    fn junk_versions_are_safe() {
        assert!(!is_newer("", "0.1.0"));
        assert!(!is_newer("abc", "0.1.0"));
    }
}
