//! 管理「虚拟机交互」中「置入NAL」的部分

use crate::test_tools::{NALInput, OutputExpectation, OutputExpectationError};
use anyhow::Result;
use nar_dev_utils::if_return;
use narsese::api::FloatPrecision;
use navm::{cmd::Cmd, output::Output, vm::VmRuntime};
use std::{ops::ControlFlow, path::Path, time::Duration};

/// 输出缓存
/// * 🎯为「使用『推送』功能，而不引入具体数据类型」设置
/// * 📌基础功能：推送输出、遍历输出
pub trait VmOutputCache {
    /// 存入输出
    /// * 🎯统一的「打印输出」逻辑
    fn put(&mut self, output: Output) -> Result<()>;

    /// 遍历输出
    /// * 🚩不是返回迭代器，而是用闭包开始计算
    /// * 📝使用最新的「控制流」数据结构
    ///   * 使用[`None`]代表「一路下来没`break`」
    fn for_each<T>(&self, f: impl FnMut(&Output) -> ControlFlow<T>) -> Result<Option<T>>;
}

/// 向虚拟机置入[`NALInput`]
/// * 🎯除了「输入指令」之外，还附带其它逻辑
/// * 🚩通过「输出缓存」参数，解决「缓存输出」问题
/// * ❓需要迁移「符合预期」的逻辑
pub fn put_nal(
    vm: &mut impl VmRuntime,
    input: NALInput,
    output_cache: &mut impl VmOutputCache,
    // 不能传入「启动配置」，就要传入「是否启用用户输入」状态变量
    enabled_user_input: bool,
    nal_root_path: &Path,
    precision_epoch: FloatPrecision,
) -> Result<()> {
    use NALInput::*;
    match input {
        // 置入NAVM指令
        Put(cmd) => vm.input_cmd(cmd),
        // 睡眠
        Sleep(duration) => nal_sleep(duration),
        // 等待一个符合预期的NAVM输出
        Await(expectation) => nal_await(vm, output_cache, expectation, precision_epoch),
        // 检查是否有NAVM输出符合预期
        ExpectContains(expectation) => {
            nal_expect_contains(vm, output_cache, expectation, precision_epoch)
        }
        // 检查在指定的「最大步数」内，是否有NAVM输出符合预期（弹性步数`0~最大步数`）
        ExpectCycle(max_cycles, step_cycles, step_duration, expectation) => nal_expect_cycle(
            max_cycles,
            vm,
            step_cycles,
            step_duration,
            output_cache,
            expectation,
            precision_epoch,
        ),
        // 保存（所有）输出
        // * 🚩输出到一个文本文件中
        // * ✨复合JSON「对象数组」格式
        SaveOutputs(path_str) => nal_save_outputs(output_cache, nal_root_path, path_str),
        // 终止虚拟机
        Terminate {
            if_not_user,
            result,
        } => nal_terminate(if_not_user, enabled_user_input, vm, result),
    }
}

fn nal_sleep(duration: Duration) -> Result<()> {
    // 睡眠指定时间
    std::thread::sleep(duration);
    // 返回`ok`
    Ok(())
}

fn nal_await(
    vm: &mut impl VmRuntime,
    output_cache: &mut impl VmOutputCache,
    expectation: OutputExpectation,
    precision_epoch: f64,
) -> Result<()> {
    loop {
        let output = match vm.fetch_output() {
            Ok(output) => {
                // 加入缓存
                output_cache.put(output.clone())?;
                // ! ❌【2024-04-03 01:19:06】无法再返回引用：不再能直接操作数组，MutexGuard也不允许返回引用
                // output_cache.last().unwrap()
                output
            }
            Err(e) => {
                println!("尝试拉取输出出错：{e}");
                continue;
            }
        };
        // 只有匹配了才返回
        if expectation.matches(&output, precision_epoch) {
            break Ok(());
        }
    }
}

