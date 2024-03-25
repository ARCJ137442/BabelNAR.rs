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

    fn terminate(self) -> ResultS<()> {
        // 杀死子进程
        self.process.kill()?;
        Ok(())
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
pub(crate) mod test {
    use super::*;
    use narsese::conversion::string::impl_lexical::shortcuts::*;
    use std::process::Command;
    use util::first;

    // 定义一系列路径
    #[allow(dead_code)]
    pub const EXE_PATH_ONA: &str = r"..\..\NARS-executables\NAR.exe";
    #[allow(dead_code)]
    pub const EXE_PATH_PYNARS: &str = r"..\..\NARS-executables\launch-pynars-console-plus.cmd";
    pub const JAR_PATH_OPENNARS: &str = r"..\..\NARS-executables\opennars-304-T-modified.jar";

    const COMMAND_JAVA: &str = "java";
    const COMMAND_ARGS_JAVA: [&str; 2] = ["-Xmx1024m", "-jar"];

    /// 实用测试工具/等待
    pub fn await_fetch_until(
        vm: &mut CommandVmRuntime,
        criterion: impl Fn(&Output, String) -> bool,
    ) -> Output {
        // 不断拉取输出
        // TODO: 💭【2024-03-24 18:21:28】后续可以结合「流式处理者列表」做集成测试
        loop {
            // 拉取输出及其内容 | ⚠️必要时等待（阻塞！）
            let output = vm.fetch_output().unwrap();
            let raw_content = output.raw_content();
            // 展示输出
            match &output {
                // 特别显示「回答」
                Output::ANSWER { content_raw, .. } => println!("捕获到回答！内容：{content_raw}"),
                _ => println!("捕获到其它输出！内容：{output:?}"),
            }
            // 包含⇒结束
            if criterion(&output, raw_content) {
                break output;
            }
        }
    }

    /// 实用测试工具/输入并等待
    pub fn input_cmd_and_await(
        vm: &mut CommandVmRuntime,
        cmd: Cmd,
        criterion: impl Fn(&Output, String) -> bool,
    ) -> Output {
        // 构造并输入任务
        vm.input_cmd(cmd).expect("无法输入指令！");
        // 「contains」非空⇒等待
        await_fetch_until(vm, criterion)
    }

    /// 实用测试工具/输入并等待「是否包含」
    /// * 🚩`input_cmd_and_await`的简单封装
    /// * 🎯【2024-03-24 18:38:50】用于「输出转换」尚未成熟时
    #[inline(always)]
    pub fn input_cmd_and_await_contains(
        vm: &mut CommandVmRuntime,
        cmd: Cmd,
        expected_contains: &str,
    ) -> Option<Output> {
        // 空预期⇒直接输入
        // * 🎯在后边测试中统一使用闭包，并且不会因此「空头拉取输出」
        //   * 📄【2024-03-24 18:47:20】有过「之前的CYC把Answer拉走了，导致后边的Answer等不到」的情况
        // * ⚠️不能简化：区别在「是否会拉取输入，即便条件永真」
        match expected_contains.is_empty() {
            true => {
                vm.input_cmd(cmd).expect("无法输入NAVM指令！");
                None
            }
            false => Some(input_cmd_and_await(vm, cmd, |_, raw_content| {
                raw_content.contains(expected_contains)
            })),
        }
    }

    /// 示例测试 | OpenNARS
    /// * 🚩通过Java命令启动
    #[test]
    fn test_opennars() {
        // 构造指令
        let mut command_java = Command::new(COMMAND_JAVA);
        // * 📝这里的`args`、`arg都返回的可变借用。。
        command_java
            .args(COMMAND_ARGS_JAVA)
            .arg(JAR_PATH_OPENNARS)
            // OpenNARS的默认参数 | ["null", "null", "null", "null"]
            // * 🔗https://github.com/opennars/opennars/blob/master/src/main/java/org/opennars/main/Shell.java
            // * ✅fixed「额外参数」问题：之前「IO进程」的测试代码`.arg("shell")`没删干净
            // .args(["null", "null", "null", "null"])
            ;
        // dbg!(&command_java);

        /// 临时构建的「输入转换」函数
        /// * 🎯用于转换`VOL 0`⇒`*volume=0`，避免大量输出造成进程卡顿
        fn input_translate(cmd: Cmd) -> ResultS<String> {
            let content = match cmd {
                // 直接使用「末尾」，此时将自动格式化任务（可兼容「空预算」的形式）
                Cmd::NSE(..) => cmd.tail(),
                // CYC指令：运行指定周期数
                Cmd::CYC(n) => n.to_string(),
                // VOL指令：调整音量
                Cmd::VOL(n) => format!("*volume={n}"),
                // 其它类型
                _ => return Err(format!("未知指令：{cmd:?}")),
            };
            // 转换
            Ok(content)
        }

        /// 临时构建的「输出转换」函数
        fn output_translate(content: String) -> ResultS<Output> {
            // 读取输出
            let output = first! {
                // 捕获Answer
                content.contains("Answer") => Output::ANSWER { content_raw: content, narsese: None },
                // 捕获OUT
                content.contains("OUT") => Output::OUT { content_raw: content, narsese: None },
                // 其它情况
                _ => Output::OTHER { content },
            };
            // 返回
            Ok(output)
        }

        // 构造并启动虚拟机
        let vm = CommandVm::from_io_process(command_java.into())
            // 输入转译器
            .input_translator(input_translate)
            // 输出转译器
            .output_translator(output_translate)
            // 🔥启动
            .launch();
        _test_opennars(vm);
    }

    /// 通用测试/OpenNARS
    pub fn _test_opennars(mut vm: CommandVmRuntime) {
        // 专有闭包 | ⚠️无法再提取出另一个闭包：重复借用问题
        let mut input_cmd_and_await =
            |cmd, contains| input_cmd_and_await_contains(&mut vm, cmd, contains);
        input_cmd_and_await(Cmd::VOL(0), "");
        input_cmd_and_await(Cmd::NSE(nse_task!(<A --> B>.)), "<A --> B>.");
        input_cmd_and_await(Cmd::NSE(nse_task!(<B --> C>.)), "<B --> C>.");
        input_cmd_and_await(Cmd::NSE(nse_task!(<A --> C>?)), "<A --> C>?");
        input_cmd_and_await(Cmd::CYC(5), ""); // * CYC无需自动等待

        // 等待回答（字符串）
        await_fetch_until(&mut vm, |_, s| {
            s.contains("Answer") && s.contains("<A --> C>.")
        });

        // 终止虚拟机
        vm.terminate().expect("无法终止虚拟机");
        println!("Virtual machine terminated...");
    }

    /// 示例测试 | PyNARS
    /// * 🚩通过预置的批处理文件启动
    #[test]
    fn test_pynars() {
        let vm = CommandVm::new(EXE_PATH_PYNARS)
            // 输入转译器：直接取其尾部
            .input_translator(|cmd| Ok(cmd.tail()))
            // 🔥启动
            .launch();
        // 可复用的测试逻辑
        _test_pynars(vm);
    }

    /// 通用测试/ONA
    pub fn _test_ona(mut vm: CommandVmRuntime) {
        // 专有闭包 | ⚠️无法再提取出另一个闭包：重复借用问题
        let mut input_cmd_and_await =
            |cmd, contains| input_cmd_and_await_contains(&mut vm, cmd, contains);
        // input_cmd_and_await(Cmd::VOL(0), "");
        input_cmd_and_await(Cmd::NSE(nse_task!(<A --> B>.)), "<A --> B>.");
        input_cmd_and_await(Cmd::NSE(nse_task!(<B --> C>.)), "<B --> C>.");
        input_cmd_and_await(Cmd::NSE(nse_task!(<A --> C>?)), "<A --> C>?");
        input_cmd_and_await(Cmd::CYC(5), ""); // * CYC无需自动等待

        // 等待回答（字符串）
        await_fetch_until(&mut vm, |o, raw_content| {
            matches!(o, Output::ANSWER { .. }) && raw_content.contains("<A --> C>.")
        });

        // 终止虚拟机
        vm.terminate().expect("无法终止虚拟机");
        println!("Virtual machine terminated...");
    }

    /// 通用测试/PyNARS
    pub fn _test_pynars(mut vm: CommandVmRuntime) {
        // // 睡眠等待
        // // std::thread::sleep(std::time::Duration::from_secs(1));
        // ! ↑现在无需睡眠等待：输入会自动在初始化后写入子进程

        // 专有闭包
        let mut input_cmd_and_await =
            |cmd, contains| input_cmd_and_await_contains(&mut vm, cmd, contains);

        // 构造并输入任务 | 输入进PyNARS后变成了紧凑版本
        input_cmd_and_await(Cmd::NSE(nse_task!(<A --> B>.)), "<A-->B>.");
        input_cmd_and_await(Cmd::NSE(nse_task!(<B --> C>.)), "<B-->C>.");
        input_cmd_and_await(Cmd::NSE(nse_task!(<A --> C>?)), "<A-->C>?");
        input_cmd_and_await(Cmd::CYC(5), ""); // * CYC无需自动等待

        // 等待回答
        await_fetch_until(&mut vm, |_, s| {
            s.contains("ANSWER") && s.contains("<A-->C>.")
        });

        // 打印所有输出
        while let Some(output) = vm.try_fetch_output().unwrap() {
            println!("{:?}", output);
        }

        // 终止虚拟机
        vm.terminate().expect("无法终止虚拟机");
        println!("Virtual machine terminated...");
        // * 📝在实际测试中会使Python报错「EOFError: EOF when reading a line」
        /* // * ✅但这不影响（不会被「命令行虚拟机」捕获为输出）
        traceback (most recent call last):
        File "<frozen runpy>", line 198, in _run_module_as_main
        File "<frozen runpy>", line 88, in _run_code
        */
    }
}
