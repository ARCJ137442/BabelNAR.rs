//! 命令行虚拟机 运行时
//! * ✨核心内容
//!   * ⇄ 基于「进程通信」的消息互转
//!     * 📌核心IO流程：
//!       1. NAVM指令[`Cmd`] >>> 进程输入 >>> 子进程
//!       2. 子进程 >>> 进程输出 >>> NAVM输出[`Output`]
//!     * 🚩实现方式：两处转译器

use super::{
    default_input_translator, default_output_translator, CommandVm, InputTranslator,
    OutputTranslator,
};
use crate::process_io::IoProcessManager;
use anyhow::{anyhow, Result};
use nar_dev_utils::if_return;
use navm::{
    cmd::Cmd,
    output::Output,
    vm::{VmLauncher, VmRuntime, VmStatus},
};

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

    /// 用于指示的「状态」变量
    status: VmStatus,
}

impl VmRuntime for CommandVmRuntime {
    fn input_cmd(&mut self, cmd: Cmd) -> Result<()> {
        // 尝试转译
        let input = (self.input_translator)(cmd)?;
        // 当输入非空时，置入转译结果
        // * 🚩【2024-04-03 02:20:48】目前用「空字串」作为「空输入」的情形
        // TODO: 后续或将让「转译器」返回`Option<String>`
        // 空⇒提前返回
        if_return! { input.is_empty() => Ok(()) }
        // 置入
        // * 🚩没有换行符
        // * 📌【2024-04-07 23:43:59】追踪「Websocket进程阻塞」漏洞：问题不在此，在`ws::Sender::send`处
        self.process.put_line(input)
    }

    fn fetch_output(&mut self) -> Result<Output> {
        let s = self.process.fetch_output()?;
        (self.output_translator)(s)
    }

    fn try_fetch_output(&mut self) -> Result<Option<Output>> {
        let s = self.process.try_fetch_output()?;
        // 匹配分支
        match s {
            // 有输出⇒尝试转译并返回
            Some(s) => Ok(Some({
                // 转译输出
                let output = (self.output_translator)(s)?;
                // * 当输出为「TERMINATED」时，将自身终止状态置为「TERMINATED」
                if let Output::TERMINATED { description } = &output {
                    // ! 🚩【2024-04-02 21:39:56】目前将所有「终止」视作「意外终止」⇒返回`Err`
                    self.status = VmStatus::Terminated(Err(anyhow!(description.clone())));
                }
                // 传出输出
                output
            })),
            // 没输出⇒没输出 | ⚠️注意：不能使用`map`，否则`?`穿透不出闭包
            None => Ok(None),
        }
    }

    fn status(&self) -> &VmStatus {
        &self.status
    }

    fn terminate(&mut self) -> Result<()> {
        // 杀死子进程
        self.process.kill()?;

        // （杀死后）设置状态
        // * 🚩【2024-04-02 21:42:30】目前直接覆盖状态
        self.status = VmStatus::Terminated(Ok(()));

        // 返回「终止完成」
        Ok(())
    }
}

/// 构建功能：启动命令行虚拟机
impl VmLauncher for CommandVm {
    type Runtime = CommandVmRuntime;
    fn launch(self) -> Result<CommandVmRuntime> {
        Ok(CommandVmRuntime {
            // 状态：正在运行
            status: VmStatus::Running,
            // 启动内部的「进程管理者」
            process: self.io_process.launch()?,
            // 输入转译器
            input_translator: self
                .input_translator
                // 解包or使用默认值
                // * 🚩【2024-04-04 02:02:53】似乎不应有如此默认行为：后续若配置载入失败，将难以识别问题
                .unwrap_or(default_input_translator()),
            // 输出转译器
            output_translator: self
                .output_translator
                // 解包or使用默认值
                // * 🚩【2024-04-04 02:02:53】似乎不应有如此默认行为：后续若配置载入失败，将难以识别问题
                .unwrap_or(default_output_translator()),
            // * 🚩【2024-03-24 02:06:59】目前到此为止：只需处理「转译」问题
        })
    }
}

