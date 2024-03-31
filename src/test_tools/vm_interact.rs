//! 与NAVM虚拟机的交互逻辑

use super::{NALInput, OutputExpectation};
use anyhow::Result;
use navm::{output::Output, vm::VmRuntime};

impl OutputExpectation {
    /// 判断一个「NAVM输出」是否与自身相符合
    /// * 🏗️TODO: 迁移功能
    pub fn matches(&self, output: &Output) -> bool {
        todo!()
    }
}

/// 向虚拟机置入[`NALInput`]
/// * 🎯除了「输入指令」之外，还附带其它逻辑
/// * 🚩通过「输出缓存」参数，解决「缓存输出」问题
/// * ❓需要迁移「符合预期」的逻辑
pub fn put_nal(
    mut vm: impl VmRuntime,
    input: NALInput,
    output_cache: &mut Vec<Output>,
) -> Result<()> {
    match input {
        // 置入NAVM指令
        NALInput::Put(cmd) => vm.input_cmd(cmd),
        // 睡眠
        NALInput::Sleep(duration) => {
            // 睡眠指定时间
            std::thread::sleep(duration);
            // 返回`ok`
            Ok(())
        }
        // 等待一个符合预期的NAVM输出
        NALInput::Await(expectation) => loop {
            let output = match vm.fetch_output() {
                Ok(output) => {
                    // 加入缓存
                    output_cache.push(output);
                    // 返回引用
                    output_cache.last().unwrap()
                }
                Err(e) => {
                    println!("尝试拉取输出出错：{e}");
                    continue;
                }
            };
            // 只有匹配了才返回
            if expectation.matches(output) {
                break Ok(());
            }
        },
        // 检查是否有NAVM输出符合预期
        NALInput::ExpectContains(expectation) => {
            // 先尝试拉取所有输出到「输出缓存」
            while let Ok(Some(output)) = vm.try_fetch_output() {
                output_cache.push(output);
            }
            // 然后逐个读取输出缓存
            for output in output_cache.iter() {
                // 只有匹配了才返回Ok
                if expectation.matches(output) {
                    return Ok(());
                }
            }
            // 否则返回Err
            Err(anyhow::anyhow!("没有找到符合要求「{expectation:?}」的输出"))
        }
    }
}
