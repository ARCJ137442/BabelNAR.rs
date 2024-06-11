//! * 🎯统一存放与「Narsese预期识别」有关的代码
//! * 🚩【2024-04-02 22:49:12】从[`crate::runtimes::command_vm::runtime::tests`]中迁移而来

use nar_dev_utils::if_return;
use narsese::{
    api::{GetBudget, GetPunctuation, GetStamp, GetTruth, NarseseValue},
    conversion::{
        inter_type::lexical_fold::TryFoldInto,
        string::impl_enum::format_instances::FORMAT_ASCII as FORMAT_ASCII_ENUM,
    },
    enum_narsese::{
        Budget as EnumBudget, Narsese as EnumNarsese, Sentence as EnumSentence, Task as EnumTask,
        Truth as EnumTruth,
    },
    lexical::{Narsese, Sentence as LexicalSentence, Task as LexicalTask, Term},
};
use navm::output::Operation;

use super::term_equal::*;

/// 判断「输出是否（在Narsese语义层面）符合预期」
/// * 🎯词法Narsese⇒枚举Narsese，以便从语义上判断
pub fn is_expected_narsese_lexical(expected: &Narsese, out: &Narsese) -> bool {
    _is_expected_narsese(expected.clone(), out.clone())
}

fn _is_expected_narsese(mut expected: Narsese, mut out: Narsese) -> bool {
    // 先比对词项
    fn get_term_mut(narsese: &mut Narsese) -> &mut Term {
        use NarseseValue::*;
        match narsese {
            Term(term)
            | Sentence(LexicalSentence { term, .. })
            | Task(LexicalTask {
                sentence: LexicalSentence { term, .. },
                ..
            }) => term,
        }
    }
    // * 🚩特制的「词项判等」截断性逻辑 | 🚩语义层面判等词项
    if_return! {
        !semantical_equal_mut(get_term_mut(&mut expected), get_term_mut(&mut out)) => false
    };
    // 临时折叠预期
    let expected =
        (expected.try_fold_into(&FORMAT_ASCII_ENUM)).expect("作为预期的词法Narsese无法折叠！");
    // 与预期一致
    let out = out.try_fold_into(&FORMAT_ASCII_ENUM); // 必须复制：折叠消耗自身
    match out {
        Ok(out) => is_expected_enum_residual(&expected, &out),
        Err(..) => false,
    }
}

/// 判断「输出是否（在Narsese层面）符合预期」
/// * 🎯预期词项⇒只比较词项，语句⇒只比较语句，……
/// * 🚩【2024-06-11 16:02:10】目前对「词项比对」使用特殊逻辑，而对其它结构照常比较
///   * ❓TODO: 【2024-06-11 21:22:15】是否需要避免重复折叠
fn is_expected_enum_residual(expected: &EnumNarsese, out: &EnumNarsese) -> bool {
    use NarseseValue::*;
    match ((expected), (out)) {
        // 词项⇒只比较词项
        // ! 🚩【2024-06-11 16:05:45】现在直接在词法层面判等，能运行至此都是已经词项相等的（枚举Narsese的集合相对难以统一）
        (Term(_term), ..) => true, /* is_expected_term(term, out.get_term()) */
        // 语句⇒只比较语句
        // ! 仍然不能直接判等：真值/预算值
        (Sentence(s_exp), Sentence(s_out) | Task(EnumTask(s_out, ..))) => {
            is_expected_sentence(s_exp, s_out)
        }
        // 任务⇒直接判断
        // ! 仍然不能直接判等：真值/预算值
        (Task(t_exp), Task(t_out)) => is_expected_task(t_exp, t_out),
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
    // 词项 | ✅已经在词法层面判等
    // (is_expected_term(expected.get_term(),out.get_term())) &&
    // 标点相等
    expected.get_punctuation() == out.get_punctuation()
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

// ! 🚩【2024-06-11 16:03:50】现在直接在词法层面判等Narsese词项
// /// 判断输出的词项是否与预期词项相同
// /// * 🎯用于独立出「词项预期」功能
// /// * 🚩【2024-04-02 22:55:13】目前直接判等
// pub fn is_expected_term(expected: &EnumTerm, out: &EnumTerm) -> bool {
//     // expected == out
// }

/// 判断「输出是否在真值层面符合预期」
/// * 🎯空真值的语句，应该符合「固定真值的语句」的预期——相当于「通配符」
pub fn is_expected_truth(expected: &EnumTruth, out: &EnumTruth) -> bool {
    match (expected, out) {
        // 预期空真值⇒通配
        (EnumTruth::Empty, ..) => true,
        // 预期单真值
        (EnumTruth::Single(f_e), EnumTruth::Single(f_o) | EnumTruth::Double(f_o, ..)) => f_e == f_o,
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
