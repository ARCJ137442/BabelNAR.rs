//! 命令行虚拟机 运行时
//! * ✨核心内容
//!   * ⇄ 基于「进程通信」的消息互转
//!     * 📌核心IO流程：
//!       1. NAVM指令[`Cmd`] >>> 进程输入 >>> 子进程
//!       2. 子进程 >>> 进程输出 >>> NAVM输出[`Output`]
//!     * 🚩实现方式：两处转译器

use super::{CommandVm, InputTranslator, OutputTranslator};
use crate::process_io::IoProcessManager;
use navm::{
    cmd::Cmd,
    output::Output,
    vm::{VmLauncher, VmRuntime},
};
use util::ResultS;

/// 命令行虚拟机运行时
/// * 🎯封装「进程通信」逻辑
pub struct CommandVmRuntime {
    /// 封装的「进程管理者」
    /// * 🚩使用[`IoProcessManager`]封装「进程通信」的逻辑细节
    process: IoProcessManager,

    /// [`Cmd`]→进程输入 转译器
    input_translator: Box<InputTranslator>,

    /// 进程输出→[`Output`]转译器
    /// * 🚩【2024-03-24 02:06:27】至于「输出侦听」等后续处理，外置给其它专用「处理者」
    output_translator: Box<OutputTranslator>,
}

impl VmRuntime for CommandVmRuntime {
    fn input_cmd(&mut self, cmd: Cmd) -> ResultS<()> {
        // 尝试转译
        let input = (self.input_translator)(cmd)?;
        // 置入转译结果
        self.process.put_line(input)
    }

    fn fetch_output(&mut self) -> ResultS<Output> {
        let s = self.process.fetch_output()?;
        (self.output_translator)(s)
    }

    fn try_fetch_output(&mut self) -> ResultS<Option<Output>> {
        let s = self.process.try_fetch_output()?;
        // 匹配分支
        match s {
            // 有输出⇒尝试转译并返回
            Some(s) => Ok(Some((self.output_translator)(s)?)),
            // 没输出⇒没输出 | ⚠️注意：不能使用`map`，否则`?`穿透不出闭包
            None => Ok(None),
        }
    }
}

/// 构建功能：启动命令行虚拟机
impl VmLauncher<CommandVmRuntime> for CommandVm {
    fn launch(self) -> CommandVmRuntime {
        CommandVmRuntime {
            // 启动内部的「进程管理者」
            process: self.io_process.launch(),
            // 输入转译器
            input_translator: self
                .input_translator
                // 默认值：直接调用Cmd的`to_string`方法 | 使用NAVM Cmd语法
                .unwrap_or(Box::new(|cmd| Ok(cmd.to_string()))),
            // 输出转译器
            output_translator: self
                .output_translator
                // 默认值：直接归入「其它」输出 | 约等于不分类
                .unwrap_or(Box::new(|content| Ok(Output::OTHER { content }))),
            // * 🚩【2024-03-24 02:06:59】目前到此为止：只需处理「转译」问题
        }
    }
}

/// 单元测试
#[cfg(test)]
mod test {
    use super::*;
    use crate::process_io::tests::await_fetch_until;
    use narsese::conversion::string::impl_lexical::shortcuts::*;

    // 定义一系列路径
    #[allow(dead_code)]
    const EXE_PATH_ONA: &str = r"..\..\NARS-executables\NAR.exe";
    #[allow(dead_code)]
    const EXE_PATH_PYNARS: &str = r"..\..\NARS-executables\launch-pynars-console-plus.cmd";
    #[allow(dead_code)]
    const JAR_PATH_OPENNARS: &str = r"..\..\NARS-executables\opennars.jar";

    /// 示例测试 | PyNARS
    #[test]
    fn test_pynars() {
        let mut vm = CommandVm::new(EXE_PATH_PYNARS)
            // 输入转换器：直接取其尾部
            .input_translator(|cmd| Ok(cmd.tail()))
            // 🔥启动
            .launch();

        // // 睡眠等待
        // // std::thread::sleep(std::time::Duration::from_secs(1));
        // ! ↑现在无需睡眠等待：输入会自动在初始化后写入子进程

        let mut input_cmd_and_await = |cmd, contains: &str| {
            // 构造并输入任务
            vm.input_cmd(cmd).expect("无法输入指令！");
            // ! 目前还是失败

            // 必要时等待
            if !contains.is_empty() {
                await_fetch_until(&mut vm.process, |s| s.contains(contains));
            }
        };

        // 构造并输入任务 | 输入进PyNARS后变成了紧凑版本
        input_cmd_and_await(Cmd::NSE(nse_task!(<A --> B>.)), "<A-->B>.");
        input_cmd_and_await(Cmd::NSE(nse_task!(<B --> C>.)), "<B-->C>.");
        input_cmd_and_await(Cmd::NSE(nse_task!(<A --> C>?)), "<A-->C>?");
        input_cmd_and_await(Cmd::CYC(5), ""); // * CYC无需自动等待

        // 等待回答
        await_fetch_until(&mut vm.process, |s| {
            s.contains("ANSWER") && s.contains("<A-->C>.")
        });

        // 打印所有输出
        while let Some(output) = vm.try_fetch_output().unwrap() {
            println!("{:?}", output);
        }
    }
}
