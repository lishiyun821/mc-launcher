//! Java 运行时自动下载（Adoptium / Eclipse Temurin JRE）
//!
//! 新手最容易被「没有 Java」卡住，这里直接按版本需求拉一个官方 JRE 下来，
//! 解压到 `<data>/runtime/java-<major>`，不动系统环境变量。

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Adoptium API：某个大版本的最新 JRE
const API: &str = "https://api.adoptium.net/v3/assets/latest";

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JreInfo {
    pub major: u32,
    /// 例如 21.0.5+11
    pub version: String,
    pub url: String,
    pub size: u64,
    pub name: String,
}

#[derive(Deserialize)]
struct Raw {
    binary: Option<Binary>,
    release_name: Option<String>,
}

#[derive(Deserialize)]
struct Binary {
    package: Option<Package>,
    version: Option<VersionInfo>,
}

#[derive(Deserialize)]
struct Package {
    link: Option<String>,
    name: Option<String>,
    size: Option<u64>,
}

#[derive(Deserialize)]
struct VersionInfo {
    semver: Option<String>,
}

/// 平台标识
fn os_name() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "mac"
    } else {
        "linux"
    }
}

fn arch_name() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else if cfg!(target_arch = "x86") {
        "x86"
    } else {
        "x64"
    }
}

/// 运行时根目录
pub fn runtime_dir() -> PathBuf {
    crate::infra::paths::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("runtime")
}

/// 某个大版本的安装目录
pub fn version_dir(major: u32) -> PathBuf {
    runtime_dir().join(format!("java-{major}"))
}

/// 查某个大版本的最新 JRE 下载信息
pub async fn fetch_latest(http: &reqwest::Client, major: u32) -> Result<JreInfo, anyhow::Error> {
    let url = format!(
        "{API}/{major}/hotspot?os={}&arch={}&image_type=jre",
        os_name(),
        arch_name()
    );

    let res = http
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("连不上 Adoptium: {e}"))?;

    if !res.status().is_success() {
        return Err(anyhow::anyhow!(
            "Adoptium 查询失败（HTTP {}）",
            res.status().as_u16()
        ));
    }

    let list: Vec<Raw> = res
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Adoptium 响应解析失败: {e}"))?;

    let first = list
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("Adoptium 没有 Java {major} 的包"))?;

    let pkg = first
        .binary
        .as_ref()
        .and_then(|b| b.package.as_ref())
        .ok_or_else(|| anyhow::anyhow!("Adoptium 响应缺少下载信息"))?;

    let url = pkg
        .link
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Adoptium 没给下载链接"))?;

    Ok(JreInfo {
        major,
        version: first
            .binary
            .as_ref()
            .and_then(|b| b.version.as_ref())
            .and_then(|v| v.semver.clone())
            .or(first.release_name.clone())
            .unwrap_or_else(|| format!("Java {major}")),
        url,
        size: pkg.size.unwrap_or(0),
        name: pkg.name.clone().unwrap_or_else(|| format!("jre-{major}.zip")),
    })
}

/// 在目录里找 `bin/java.exe`（不同包结构不一样，递归找）
pub fn find_java_exe(dir: &Path) -> Option<PathBuf> {
    let exe = if cfg!(windows) { "java.exe" } else { "java" };

    let mut stack = vec![dir.to_path_buf()];
    let mut depth_guard = 0;

    while let Some(d) = stack.pop() {
        depth_guard += 1;
        if depth_guard > 4000 {
            break;
        }
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.file_name().and_then(|n| n.to_str()) == Some(exe) {
                // 确认在 bin 目录里
                if p.parent().and_then(|x| x.file_name()).and_then(|x| x.to_str()) == Some("bin") {
                    return Some(p);
                }
            }
        }
    }
    None
}

/// 解压 zip 到目标目录
fn extract_zip(zip_path: &Path, dest: &Path) -> Result<(), anyhow::Error> {
    use std::io::Write as _;

    let f = std::fs::File::open(zip_path)?;
    let mut zip = zip::ZipArchive::new(f)?;
    std::fs::create_dir_all(dest)?;

    for i in 0..zip.len() {
        let mut entry = match zip.by_index(i) {
            Ok(e) => e,
            Err(_) => continue,
        };
        if entry.is_dir() {
            continue;
        }

        let Some(rel) = entry.enclosed_name() else {
            continue;
        };

        let out_path = dest.join(&rel);
        if !out_path.starts_with(dest) {
            continue; // zip slip
        }

        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut out = std::fs::File::create(&out_path)?;
        std::io::copy(&mut entry, &mut out)?;

        // 让可执行文件带上执行权限（非 Windows 平台需要）
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let name = out_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name == "java" || out_path.starts_with(dest.join("bin")) {
                let _ = std::fs::set_permissions(
                    &out_path,
                    std::fs::Permissions::from_mode(0o755),
                );
            }
        }

        out.flush().ok();
    }

    Ok(())
}

