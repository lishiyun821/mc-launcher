use std::path::{Path, PathBuf};

use crate::core::models::Library;

use crate::services::install::rules;
use crate::services::install::version_json::ResolvedVersion;

/// Windows 的 classpath 分隔符
pub const SEPARATOR: &str = if cfg!(windows) { ";" } else { ":" };

/// 拼装 classpath
///
/// 组成（顺序与官方启动器一致）：
/// 1. 所有按 rules 筛过的 library jar（跳过 native 条目 —— 它们只解压不进 classpath）
/// 2. 最后加客户端 jar
///
/// 返回的每个元素都是绝对路径。
pub fn build(mc_dir: &Path, rv: &ResolvedVersion) -> Vec<PathBuf> {
    let libs_dir = mc_dir.join("libraries");
    let mut out: Vec<PathBuf> = Vec::new();

    for lib in &rv.libraries {
        if !rules::allows(&lib.rules) {
            continue;
        }
        // native 条目不进 classpath（1.21+ 是独立条目，带 :natives-xxx）
        if is_native_entry(lib) {
            continue;
        }
        // 两种给法，跟 install 那边保持一致：
        // ① 标准 Mojang 形式 —— downloads.artifact 里有 path
        // ② Maven 形式（Fabric / Quilt）—— 只有 url + sha1，路径从坐标推导
        //
        // 早先这里只认 ①，导致 Fabric 的 fabric-loader / mixin / asm 全被跳过，
        // classpath 里没有 KnotClient 所在的 jar，启动直接 ClassNotFound 秒退。
        let rel = if let Some(a) = lib.downloads.as_ref().and_then(|d| d.artifact.as_ref()) {
            a.path.clone().or_else(|| maven_rel(&lib.name))
        } else if lib.url.as_deref().filter(|u| !u.is_empty()).is_some() {
            maven_rel(&lib.name)
        } else {
            None
        };

        let Some(rel) = rel else {
            continue;
        };
        out.push(libs_dir.join(rel));
    }

    // 客户端 jar 放最后
    // 客户端 jar：优先用自己目录下的；继承版本（Fabric 等）没有就回落到父版本那份
    let own = mc_dir.join("versions").join(&rv.id).join(format!("{}.jar", rv.id));
    if own.is_file() {
        out.push(own);
    } else if let Some(parent) = rv.chain.iter().rev().find(|p| p.as_str() != rv.id.as_str()) {
        let pj = mc_dir.join("versions").join(parent).join(format!("{parent}.jar"));
        if pj.is_file() {
            tracing::info!("版本 {} 没有自己的客户端 jar，回落到父版本 {}", rv.id, parent);
            out.push(pj);
        } else {
            out.push(own);
        }
    } else {
        out.push(own);
    }

    out
}

/// 拼成 classpath 字符串
pub fn join(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(SEPARATOR)
}

/// 是不是 native 条目（只解压、不进 classpath）
fn is_native_entry(lib: &Library) -> bool {
    lib.name
        .split(':')
        .nth(3)
        .map(|cls| cls.starts_with("natives-"))
        .unwrap_or(false)
}

/// group:artifact:version[:classifier] → 相对路径
fn maven_rel(coords: &str) -> Option<String> {
    let base = coords.split('@').next().unwrap_or(coords);
    let parts: Vec<&str> = base.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    let (group, artifact, version) = (parts[0], parts[1], parts[2]);
    let classifier = parts.get(3).map(|c| format!("-{c}")).unwrap_or_default();
    let ext = coords.split('@').nth(1).unwrap_or("jar");
    Some(format!(
        "{}/{artifact}/{version}/{artifact}-{version}{classifier}.{ext}",
        group.replace('.', "/")
    ))
}

/// 检查 classpath 里缺失的文件（启动前自检用）
pub fn missing_files(mc_dir: &Path, rv: &ResolvedVersion) -> Vec<String> {
    build(mc_dir, rv)
        .into_iter()
        .filter(|p| !p.is_file())
        .map(|p| {
            p.strip_prefix(mc_dir)
                .unwrap_or(&p)
                .display()
                .to_string()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maven_rel_basic() {
        assert_eq!(
            maven_rel("org.ow2.asm:asm:9.7.1").as_deref(),
            Some("org/ow2/asm/asm/9.7.1/asm-9.7.1.jar")
        );
    }

    #[test]
    fn separator_is_platform_correct() {
        assert_eq!(SEPARATOR == ";", cfg!(windows));
    }
}
