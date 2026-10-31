use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/* ═══════════════ 版本清单 ═══════════════ */

/// version_manifest_v2.json 里的一个版本条目
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawVersionEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
    pub time: String,
    pub release_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawLatest {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawManifest {
    pub latest: RawLatest,
    pub versions: Vec<RawVersionEntry>,
}

/// 给前端用的版本摘要
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionSummary {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
    pub release_time: String,
    pub is_latest: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LatestVersions {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VersionManifest {
    pub latest: LatestVersions,
    pub versions: Vec<VersionSummary>,
}

impl From<RawManifest> for VersionManifest {
    fn from(raw: RawManifest) -> Self {
        let latest_release = raw.latest.release.clone();
        Self {
            latest: LatestVersions {
                release: raw.latest.release,
                snapshot: raw.latest.snapshot,
            },
            versions: raw
                .versions
                .into_iter()
                .map(|v| VersionSummary {
                    is_latest: v.id == latest_release,
                    id: v.id,
                    kind: v.kind,
                    url: v.url,
                    release_time: v.release_time,
                })
                .collect(),
        }
    }
}

/* ═══════════════ 完整版本 JSON（启动用，Phase 1） ═══════════════ */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionJson {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    #[serde(rename = "mainClass")]
    pub main_class: Option<String>,
    #[serde(rename = "inheritsFrom")]
    pub inherits_from: Option<String>,
    /// 合并时记下的完整继承链（父 → 子）。
    ///
    /// 合并后 inheritsFrom 会被抹掉，但启动、完整性校验还需要知道父版本是谁
    /// （例如 Fabric 版本的客户端 jar 其实就是原版那一份）。
    #[serde(rename = "mergedChain", default, skip_serializing_if = "Option::is_none")]
    pub merged_chain: Option<Vec<String>>,
    #[serde(rename = "minecraftArguments")]
    pub minecraft_arguments: Option<String>,
    pub arguments: Option<Arguments>,
    #[serde(default)]
    pub libraries: Vec<Library>,
    #[serde(rename = "assetIndex")]
    pub asset_index: Option<AssetIndex>,
    pub assets: Option<String>,
    pub downloads: Option<Downloads>,
    #[serde(rename = "javaVersion")]
    pub java_version: Option<JavaVersion>,
    #[serde(rename = "releaseTime")]
    pub release_time: Option<String>,
    pub time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<serde_json::Value>,
    #[serde(default)]
    pub jvm: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub name: String,
    pub downloads: Option<LibraryDownloads>,
    #[serde(default)]
    pub rules: Vec<Rule>,
    pub natives: Option<std::collections::HashMap<String, String>>,
    pub extract: Option<Extract>,

    // ── Maven 形式（Fabric / Quilt 的库这么给）──
    // 没有 downloads.artifact，只给仓库根地址 + 校验值，路径从坐标推导。
    /// Maven 仓库根，如 "https://maven.fabricmc.net/"
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryDownloads {
    pub artifact: Option<Artifact>,
    pub classifiers: Option<std::collections::HashMap<String, Artifact>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub path: Option<String>,
    pub sha1: Option<String>,
    pub size: Option<u64>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extract {
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub action: String,
    pub os: Option<OsRule>,
    pub features: Option<std::collections::HashMap<String, bool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsRule {
    pub name: Option<String>,
    pub arch: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetIndex {
    pub id: String,
    pub sha1: Option<String>,
    pub size: Option<u64>,
    #[serde(rename = "totalSize")]
    pub total_size: Option<u64>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Downloads {
    pub client: Option<Artifact>,
    pub server: Option<Artifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaVersion {
    pub component: Option<String>,
    #[serde(rename = "majorVersion")]
    pub major_version: Option<u32>,
}

/* ═══════════════ 配置 ═══════════════ */

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DownloadSource {
    Official,
    Mirror,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub download_source: DownloadSource,
    pub language: String,
    /// 离线账户名字（启动时用）
    #[serde(default = "default_player_name")]
    pub player_name: String,
    /// 首页选中的实例 id
    #[serde(default)]
    pub active_instance: Option<String>,
    /// 主题：system / light / dark
    #[serde(default = "default_theme")]
    pub theme: String,
    /// 下载限速 KB/s（0 = 不限）
    #[serde(default)]
    pub download_limit_kbps: u32,
}

fn default_player_name() -> String {
    "MyisaCN".to_string()
}

fn default_theme() -> String {
    "system".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            download_source: DownloadSource::Official,
            language: "zh-CN".to_string(),
            player_name: default_player_name(),
            active_instance: None,
            theme: default_theme(),
            download_limit_kbps: 0,
        }
    }
}

impl AppConfig {
    fn path() -> Option<PathBuf> {
        crate::infra::paths::config_dir().map(|d| d.join("config.json"))
    }

    pub fn load() -> Result<Self, anyhow::Error> {
        let p = Self::path().ok_or_else(|| anyhow::anyhow!("no config dir"))?;
        if !p.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(p)?;
        Ok(serde_json::from_str(&text)?)
    }

    pub fn save(&self) -> Result<(), anyhow::Error> {
        let p = Self::path().ok_or_else(|| anyhow::anyhow!("no config dir"))?;
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(p, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}

/* ═══════════════ Java ═══════════════ */

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaInfo {
    pub path: String,
    pub version: String,
    pub major: u32,
    pub vendor: String,
    pub is64_bit: bool,
}