/// 清华 TUNA 的 Adoptium 镜像地址（国内直连 GitHub 经常卡死，镜像几乎秒开）
fn mirror_url(major: u32, file_name: &str) -> String {
    format!(
        "https://mirrors.tuna.tsinghua.edu.cn/Adoptium/{}/{}/{}/{}/{}",
        major,
        "jre",
        arch_name(),
        os_name(),
        file_name
    )
}

/// 下载并安装某个大版本的 JRE，返回 `java` 可执行文件路径
pub async fn install(
    http: &reqwest::Client,
    major: u32,
    on_progress: impl Fn(u32, String) + Send + Sync + 'static,
) -> Result<PathBuf, anyhow::Error> {
    on_progress(2, format!("查询 Java {major} 最新版本…"));
    let info = fetch_latest(http, major).await?;

    let target = version_dir(major);

    // 已经装过 → 直接找可执行文件
    if target.is_dir() {
        if let Some(exe) = find_java_exe(&target) {
            on_progress(100, format!("Java {major} 已装好"));
            return Ok(exe);
        }
    }

    let zip_path = runtime_dir().join(&info.name);
    std::fs::create_dir_all(runtime_dir())?;

    on_progress(
        5,
        format!(
            "下载 {}（{:.0} MB）",
            info.version,
            info.size as f64 / 1024.0 / 1024.0
        ),
    );

    // 镜像优先，官方兜底
    let mirror = mirror_url(major, &info.name);
    let mut resp: Option<reqwest::Response> = None;
    for url in [mirror.as_str(), info.url.as_str()] {
        match http.get(url).send().await {
            Ok(r) if r.status().is_success() => {
                tracing::info!("Java 下载源: {url}");
                resp = Some(r);
                break;
            }
            Ok(r) => tracing::warn!("Java 下载源返回 {}: {url}", r.status().as_u16()),
            Err(e) => tracing::warn!("Java 下载源不可用（{e}）: {url}"),
        }
    }

    let mut resp = resp.ok_or_else(|| anyhow::anyhow!("所有 Java 下载源都不可用"))?;

    let total = resp.content_length().unwrap_or(info.size);
    {
        use tokio::io::AsyncWriteExt;
        let mut file = tokio::fs::File::create(&zip_path).await?;
        let mut done: u64 = 0;
        let mut last_pct = 0u32;

        while let Some(chunk) = resp.chunk().await? {
            file.write_all(&chunk).await?;
            done += chunk.len() as u64;

            let pct = if total > 0 {
                (5 + (done as f64 / total as f64 * 85.0) as u32).min(90)
            } else {
                50
            };
            if pct != last_pct {
                last_pct = pct;
                on_progress(
                    pct,
                    format!(
                        "下载 Java {} · {:.0}/{:.0} MB",
                        major,
                        done as f64 / 1024.0 / 1024.0,
                        total as f64 / 1024.0 / 1024.0
                    ),
                );
            }
        }
        file.flush().await?;
    }

    on_progress(92, "解压 Java…".to_string());

    // 解压到临时目录再改名，避免半成品
    let tmp_dir = runtime_dir().join(format!("java-{major}.tmp"));
    let _ = std::fs::remove_dir_all(&tmp_dir);
    extract_zip(&zip_path, &tmp_dir)?;

    let _ = std::fs::remove_dir_all(&target);
    std::fs::rename(&tmp_dir, &target)?;
    let _ = std::fs::remove_file(&zip_path);

    on_progress(98, "查找 java 可执行文件…".to_string());
    let exe = find_java_exe(&target)
        .ok_or_else(|| anyhow::anyhow!("解压后没找到 java 可执行文件"))?;

    on_progress(100, format!("Java {major} 安装完成"));
    Ok(exe)
}

/// 列出本地已经装好的运行时
pub fn list_installed() -> Vec<(u32, PathBuf)> {
    let dir = runtime_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if !name.starts_with("java-") || name.ends_with(".tmp") {
            continue;
        }
        let Ok(major) = name.trim_start_matches("java-").parse::<u32>() else {
            continue;
        };
        if let Some(exe) = find_java_exe(&e.path()) {
            out.push((major, exe));
        }
    }
    out.sort_by_key(|(m, _)| *m);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_dir_naming() {
        assert!(version_dir(21).to_string_lossy().ends_with("java-21"));
    }

    #[test]
    fn platform_ids_are_known() {
        assert!(["windows", "mac", "linux"].contains(&os_name()));
        assert!(["x64", "x86", "aarch64"].contains(&arch_name()));
    }
}
