//! Python模块 启动器
//! * 📌PyNARS运行时的启动器
//! * 🎯允许PyNARS对原先运行时特别配置功能，同时也支持为PyNARS定制配置
//! * 🚩只憎加「启动器」类型，而不增加「运行时」类型
//!   * ✨不同启动器可以启动到相同运行时

use super::{input_translate, output_translate};
use crate::runtime::{CommandVm, CommandVmRuntime};
use navm::vm::VmLauncher;
use std::{path::PathBuf, process::Command};

/// 启动Python运行时的命令
const COMMAND_PYTHON: &str = "python";

/// 启动Python模块的默认指令参数
/// * 🎯默认预置指令：`python -m [当前工作目录下的Python模块]`
const COMMAND_ARGS_PYTHON: [&str; 1] = ["-m"];

/// PyNARS运行时启动器
/// * 🎯配置PyNARS专有的东西
/// * 🎯以Python模块形式启动PyNARS
/// * 📌没有内置的「音量」配置
///   * ⚠️该配置参考的是PyNARS的`ConsolePlus`模块
/// * 🚩【2024-03-25 08:55:07】基于Python模块文件启动PyNARS Shell
///   * 默认预置指令：`python -m [Python模块根目录] [Python模块路径]`
/// * 🚩【2024-03-25 09:15:07】删去[`Default`]派生：因为可能导致无效的路径
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct VmPython {
    /// 根目录
    /// * 📄`root/home/dev/pynars`
    root_path: PathBuf,

    /// 模块路径
    /// * 📌相对根目录而言
    /// * 📄`pynars.Console`
    /// * 📄`root_path` + `pynars.Console` => `root_path/pynars/Console`
    module_path: String,
}

/// 兼容性别名
#[doc(alias = "VmPython")]
pub type PyNARS = VmPython;

impl VmPython {
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
impl VmLauncher<CommandVmRuntime> for VmPython {
    fn launch(self) -> CommandVmRuntime {
        // 构造指令
        let mut command = Command::new(COMMAND_PYTHON);
        command
            // * 🚩设置指令工作目录
            // * 📝`python -m`无法自行指定所执行的工作目录，必须在`Command`中设置
            .current_dir(self.root_path) // 以此设置当前工作目录
            .args(COMMAND_ARGS_PYTHON)
            .arg(self.module_path);

        // 构造并启动虚拟机
        CommandVm::from_io_process(command.into())
            // * 🚩固定的「输入输出转换器」
            .input_translator(input_translate)
            .output_translator(output_translate)
            // 🔥启动
            .launch()
    }
}

// ! 单元测试见[`super`]
