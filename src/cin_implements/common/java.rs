//! Java jar启动器
//! * 🎯通用于任何基于jar文件的CIN，不仅仅是OpenNARS
//! * 🎯封装「NAVM运行时启动过程」中有关「Java启动环境配置」的部分
//! * 🚩从jar文件启动NARS
//! * 🚩【2024-03-27 15:31:02】取消「初始音量」的特化配置，将其变成一个「命令行参数生成器」而非独立的「启动器」

use crate::runtimes::CommandGenerator;
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
/// * 🎯以Java运行时专有形式启动虚拟机运行时
///   * 📄基于jar文件启动OpenNARS Shell
///   * 默认预置指令：`java -jar [.jar文件路径] [..其它jar启动参数]`
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CommandGeneratorJava {
    /// jar文件路径
    /// * 📌必须有
    jar_path: PathBuf,
    /// Java运行时的初始堆大小/最小堆大小
    /// * 📄在Java指令中的参数：`-Xms[数值]m`
    /// * 🚩可能没有：此时不会附加参数
    min_heap_size: Option<usize>,
    /// Java运行时的最大堆大小
    /// * 📄在Java指令中的参数：`-Xmx[数值]m`
    /// * 🚩可能没有：此时不会附加参数
    max_heap_size: Option<usize>,
}

impl CommandGeneratorJava {
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

/// 根据自身生成命令
impl CommandGenerator for CommandGeneratorJava {
    fn generate_command(&self) -> Command {
        // 构造指令
        let mut command_java = Command::new(COMMAND_JAVA);
        // * 📝这里的`args`、`arg都返回的可变借用。。
        command_java.args(COMMAND_ARGS_JAVA).arg(&self.jar_path);

        // 选择性添加参数
        if let Some(size) = self.min_heap_size {
            command_java.arg(command_arg_xms(size));
        }
        if let Some(size) = self.max_heap_size {
            command_java.arg(command_arg_xmx(size));
        }

        command_java
    }
}
