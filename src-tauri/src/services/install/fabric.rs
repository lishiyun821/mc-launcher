//! Fabric 加载器安装
//!
//! Fabric 的安装比 Forge 简单很多，全程只有两步 HTTP：
//!
//! 1. `GET /v2/versions/loader/<游戏版本>` —— 拿这个游戏版本支持的 loader 列表
//! 2. `GET /v2/versions/loader/<游戏版本>/<loader>/profile/json` —— 拿一份完整版本定义
//!
//! 第 2 步返回的 JSON 和原版的版本 JSON 结构一致，关键是有 `inheritsFrom`
//! 指向原版（比如 1.21.11），所以只要把它写进 `versions/<id>/<id>.raw.json`，
//! 后面完全可以复用原版那条安装链路：
//!
//! ```text
//! prepare() 写 raw.json
//!   → resolve() 沿 inheritsFrom 爬到原版并合并
//!   → plan() 计算要下哪些库（含 Fabric 的 Maven 形式库）
//!   → runner() 下载
//! ```
//!
//! 也就是说这里只负责「产出一个合法的版本定义」，真正下载交给 install 流程。

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::infra::http::Downloader;

/// Fabric 官方元数据服务
const META: &str = "https://meta.fabricmc.net";

/// 一个可选的 loader 版本
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FabricLoader {
    pub version: String,
    pub stable: bool,
}

// ── meta API 的原始结构（只取用得上的字段）──

#[derive(Deserialize)]
struct LoaderEntry {
    loader: LoaderInfo,
}

#[derive(Deserialize)]
struct LoaderInfo {
    version: String,
    #[serde(default)]
    stable: bool,
}

/// 列出某个游戏版本可用的 loader，按上游返回顺序（新的在前）
pub async fn list_loaders(dl: &Downloader, game: &str) -> Result<Vec<FabricLoader>> {
    let url = format!("{META}/v2/versions/loader/{game}");
    let bytes = dl
        .get_bytes(&url)
        .await
        .with_context(|| format!("拉取 Fabric loader 列表失败（游戏版本 {game}）"))?;

    let list: Vec<LoaderEntry> =
        serde_json::from_slice(&bytes).context("解析 Fabric loader 列表失败")?;

    Ok(list
        .into_iter()
        .map(|e| FabricLoader {
            version: e.loader.version,
            stable: e.loader.stable,
        })
        .collect())
}

/// 挑一个 loader：优先最新稳定版，没有稳定版就取最新的
pub async fn recommended(dl: &Downloader, game: &str) -> Result<FabricLoader> {
    let list = list_loaders(dl, game).await?;
    list.iter()
        .find(|l| l.stable)
        .or_else(|| list.first())
        .cloned()
        .ok_or_else(|| anyhow!("{game} 没有可用的 Fabric loader"))
}

/// 拉 profile，返回 (版本 id, 原始字节)
///
/// 版本 id 形如 `fabric-loader-0.20.1-1.21.11`，由 Fabric 自己决定，
/// 不要自己拼，直接用返回的。
pub async fn fetch_profile(dl: &Downloader, game: &str, loader: &str) -> Result<(String, Vec<u8>)> {
    let url = format!("{META}/v2/versions/loader/{game}/{loader}/profile/json");
    let bytes = dl
        .get_bytes(&url)
        .await
        .with_context(|| format!("拉取 Fabric profile 失败（{game} + loader {loader}）"))?;

    let v: serde_json::Value =
        serde_json::from_slice(&bytes).context("解析 Fabric profile 失败")?;

    let id = v
        .get("id")
        .and_then(|x| x.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Fabric profile 缺少 id 字段"))?
        .to_string();

    // 没有 inheritsFrom 就合并不了原版，装上也是废的
    if v.get("inheritsFrom").and_then(|x| x.as_str()).is_none() {
        return Err(anyhow!(
            "Fabric profile 没有 inheritsFrom（{id}），无法继承原版版本定义"
        ));
    }

    Ok((id, bytes))
}

/// 准备一个 Fabric 版本：选 loader → 拉 profile → 落盘
///
/// 只写版本定义，不下载任何库。之后调 `install_version(id)` 走通用安装。
/// 返回 Fabric 的版本 id。
pub async fn prepare(
    dl: &Downloader,
    versions_dir: &Path,
    game: &str,
    loader: Option<&str>,
) -> Result<String> {
    let loader_ver = match loader.map(str::trim).filter(|s| !s.is_empty()) {
        Some(l) => l.to_string(),
        None => recommended(dl, game).await?.version,
    };

    let (id, bytes) = fetch_profile(dl, game, &loader_ver).await?;

    let dir = versions_dir.join(&id);
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join(format!("{id}.raw.json")), &bytes)?;

    tracing::info!("Fabric {loader_ver} + {game} → 版本目录已写入 {id}");
    Ok(id)
}

#[cfg(test)]
mod tests {
    /// profile 的 id 与 inheritsFrom 是后续合并的前提，结构错了要能早发现
    #[test]
    fn profile_shape_check() {
        let sample = r#"{
            "id": "fabric-loader-0.20.1-1.21.11",
            "inheritsFrom": "1.21.11",
            "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient",
            "libraries": [
                {"name": "org.ow2.asm:asm:9.10.1", "url": "https://maven.fabricmc.net/",
                 "sha1": "ada2141c0cc52ee8f5c48cd5fa4ce0e794f22236", "size": 126151}
            ]
        }"#;
        let v: serde_json::Value = serde_json::from_str(sample).unwrap();
        assert_eq!(
            v.get("id").and_then(|x| x.as_str()),
            Some("fabric-loader-0.20.1-1.21.11")
        );
        assert_eq!(v.get("inheritsFrom").and_then(|x| x.as_str()), Some("1.21.11"));
        assert!(v.get("libraries").and_then(|x| x.as_array()).unwrap().len() == 1);
    }
}
