use std::path::PathBuf;


use crate::core::models::JavaInfo;

/// 扫描系统中的 Java：注册表常见路径 + PATH + JAVA_HOME
pub async fn scan() -> Vec<JavaInfo> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    // 1) JAVA_HOME
    if let Ok(jh) = std::env::var("JAVA_HOME") {
        candidates.push(PathBuf::from(jh).join("bin").join(exe("java")));
    }

    // 2) PATH 里的 java
    if let Ok(path) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path) {
            let p = dir.join(exe("java"));
            if p.exists() {
                candidates.push(p);
                break; // PATH 上只取第一个
            }
        }
    }

    // 3) 常见安装位置
    let roots = [
        r"C:\Program Files\Java",
        r"C:\Program Files\Eclipse Adoptium",
        r"C:\Program Files\Microsoft",
        r"C:\Program Files\Zulu",
        r"C:\Program Files\Amazon Corretto",
        r"C:\Program Files (x86)\Java",
    ];
    for root in roots {
        let Ok(entries) = std::fs::read_dir(root) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path().join("bin").join(exe("java"));
            if p.exists() {
                candidates.push(p);
            }
        }
    }

    // 4) 启动器自己下载的运行时
    if let Ok(entries) = std::fs::read_dir(crate::services::java_download::runtime_dir()) {
        for e in entries.flatten() {
            if let Some(exe) = crate::services::java_download::find_java_exe(&e.path()) {
                candidates.push(exe);
            }
        }
    }

    // 5) 把 Windows 的 javapath 转发器换成真实路径
    //
    // `C:\Program Files\Common Files\Oracle\Java\javapath\java.exe` 不是真的 JVM，
    // 而是一个转发 shim：它把 argv 重新拼成命令字符串再解析一遍。
    // 带空格的参数会被拆开（Fabric 就会传 `-DFabricMcEmu= net.minecraft.client.main.Main `），
    // 拆开后主类位置错位，JVM 立刻以退出码 1 结束，而且不输出任何东西。
    // 所以这里尽量解析成真实路径；解析不出来就排到最后，让真实 JDK 优先。
    let mut shims: Vec<PathBuf> = Vec::new();
    candidates = candidates
        .into_iter()
        .filter_map(|p| {
            let is_shim = p
                .to_string_lossy()
                .to_ascii_lowercase()
                .contains("oracle\\java\\javapath") ||
                p.to_string_lossy().to_ascii_lowercase().contains("javapath_target");

            if !is_shim {
                return Some(p);
            }
            // 试着顺着链接找真身
            match std::fs::canonicalize(&p) {
                Ok(real) if real.is_file() && !real.to_string_lossy().contains("javapath") => {
                    tracing::debug!("javapath 转发器 {} 解析到 {}", p.display(), real.display());
                    Some(real)
                }
                _ => {
                    shims.push(p);
                    None
                }
            }
        })
        .collect();

    // 6) 去重（真实 JDK 在前，shim 兜底放最后）
    candidates.sort();
    candidates.dedup();
    let mut all = candidates;
    all.extend(shims);

    let mut out = Vec::new();
    for c in all {
        if let Some(info) = probe(&c).await {
            out.push(info);
        }
    }

    // 按大版本降序
    out.sort_by(|a, b| b.major.cmp(&a.major));
    out.dedup_by(|a, b| a.version == b.version && a.path == b.path);
    out
}

fn exe(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

/// 执行 java -version 解析版本
/// 注意：Java 把版本信息打到 **stderr**，不是 stdout
async fn probe(path: &PathBuf) -> Option<JavaInfo> {
    let out = crate::core::shell::hidden_command(path).arg("-version").output().await.ok()?;
    let text = String::from_utf8_lossy(&out.stderr).to_string()
        + &String::from_utf8_lossy(&out.stdout);

    let version = parse_version(&text)?;
    let major = parse_major(&version)?;

    Some(JavaInfo {
        path: path.to_string_lossy().to_string(),
        version,
        major,
        vendor: parse_vendor(&text),
        is64_bit: text.contains("64-Bit") || text.contains("64-bit"),
    })
}

/// 从 `java version "21.0.2" 2024-01-16` 里取版本号
fn parse_version(text: &str) -> Option<String> {
    // 优先匹配带引号的
    if let Some(start) = text.find('"') {
        if let Some(end) = text[start + 1..].find('"') {
            return Some(text[start + 1..start + 1 + end].to_string());
        }
    }
    // fallback: "openjdk 21.0.2 2024-01-16"
    for line in text.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("openjdk version ") {
            return Some(rest.trim_matches('"').split_whitespace().next()?.to_string());
        }
    }
    None
}

/// 由版本号推出主版本：1.8.0_392 → 8，21.0.2 → 21
fn parse_major(version: &str) -> Option<u32> {
    let mut parts = version.split('.');
    let first: u32 = parts.next()?.parse().ok()?;
    if first == 1 {
        parts.next()?.parse().ok()
    } else {
        Some(first)
    }
}

fn parse_vendor(text: &str) -> String {
    let lower = text.to_lowercase();
    let table = [
        ("temurin", "Eclipse Temurin"),
        ("adoptium", "Eclipse Temurin"),
        ("zulu", "Azul Zulu"),
        ("corretto", "Amazon Corretto"),
        ("microsoft", "Microsoft OpenJDK"),
        ("graalvm", "GraalVM"),
        ("openjdk", "OpenJDK"),
        ("hotspot", "Oracle/OpenJDK"),
    ];
    for (k, v) in table {
        if lower.contains(k) {
            return v.to_string();
        }
    }
    "Unknown".to_string()
}
