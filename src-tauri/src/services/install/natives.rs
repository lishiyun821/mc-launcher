use std::io::Write;
use std::path::Path;

use zip::ZipArchive;

/// 解压 native jar 到目标目录
///
/// 规则（与官方启动器一致）：
/// - 跳过目录条目
/// - 跳过 `META-INF/`
/// - 跳过版本 JSON 的 `extract.exclude` 里列出的前缀
/// - 防 zip slip：解压路径必须落在目标目录内
pub fn extract_jar(
    jar: &Path,
    dest: &Path,
    exclude: &[String],
) -> Result<usize, anyhow::Error> {
    let file = std::fs::File::open(jar)
        .map_err(|e| anyhow::anyhow!("打开 {} 失败: {e}", jar.display()))?;
    let mut zip = ZipArchive::new(file)
        .map_err(|e| anyhow::anyhow!("{} 不是有效 zip: {e}", jar.display()))?;

    std::fs::create_dir_all(dest)?;

    let mut count = 0usize;

    for i in 0..zip.len() {
        let mut entry = match zip.by_index(i) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("跳过损坏条目 #{i}: {e}");
                continue;
            }
        };

        if entry.is_dir() {
            continue;
        }

        let name = entry.name().to_string();

        if name.starts_with("META-INF/") || name.starts_with("META-INF\\") {
            continue;
        }
        if exclude
            .iter()
            .any(|ex| !ex.is_empty() && (name.starts_with(ex.as_str()) || name.starts_with(&ex.replace('/', "\\"))))
        {
            continue;
        }

        let target = dest.join(&name);

        // zip slip 防护
        if !target.starts_with(dest) {
            tracing::warn!("跳过越界条目: {name}");
            continue;
        }

        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut out = std::fs::File::create(&target)?;
        std::io::copy(&mut entry, &mut out)?;
        out.flush()?;
        count += 1;
    }

    Ok(count)
}

/// 批量解压所有 native jar
pub fn extract_all(
    jars: &[std::path::PathBuf],
    dest: &Path,
    exclude: &[String],
) -> Result<(usize, usize), anyhow::Error> {
    let mut files = 0usize;
    let mut jars_ok = 0usize;

    for jar in jars {
        match extract_jar(jar, dest, exclude) {
            Ok(n) => {
                files += n;
                jars_ok += 1;
            }
            Err(e) => tracing::warn!("解压 {} 失败: {e}", jar.display()),
        }
    }

    tracing::info!("native 解压完成：{jars_ok} 个 jar → {files} 个文件");
    Ok((jars_ok, files))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    #[test]
    fn excludes_meta_inf() {
        // 造一个最小 zip：META-INF/MANIFEST.MF + 一个 dll
        let dir = std::env::temp_dir().join("mcl_natives_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let jar = dir.join("t.jar");

        {
            let f = std::fs::File::create(&jar).unwrap();
            let mut zw = zip::ZipWriter::new(f);
            let opts = zip::write::SimpleFileOptions::default();
            zw.start_file("META-INF/MANIFEST.MF", opts).unwrap();
            zw.write_all(b"Manifest-Version: 1.0").unwrap();
            zw.start_file("lwjgl.dll", opts).unwrap();
            zw.write_all(b"fake dll").unwrap();
            zw.finish().unwrap();
        }

        let dest = dir.join("out");
        let n = extract_jar(&jar, &dest, &[]).unwrap();
        assert_eq!(n, 1, "只应解出 dll，不该解 MANIFEST");
        assert!(dest.join("lwjgl.dll").is_file());
        assert!(!dest.join("META-INF").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