/// 单元测试
/// * 🎯作为任何NAVM运行时的共用测试包
/// * 🚩【2024-03-29 23:23:12】进一步开放：仍然只限定在「测试」环境中使用
#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::{
        cin_implements::common::generate_command,
        runtimes::TranslateError,
        tests::cin_paths::{OPENNARS, PYNARS_MODULE, PYNARS_ROOT},
    };
    use nar_dev_utils::manipulate;
    use narsese::{
        api::{GetBudget, GetPunctuation, GetStamp, GetTerm, GetTruth},
        conversion::{
            inter_type::lexical_fold::TryFoldInto,
            string::{
                impl_enum::format_instances::FORMAT_ASCII as FORMAT_ASCII_ENUM,
                impl_lexical::{format_instances::FORMAT_ASCII, shortcuts::*},
            },
        },
        enum_narsese::{
            Budget as EnumBudget, Narsese as EnumNarsese, Sentence as EnumSentence,
            Task as EnumTask, Truth as EnumTruth,
        },
        lexical::Narsese,
    };
    use std::process::Command;
    use util::first;

    // ! 🚩【2024-04-07 12:09:44】现在路径统一迁移到`lib.rs`的`tests`模块下

    const COMMAND_JAVA: &str = "java";
    const COMMAND_ARGS_JAVA: [&str; 2] = ["-Xmx1024m", "-jar"];

    /// 实用测试工具/等待
    pub fn await_fetch_until(
        vm: &mut CommandVmRuntime,
        criterion: impl Fn(&Output, &str) -> bool,
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
                Output::ANSWER { .. } => println!("捕获到回答！内容：{output:?}"),
                // 特别显示「操作」
                Output::EXE { operation, .. } => {
                    println!(
                        "捕获到操作！操作名称：{:?}，内容：{:?}",
                        operation.operator_name,
                        operation
                            .params
                            .iter()
                            .map(|param| FORMAT_ASCII.format_term(param))
                            .collect::<Vec<_>>()
                    )
                }
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
        criterion: impl Fn(&Output, &str) -> bool,
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

    /// 实用测试工具/输入并等待「Narsese回显」
    /// * 🚩`input_cmd_and_await`的简单封装
    /// * ✅【2024-03-29 22:55:11】现在「输出转换」已经成熟（可以提取出Narsese）
    /// * 🚩通过「转换为『枚举Narsese』」以实现判等逻辑（主要为「语义相等」）
    #[inline(always)]
    pub fn input_cmd_and_await_narsese(
        vm: &mut CommandVmRuntime,
        cmd: Cmd,
        expected: Narsese,
    ) -> Output {
        // 预先构建预期
        let expected = expected
            .clone()
            .try_fold_into(&FORMAT_ASCII_ENUM)
            .expect("作为预期的词法Narsese无法折叠！");
        // 输入 & 等待
        input_cmd_and_await(vm, cmd, |out, _| {
            // 有Narsese
            out.get_narsese().is_some_and(|out| {
                // 且与预期一致
                out.clone() // 必须复制：折叠消耗自身
                    .try_fold_into(&FORMAT_ASCII_ENUM)
                    .is_ok_and(|out| is_expected_narsese(&expected, &out))
            })
        })
    }

    /// 判断「输出是否（在Narsese语义层面）符合预期」
    /// * 🎯词法Narsese⇒枚举Narsese，以便从语义上判断
    pub fn is_expected_narsese_lexical(expected: &Narsese, out: &Narsese) -> bool {
        // 临时折叠预期
        let expected = (expected.clone().try_fold_into(&FORMAT_ASCII_ENUM))
            .expect("作为预期的词法Narsese无法折叠！");
        // 与预期一致
        (out.clone() // 必须复制：折叠消耗自身
            .try_fold_into(&FORMAT_ASCII_ENUM))
        .is_ok_and(|out| is_expected_narsese(&expected, &out))
    }

    /// 判断「输出是否（在Narsese层面）符合预期」
    /// * 🎯预期词项⇒只比较词项，语句⇒只比较语句，……
    pub fn is_expected_narsese(expected: &EnumNarsese, out: &EnumNarsese) -> bool {
        match ((expected), (out)) {
            // 词项⇒只比较词项 | 直接判等
            (EnumNarsese::Term(term), ..) => term == out.get_term(),
            // 语句⇒只比较语句
            // ! 仍然不能直接判等：真值/预算值
            (
                EnumNarsese::Sentence(s_exp),
                EnumNarsese::Sentence(s_out) | EnumNarsese::Task(EnumTask(s_out, ..)),
            ) => is_expected_sentence(s_exp, s_out),
            // 任务⇒直接判断
            // ! 仍然不能直接判等：真值/预算值
            (EnumNarsese::Task(t_exp), EnumNarsese::Task(t_out)) => is_expected_task(t_exp, t_out),
            // 所有其它情况⇒都是假
            (..) => false,
        }
    }

    /// 判断输出的任务是否与预期任务相同
    /// * 🎯用于细粒度判断「预算值」「语句」的预期
    pub fn is_expected_task(expected: &EnumTask, out: &EnumTask) -> bool {
        // 预算
        is_expected_budget(expected.get_budget(), out.get_budget())
        // 语句
        && is_expected_sentence(expected.get_sentence(), out.get_sentence())
    }

    /// 判断输出的语句是否与预期语句相同
    /// * 🎯用于细粒度判断「真值」的预期
    pub fn is_expected_sentence(expected: &EnumSentence, out: &EnumSentence) -> bool {
        // 词项判等
        ((expected.get_term())==(out.get_term()))
        // 标点相等
        && expected.get_punctuation() == out.get_punctuation()
        // 时间戳相等
        && expected.get_stamp()== out.get_stamp()
        // 真值兼容 | 需要考虑「没有真值可判断」的情况
            && match (expected.get_truth(),out.get_truth()) {
                // 都有⇒判断「真值是否符合预期」
                (Some(t_e), Some(t_o)) => is_expected_truth(t_e, t_o),
                // 都没⇒肯定真
                (None, None) => true,
                // 有一个没有⇒肯定假
                _ => false,
            }
    }

    /// 判断「输出是否在真值层面符合预期」
    /// * 🎯空真值的语句，应该符合「固定真值的语句」的预期——相当于「通配符」
    pub fn is_expected_truth(expected: &EnumTruth, out: &EnumTruth) -> bool {
        match (expected, out) {
            // 预期空真值⇒通配
            (EnumTruth::Empty, ..) => true,
            // 预期单真值
            (EnumTruth::Single(f_e), EnumTruth::Single(f_o) | EnumTruth::Double(f_o, ..)) => {
                f_e == f_o
            }
            // 预期双真值
            (EnumTruth::Double(..), EnumTruth::Double(..)) => expected == out,
            // 其它情况
            _ => false,
        }
    }

    /// 判断「输出是否在预算值层面符合预期」
    /// * 🎯空预算的语句，应该符合「固定预算值的语句」的预期——相当于「通配符」
    pub fn is_expected_budget(expected: &EnumBudget, out: &EnumBudget) -> bool {
        match (expected, out) {
            // 预期空预算⇒通配
            (EnumBudget::Empty, ..) => true,
            // 预期单预算
            (
                EnumBudget::Single(p_e),
                EnumBudget::Single(p_o) | EnumBudget::Double(p_o, ..) | EnumBudget::Triple(p_o, ..),
            ) => p_e == p_o,
            // 预期双预算
            (
                EnumBudget::Double(p_e, d_e),
                EnumBudget::Double(p_o, d_o) | EnumBudget::Triple(p_o, d_o, ..),
            ) => p_e == p_o && d_e == d_o,
            // 预期三预算
            (EnumBudget::Triple(..), EnumBudget::Triple(..)) => expected == out,
            // 其它情况
            _ => false,
        }
    }

    /// 示例测试 | OpenNARS
    /// * 🚩通过Java命令启动
    #[test]
    #[ignore = "【2024-04-14 20:24:52】会导致残留子进程"]
    fn test_opennars() {
        // 构造指令
        let mut command_java = Command::new(COMMAND_JAVA);
        // * 📝这里的`args`、`arg都返回的可变借用。。
        command_java
            .args(COMMAND_ARGS_JAVA)
            .arg(OPENNARS)
            // OpenNARS的默认参数 | ["null", "null", "null", "null"]
            // * 🔗https://github.com/opennars/opennars/blob/master/src/main/java/org/opennars/main/Shell.java
            // * ✅fixed「额外参数」问题：之前「IO进程」的测试代码`.arg("shell")`没删干净
            // .args(["null", "null", "null", "null"])
            ;
        // dbg!(&command_java);

        /// 临时构建的「输入转换」函数
        /// * 🎯用于转换`VOL 0`⇒`*volume=0`，避免大量输出造成进程卡顿
        fn input_translate(cmd: Cmd) -> Result<String> {
            let content = match cmd {
                // 直接使用「末尾」，此时将自动格式化任务（可兼容「空预算」的形式）
                Cmd::NSE(..) => cmd.tail(),
                // CYC指令：运行指定周期数
                Cmd::CYC(n) => n.to_string(),
                // VOL指令：调整音量
                Cmd::VOL(n) => format!("*volume={n}"),
                // 其它类型
                _ => return Err(TranslateError::UnsupportedInput(cmd).into()),
            };
            // 转换
            Ok(content)
        }

        /// 临时构建的「输出转换」函数
        fn output_translate(content: String) -> Result<Output> {
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
        let vm = manipulate!(
            CommandVm::from(command_java)
            // 输入转译器
            => .input_translator(input_translate)
            // 输出转译器
            => .output_translator(output_translate)
        )
        // 🔥启动
        .launch()
        .expect("无法启动虚拟机");
        _test_opennars(vm);
    }

    /// 通用测试/OpenNARS
    pub fn _test_opennars(mut vm: CommandVmRuntime) {
        // 专有闭包 | ⚠️无法再提取出另一个闭包：重复借用问题
        let mut input_cmd_and_await =
            |cmd, contains| input_cmd_and_await_contains(&mut vm, cmd, contains);
        // ! ✅【2024-03-25 13:54:36】现在内置进OpenNARS启动器，不再需要执行此操作
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
    /// * 🚩通过Python命令从**内置文件**启动
    #[test]
    fn test_pynars() {
        let vm = manipulate!(
            CommandVm::from(generate_command("python", Some(PYNARS_ROOT), ["-m", PYNARS_MODULE]))
            // 输入转译器：直接取其尾部
            => .input_translator(|cmd| Ok(cmd.tail()))
            // 暂无输出转译器
            // => .output_translator(output_translate)
        )
        // 🔥启动
        .launch()
        .expect("无法启动虚拟机");
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

    /// 通用测试/简单回答 | 基于Narsese
    /// * 📌考察NARS最基础的「继承演绎推理」
    pub fn test_simple_answer(mut vm: CommandVmRuntime) {
        // 构造并输入任务 | 输入进PyNARS后变成了紧凑版本
        let _ = vm.input_cmd(Cmd::VOL(0)); // * 尝试静音
        input_cmd_and_await_narsese(&mut vm, Cmd::NSE(nse_task!(<A --> B>.)), nse!(<A --> B>.));
        input_cmd_and_await_narsese(&mut vm, Cmd::NSE(nse_task!(<B --> C>.)), nse!(<B --> C>.));
        input_cmd_and_await_narsese(&mut vm, Cmd::NSE(nse_task!(<A --> C>?)), nse!(<A --> C>?));
        vm.input_cmd(Cmd::CYC(5)).expect("无法输入CYC指令"); // * CYC无需自动等待

        // 等待回答
        let expected_answer = nse!(<A --> C>.);
        await_fetch_until(&mut vm, |output, _| match output {
            Output::ANSWER { narsese: out, .. } => {
                is_expected_narsese_lexical(
                    &expected_answer,
                    // ! 不允许回答内容为空 | 必须拷贝再比对
                    &out.clone().expect("预期的回答内容为空！"),
                )
            }
            _ => false,
        });

        // 打印所有输出
        while let Some(output) = vm.try_fetch_output().unwrap() {
            println!("{:?}", output);
        }

        // 终止虚拟机
        vm.terminate().expect("无法终止虚拟机");
        println!("Virtual machine terminated...");
    }
}
