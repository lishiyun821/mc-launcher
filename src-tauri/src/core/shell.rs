//! 子进程启动的统一入口。
//!
//! Windows 上凡是启动控制台程序（java、reg、wmic 之类）而不隐藏窗口的，
//! 都会在屏幕上闪一下黑色控制台窗口 —— 对启动器来说是明显的观感问题。
//! 这里统一加上 `CREATE_NO_WINDOW`，杜绝闪框。

use tokio::process::Command;

/// Windows 的「不创建控制台窗口」标志位
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// 建一个不会弹控制台窗口的子进程命令
pub fn hidden_command<S: AsRef<std::ffi::OsStr>>(program: S) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}
