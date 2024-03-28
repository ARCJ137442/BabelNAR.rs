//! 可执行文件（exe）启动器
//! * 🎯适用于任何直接从可执行文件（可能带参数）启动的CIN
//!   * 📄ONA
//!   * 📄NARS-Python
//! * 🚩【2024-03-28 10:00:00】暂且只需提供[`Command`]生成函数
//!   * ❗没必要使用新的数据结构

use crate::runtime::{CommandVm, IoTranslators};
use std::{ffi::OsStr, path::Path, process::Command};

/// 根据配置统一生成[`Command`]对象
/// * 📌「配置」的定义
///   * exe路径（可能不直接是可执行文件的路径）
///   * 当前文件夹（设置命令启动时的工作目录）
///   * 命令行参数（可以为空）
pub fn generate_command(
    exe_path: impl AsRef<OsStr>,
    current_dir: Option<impl AsRef<Path>>,
    args: &[&str],
) -> Command {
    // 构造指令
    let mut command = Command::new(exe_path);

    // 设置路径
    if let Some(current_dir) = current_dir {
        command.current_dir(current_dir);
    }

    // 设置附加参数
    // * 📝这里的`args`、`arg都返回的可变借用。。
    command.args(args);

    // 返回
    command
}

/// 根据「输入输出转译器」构建[`CommandVm`]对象
pub fn generate_command_vm(command: Command, translators: impl Into<IoTranslators>) -> CommandVm {
    CommandVm::from(command).translators(translators)
}
