//! OpenNARS运行时的启动器
//! * 🎯允许OpenNARS对原先运行时特别配置功能，同时也支持为OpenNARS定制配置
//! * 🚩只憎加「启动器」类型，而不增加「运行时」类型
//!   * ✨不同启动器可以启动到相同运行时

use super::{input_translate, output_translate};
use crate::runtime::{CommandVm, CommandVmRuntime};
use navm::vm::VmLauncher;
use std::{path::PathBuf, process::Command};

/// 启动Java运行时的命令
const COMMAND_JAVA: &str = "java";

/// jar文件启动的默认指令参数
/// * 🎯默认预置指令：`java -Xmx1024m -jar [.jar文件路径]`
const COMMAND_ARGS_JAVA: [&str; 2] = ["-Xmx1024m", "-jar"];

/// OpenNARS运行时启动器
/// * 🎯配置OpenNARS专有的东西
/// * 🎯以Java运行时专有形式启动OpenNARS
/// * 🚩基于jar文件启动OpenNARS Shell
///   * 默认预置指令：`java -Xmx1024m -jar [.jar文件路径]`
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OpenNARS {
    /// jar文件路径
    /// * 📌必须有
    jar_path: PathBuf,
    /// OpenNARS Shell
    default_volume: Option<usize>,
}

impl OpenNARS {
    pub fn new(jar_path: impl Into<PathBuf>) -> Self {
        Self {
            // 转换为路径
            jar_path: jar_path.into(),
            // 其它全是`None`
            ..Default::default()
        }
    }
}

/// 启动到「命令行运行时」
impl VmLauncher<CommandVmRuntime> for OpenNARS {
    fn launch(self) -> CommandVmRuntime {
        // 构造指令
        let mut command_java = Command::new(COMMAND_JAVA);
        // * 📝这里的`args`、`arg都返回的可变借用。。
        command_java.args(COMMAND_ARGS_JAVA).arg(self.jar_path);

        // 构造并启动虚拟机
        CommandVm::from_io_process(command_java.into())
            // * 🚩固定的「输入输出转换器」
            .input_translator(input_translate)
            .output_translator(output_translate)
            // 🔥启动
            .launch()
    }
}

// ! 单元测试见[`super`]
