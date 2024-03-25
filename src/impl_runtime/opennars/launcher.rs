//! Java jar启动器
//! * 📌OpenNARS运行时的启动器
//! * 🎯允许OpenNARS对原先运行时特别配置功能，同时也支持为OpenNARS定制配置
//! * 🚩从jar文件启动NARS
//! * 🚩只憎加「启动器」类型，而不增加「运行时」类型
//!   * ✨不同启动器可以启动到相同运行时

use super::{input_translate, output_translate};
use crate::runtime::{CommandVm, CommandVmRuntime};
use navm::{
    cmd::Cmd,
    vm::{VmLauncher, VmRuntime},
};
use std::{path::PathBuf, process::Command};

/// 启动Java运行时的命令
const COMMAND_JAVA: &str = "java";

/// jar文件启动的默认指令参数
/// * 🎯默认预置指令：`java -Xmx1024m -jar [.jar文件路径]`
/// * 🚩实际上"-Xmx1024m"非必要
const COMMAND_ARGS_JAVA: [&str; 1] = ["-jar"];

/// Java运行时启动配置参数：初始堆大小/最小堆大小
#[inline(always)]
fn command_arg_xms(size: usize) -> String {
    format!("-Xms{size}m")
}

/// Java运行时启动配置参数：最大堆大小
#[inline(always)]
fn command_arg_xmx(size: usize) -> String {
    format!("-Xmx{size}m")
}

/// Java jar启动器
/// * 🎯配置OpenNARS专有的东西
/// * 🎯以Java运行时专有形式启动OpenNARS
/// * 🚩基于jar文件启动OpenNARS Shell
///   * 默认预置指令：`java -jar [.jar文件路径]`
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct VmJava {
    /// jar文件路径
    /// * 📌必须有
    jar_path: PathBuf,
    /// NARS的初始音量
    /// * 🚩可能没有：此时不会输入指令
    initial_volume: Option<usize>,
    /// Java运行时的初始堆大小/最小堆大小
    /// * 📄在Java指令中的参数：`-Xms[数值]m`
    /// * 🚩可能没有：此时不会附加参数
    min_heap_size: Option<usize>,
    /// Java运行时的最大堆大小
    /// * 📄在Java指令中的参数：`-Xmx[数值]m`
    /// * 🚩可能没有：此时不会附加参数
    max_heap_size: Option<usize>,
}

/// 兼容性别名
#[doc(alias = "VmJava")]
pub type OpenNARS = VmJava;

impl VmJava {
    /// 构造函数
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
impl VmLauncher<CommandVmRuntime> for VmJava {
    fn launch(self) -> CommandVmRuntime {
        // 构造指令
        let mut command_java = Command::new(COMMAND_JAVA);
        // * 📝这里的`args`、`arg都返回的可变借用。。
        command_java.args(COMMAND_ARGS_JAVA).arg(self.jar_path);

        // 选择性添加参数 |设置初始音量
        if let Some(size) = self.min_heap_size {
            command_java.arg(command_arg_xms(size));
        }
        if let Some(size) = self.max_heap_size {
            command_java.arg(command_arg_xmx(size));
        }

        // 构造并启动虚拟机
        let mut vm = CommandVm::from_io_process(command_java.into())
            // * 🚩固定的「输入输出转换器」
            .input_translator(input_translate)
            .output_translator(output_translate)
            // 🔥启动
            .launch();
        // 设置初始音量
        self.initial_volume.inspect(|volume| {
            // 输入指令，并在执行错误时打印信息
            if let Err(e) = vm.input_cmd(Cmd::VOL(*volume)) {
                println!("无法设置初始音量「{volume}」：{e}");
            }
        });
        // 返回
        vm
    }
}

// ! 单元测试见[`super`]
