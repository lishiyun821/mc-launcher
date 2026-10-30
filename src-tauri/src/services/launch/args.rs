use std::collections::HashMap;
use std::path::PathBuf;

use serde_json::Value;

use crate::services::install::rules;
use crate::services::install::version_json::ResolvedVersion;

use super::account::Account;
use super::classpath;

/// 启动所需的一切参数
#[derive(Debug, Clone)]
pub struct LaunchOptions {
    pub java: PathBuf,
    pub mc_dir: PathBuf,
    pub natives_dir: PathBuf,
    pub classpath: String,
    pub account: Account,
    /// 最大堆内存 MB
    pub memory_mb: u32,
    pub width: u32,
    pub height: u32,
    /// 用户追加的 JVM 参数
    pub extra_jvm: Vec<String>,
}

/// 组装变量表（用于替换 `${xxx}`）
fn variables(rv: &ResolvedVersion, opt: &LaunchOptions) -> HashMap<&'static str, String> {
    let mut m = HashMap::new();

    let libs_dir = opt.mc_dir.join("libraries");
    let assets_dir = opt.mc_dir.join("assets");

    m.insert("natives_directory", opt.natives_dir.display().to_string());
    m.insert("classpath", opt.classpath.clone());
    m.insert("classpath_separator", classpath::SEPARATOR.to_string());
    m.insert("library_directory", libs_dir.display().to_string());
    m.insert("launcher_name", "mc-launcher".to_string());
    m.insert("launcher_version", env!("CARGO_PKG_VERSION").to_string());

    m.insert("auth_player_name", opt.account.name.clone());
    m.insert("auth_uuid", opt.account.uuid.clone());
    m.insert("auth_access_token", opt.account.access_token.clone());
    m.insert("auth_session", opt.account.access_token.clone());
    m.insert("user_type", opt.account.user_type.clone());
    m.insert("user_properties", "{}".to_string());

    // 微软账户专有，离线模式留空
    m.insert("clientid", String::new());
    m.insert("auth_xuid", String::new());

    m.insert("version_name", rv.id.clone());
    m.insert("version_type", rv.kind.clone().unwrap_or_else(|| "release".into()));
    m.insert("game_directory", opt.mc_dir.display().to_string());
    m.insert("assets_root", assets_dir.display().to_string());
    m.insert("game_assets", assets_dir.display().to_string());
    m.insert("assets_index_name", rv.asset_index.id.clone());

    m.insert("resolution_width", opt.width.to_string());
    m.insert("resolution_height", opt.height.to_string());

    m
}

/// 把 `${key}` 替换成实际值；未知变量留空（官方启动器同行为）
fn substitute(s: &str, vars: &HashMap<&'static str, String>) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'$' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            if let Some(end) = s[i + 2..].find('}') {
                let key = &s[i + 2..i + 2 + end];
                match vars.get(key) {
                    Some(v) => out.push_str(v),
                    None => {
                        // 未定义变量替换为空
                        tracing::debug!("参数里的未知变量 ${{{key}}}");
                    }
                }
                i = i + 2 + end + 1;
                continue;
            }
        }
        // 逐字符推进（注意 UTF-8 边界）
        let ch = s[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }

    out
}

/// 展开 arguments 数组：[ 字符串 或 { rules, value } ]
fn flatten(items: &[Value], vars: &HashMap<&'static str, String>, out: &mut Vec<String>) {
    for item in items {
        match item {
            Value::String(s) => out.push(substitute(s, vars)),
            Value::Object(obj) => {
                let rules: Vec<crate::core::models::Rule> = obj
                    .get("rules")
                    .and_then(|r| serde_json::from_value(r.clone()).ok())
                    .unwrap_or_default();

                if !rules::allows(&rules) {
                    continue;
                }

                match obj.get("value") {
                    Some(Value::String(s)) => out.push(substitute(s, vars)),
                    Some(Value::Array(arr)) => {
                        for v in arr {
                            if let Some(s) = v.as_str() {
                                out.push(substitute(s, vars));
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

/// 生成 JVM 参数（不含 java 可执行文件本身）
pub fn build_jvm_args(rv: &ResolvedVersion, opt: &LaunchOptions) -> Vec<String> {
    let vars = variables(rv, opt);
    let mut out = vec![format!("-Xmx{}M", opt.memory_mb.max(512))];
    out.push(format!("-Xms{}M", (opt.memory_mb / 2).max(256)));

    // 现代版本（1.13+）走 arguments.jvm
    if !rv.arguments.jvm.is_empty() {
        flatten(&rv.arguments.jvm, &vars, &mut out);
    }

    // 兜底：如果 arguments.jvm 里没给 -cp，就手动补上
    if !out.iter().any(|a| a == "-cp" || a == "-classpath") {
        out.push("-Djava.library.path=".to_string() + &vars["natives_directory"]);
        out.push("-cp".to_string());
        out.push(opt.classpath.clone());
    }

    out.extend(opt.extra_jvm.iter().cloned());
    out
}

/// 生成游戏参数（含主类之前的都算 JVM，主类之后是游戏参数）
pub fn build_game_args(rv: &ResolvedVersion, opt: &LaunchOptions) -> Vec<String> {
    let vars = variables(rv, opt);
    let mut out = Vec::new();

    if !rv.arguments.game.is_empty() {
        flatten(&rv.arguments.game, &vars, &mut out);
        return out;
    }

    // 老版本（≤1.12）：用 minecraftArguments 这一整串
    if let Some(raw) = &rv.minecraft_arguments {
        for token in raw.split_whitespace() {
            out.push(substitute(token, &vars));
        }
    }

    out
}

/// 拼出完整命令行（java 在第一位）
pub fn build_full_command(rv: &ResolvedVersion, opt: &LaunchOptions) -> Vec<String> {
    let mut cmd = vec![opt.java.display().to_string()];
    cmd.extend(build_jvm_args(rv, opt));
    cmd.push(rv.main_class.clone());
    cmd.extend(build_game_args(rv, opt));
    cmd
}

/// 给用户看的命令行摘要（把超长的 classpath 缩写）
pub fn preview_command(rv: &ResolvedVersion, opt: &LaunchOptions) -> String {
    let cmd = build_full_command(rv, opt);
    let mut parts: Vec<String> = Vec::with_capacity(cmd.len());

    for (i, arg) in cmd.iter().enumerate() {
        if i == 0 {
            parts.push(format!("\"{arg}\""));
        } else if arg.len() > 120 && arg.contains(classpath::SEPARATOR) {
            parts.push(format!("<classpath {} 项>", opt.classpath.matches(classpath::SEPARATOR).count() + 1));
        } else if arg.contains(' ') {
            parts.push(format!("\"{arg}\""));
        } else {
            parts.push(arg.clone());
        }
    }

    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitute_basic() {
        let mut vars = HashMap::new();
        vars.insert("version_name", "1.21.11".to_string());
        vars.insert("game_directory", "C:/mc".to_string());
        assert_eq!(
            substitute("--version ${version_name} --gameDir ${game_directory}", &vars),
            "--version 1.21.11 --gameDir C:/mc"
        );
    }

    #[test]
    fn substitute_unknown_becomes_empty() {
        let vars = HashMap::new();
        assert_eq!(substitute("--x ${nope} --y 1", &vars), "--x  --y 1");
    }

    #[test]
    fn substitute_keeps_plain_text() {
        let vars = HashMap::new();
        assert_eq!(substitute("--demo", &vars), "--demo");
    }
}