fn nal_expect_contains(
    vm: &mut impl VmRuntime,
    output_cache: &mut impl VmOutputCache,
    expectation: OutputExpectation,
    precision_epoch: f64,
) -> Result<()> {
    // 先尝试拉取所有输出到「输出缓存」
    while let Some(output) = vm.try_fetch_output()? {
        output_cache.put(output)?;
    }
    // 然后读取并匹配缓存
    let result =
        output_cache.for_each(
            |output| match expectation.matches(output, precision_epoch) {
                true => ControlFlow::Break(true),
                false => ControlFlow::Continue(()),
            },
        )?;
    // for output in output_cache.for_each() {
    //     // 只有匹配了才返回Ok
    //     if expectation.matches(output) {
    //     }
    // }
    match result {
        // 只有匹配到了一个，才返回Ok
        Some(true) => Ok(()),
        // 否则返回Err
        _ => Err(OutputExpectationError::ExpectedNotExists(expectation).into()),
    }
}

fn nal_expect_cycle(
    max_cycles: usize,
    vm: &mut impl VmRuntime,
    step_cycles: usize,
    step_duration: Option<Duration>,
    output_cache: &mut impl VmOutputCache,
    expectation: OutputExpectation,
    precision_epoch: f64,
) -> Result<()> {
    let mut cycles = 0;
    while cycles < max_cycles {
        // 推理步进
        vm.input_cmd(Cmd::CYC(step_cycles))?;
        cycles += step_cycles;
        // 等待指定时长
        if let Some(duration) = step_duration {
            std::thread::sleep(duration);
        }
        // 先尝试拉取所有输出到「输出缓存」
        while let Some(output) = vm.try_fetch_output()? {
            output_cache.put(output)?;
        }
        // 然后读取并匹配缓存
        let result =
            output_cache.for_each(
                |output| match expectation.matches(output, precision_epoch) {
                    true => ControlFlow::Break(true),
                    false => ControlFlow::Continue(()),
                },
            )?;
        // 匹配到一个⇒提前返回Ok
        if let Some(true) = result {
            // * 🚩【2024-09-12 17:54:50】目前逻辑从「直接打印到终端」改为「向输出缓存打印输出（以便外部识别）」
            // * 📌【2024-09-13 00:46:16】已在 BabelNAR-CLI.rs 中通过「本地路径crate替换」验证可以
            let message = format!("expect-cycle({cycles}): {expectation}");
            let output = Output::INFO { message };
            output_cache.put(output)?;
            return Ok(());
        }
    }
    // 步进完所有步数，仍未有匹配⇒返回Err
    Err(OutputExpectationError::ExpectedNotExists(expectation).into())
}

fn nal_save_outputs(
    output_cache: &mut impl VmOutputCache,
    nal_root_path: &Path,
    path_str: String,
) -> Result<()> {
    let file_str = collect_outputs_to_json(output_cache)?;
    // 保存到文件中 | 使用基于`nal_root_path`的相对路径
    let path = nal_root_path.join(path_str.trim());
    std::fs::write(path, file_str)?;
    // 提示 | ❌【2024-04-09 22:22:04】执行「NAL输入」时，应始终静默
    // println_cli!([Info] "已将所有NAVM输出保存到文件{path:?}");
    // 返回
    Ok(())
}

fn collect_outputs_to_json(output_cache: &mut impl VmOutputCache) -> Result<String> {
    let mut file_str = "[".to_string();
    output_cache.for_each(|output| {
        // 换行制表
        file_str += "\n\t";
        // 统一追加到字符串中
        file_str += &output.to_json_string();
        // 逗号
        file_str.push(',');
        // 继续
        ControlFlow::<()>::Continue(())
    })?;
    file_str.pop();
    file_str += "\n]";
    Ok(file_str)
}

fn nal_terminate(
    if_not_user: bool,
    enabled_user_input: bool,
    vm: &mut impl VmRuntime,
    result: std::result::Result<(), String>,
) -> Result<()> {
    // 检查前提条件 | 仅「非用户输入」&启用了用户输入 ⇒ 放弃终止
    if_return! { if_not_user && enabled_user_input => Ok(()) }
    // 终止虚拟机
    vm.terminate()?;
    // 返回
    result.map_err(|e| anyhow::anyhow!("{e}"))
}
