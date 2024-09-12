//! CXinNARS.js运行时的启动器
//! * 🎯允许CXinNARS.js对原先运行时特别配置功能，同时也支持为CXinNARS.js定制配置
//! * 🚩只憎加「启动器」类型，而不增加「运行时」类型
//!   * ✨不同启动器可以启动到相同运行时

use super::{input_translate, output_translate};
use crate::{
    cin_implements::common::{generate_command_vm, CommandGeneratorNodeJS},
    runtimes::{CommandGenerator, CommandVmRuntime},
};
use anyhow::Result;
use nar_dev_utils::pipe;
use navm::vm::VmLauncher;
use std::path::PathBuf;

/// CXinNARS.js Shell启动的默认指令参数
/// * 🎯默认预置指令：`[.js文件路径] shell`
const COMMAND_ARGS_CXIN_NARS: [&str; 1] = ["shell"];

/// CXinNARS.js运行时启动器
/// * 🎯配置CXinNARS.js专有的东西
/// * 🚩基于js文件启动CXinNARS.js Shell
///   * 默认预置指令：`[.js文件路径] shell`
/// * 🚩【2024-03-25 08:51:30】目前保留原有缩写的大小写风格，与OpenNARS、PyNARS一致
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CXinJS {
    /// Node.js命令生成器
    command_generator: CommandGeneratorNodeJS,
}

// ! 🚩【2024-03-25 09:37:22】目前暂时不提取至「VmExe」：预置的`shell`参数需要被处理
impl CXinJS {
    /// 构造函数
    pub fn new(js_path: impl Into<PathBuf>) -> Self {
        Self {
            command_generator: CommandGeneratorNodeJS::new(js_path, COMMAND_ARGS_CXIN_NARS),
        }
    }
}

/// 启动到「命令行运行时」
impl VmLauncher for CXinJS {
    type Runtime = CommandVmRuntime;
    fn launch(self) -> Result<CommandVmRuntime> {
        // 构造并启动虚拟机
        pipe! {
            self.command_generator
            // 构造指令 | 预置的指令参数
            => .generate_command()
            // * 🚩固定的「输入输出转译器」
            => generate_command_vm(_, (input_translate, output_translate))
            // 🔥启动
            => .launch()
        }
    }
}

// ! 单元测试见[`super`]
