//! 与NAVM虚拟机的交互逻辑

use std::ops::ControlFlow;

use crate::cli_support::error_handling_boost::error_anyhow;

use super::{NALInput, OutputExpectation, OutputExpectationError};
use anyhow::Result;
use nar_dev_utils::{if_return, ResultBoost};
use navm::{output::Output, vm::VmRuntime};

/// * 🎯统一存放与「Narsese预期识别」有关的代码
/// * 🚩【2024-04-02 22:49:12】从[`crate::runtimes::command_vm::runtime::tests`]中迁移而来
mod narsese_expectation {
    use nar_dev_utils::if_return;
    use narsese::{
        api::{GetBudget, GetPunctuation, GetStamp, GetTerm, GetTruth},
        conversion::{
            inter_type::lexical_fold::TryFoldInto,
            string::impl_enum::format_instances::FORMAT_ASCII as FORMAT_ASCII_ENUM,
        },
        enum_narsese::{
            Budget as EnumBudget, Narsese as EnumNarsese, Sentence as EnumSentence,
            Task as EnumTask, Term as EnumTerm, Truth as EnumTruth,
        },
        lexical::Narsese,
    };
    use navm::output::Operation;

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
            (EnumNarsese::Term(term), ..) => is_expected_term(term, out.get_term()),
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
        // 词项
        (is_expected_term(expected.get_term(),out.get_term()))
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

    /// 判断输出的词项是否与预期词项相同
    /// * 🎯用于独立出「词项预期」功能
    /// * 🚩【2024-04-02 22:55:13】目前直接判等
    pub fn is_expected_term(expected: &EnumTerm, out: &EnumTerm) -> bool {
        expected == out
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

    /// 判断「输出是否在操作层面符合预期」
    /// * 🎯仅有「操作符」的「NARS操作」应该能通配所有「NARS操作」
    pub fn is_expected_operation(expected: &Operation, out: &Operation) -> bool {
        // 操作符名不同⇒直接pass
        if_return! { expected.operator_name != out.operator_name => false }

        // 比对操作参数：先判空
        match (expected.no_params(), out.no_params()) {
            // 预期无⇒通配
            (true, ..) => true,
            // 预期有，输出无⇒直接pass
            (false, true) => false,
            // 预期有，输出有⇒判断参数是否相同
            (false, false) => expected.params == out.params,
        }
    }
}
pub use narsese_expectation::*;

/// 实现/预期匹配功能
impl OutputExpectation {
    /// 判断一个「NAVM输出」是否与自身相符合
    pub fn matches(&self, output: &Output) -> bool {
        // 输出类型
        if let Some(expected) = &self.output_type {
            if_return! { expected != output.type_name() => false }
        }

        // Narsese
        match (&self.narsese, output.get_narsese()) {
            // 预期有，输出无⇒直接pass
            (Some(..), None) => return false,
            // 预期输出都有⇒判断Narsese是否相同
            (Some(expected), Some(out)) => {
                if_return! { !is_expected_narsese_lexical(expected, out) => false }
            }
            _ => (),
        }

        // 操作 | 最后返回
        match (&self.operation, output.get_operation()) {
            // 预期无⇒通配
            (None, ..) => true,
            // 预期有，输出无⇒直接pass
            (Some(_), None) => false,
            // 预期有，输出有⇒判断操作是否相同
            (Some(expected), Some(out)) => is_expected_operation(expected, out),
        }
    }
}

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
    fn for_each<T>(&self, f: impl Fn(&Output) -> ControlFlow<T>) -> Result<Option<T>>;
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
            if expectation.matches(&output) {
                break Ok(());
            }
        },
        // 检查是否有NAVM输出符合预期
        NALInput::ExpectContains(expectation) => {
            // 先尝试拉取所有输出到「输出缓存」
            while let Some(output) = vm.try_fetch_output()? {
                output_cache.put(output)?;
            }
            // 然后读取并匹配缓存
            let result = output_cache.for_each(|output| match expectation.matches(output) {
                true => ControlFlow::Break(true),
                false => ControlFlow::Continue(()),
            })?;
            match result {
                // 只有匹配到了一个，才返回Ok
                Some(true) => Ok(()),
                // 否则返回Err
                _ => Err(OutputExpectationError::ExpectedNotExists(expectation).into()),
            }
            // for output in output_cache.for_each() {
            //     // 只有匹配了才返回Ok
            //     if expectation.matches(output) {
            //     }
            // }
        }
        // 终止虚拟机
        NALInput::Terminate {
            if_not_user,
            result,
        } => {
            // 检查前提条件 | 仅「非用户输入」&启用了用户输入 ⇒ 放弃终止
            if_return! { if_not_user && enabled_user_input => Ok(()) }

            // 终止虚拟机
            vm.terminate()?;

            // 返回
            result.transform_err(error_anyhow)
        }
    }
}
