//! Python模块 启动器
//! * 🎯通用于任何基于Python源码的CIN，不仅仅是PyNARS
//! * 🎯封装「NAVM运行时启动过程」中有关「Python启动环境配置」的部分
//! * 🚩从Python模块（`.py`脚本）启动NARS

use crate::runtimes::CommandGenerator;
use std::{path::PathBuf, process::Command};

/// 启动Python运行时的命令
const COMMAND_PYTHON: &str = "python";

/// 启动Python模块的默认指令参数
/// * 🎯默认预置指令：`python -m [当前工作目录下的Python模块]`
const COMMAND_ARGS_PYTHON: [&str; 1] = ["-m"];

/// Python启动命令生成器
/// * 🎯以Python模块形式生成启动命令
/// * 🚩【2024-03-25 08:55:07】基于Python模块文件启动NARS
///   * 默认预置指令：`python -m [Python模块根目录] [Python模块路径]`
/// * 🚩【2024-03-25 09:15:07】删去[`Default`]派生：因为可能导致无效的路径
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CommandGeneratorPython {
    /// 根目录
    /// * 📄`root/home/dev/pynars`
    root_path: PathBuf,

    /// 模块路径
    /// * 📌相对根目录而言
    /// * 📄`pynars.Console`
    /// * 📄`root_path` + `pynars.Console` => `root_path/pynars/Console`
    module_path: String,
}

impl CommandGeneratorPython {
    pub fn new(root_path: impl Into<PathBuf>, module_path: &str) -> Self {
        Self {
            // 转换为路径
            root_path: root_path.into(),
            // 转换为字符串
            module_path: module_path.to_string(),
        }
    }
}

/// 启动到「命令行运行时」
impl CommandGenerator for CommandGeneratorPython {
    fn generate_command(&self) -> Command {
        // 构造指令
        let mut command = Command::new(COMMAND_PYTHON);
        command
            // * 🚩设置指令工作目录
            // * 📝`python -m`无法自行指定所执行的工作目录，必须在`Command`中设置
            .current_dir(&self.root_path) // 以此设置当前工作目录
            .args(COMMAND_ARGS_PYTHON)
            .arg(&self.module_path);

        command
    }
}
