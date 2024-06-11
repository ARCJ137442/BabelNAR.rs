//! * 🎯统一存放与「Narsese预期识别」有关的代码
//! * 🚩【2024-04-02 22:49:12】从[`crate::runtimes::command_vm::runtime::tests`]中迁移而来

use super::term_equal::*;
use anyhow::Result;
use nar_dev_utils::if_return;
use narsese::{
    api::NarseseValue,
    conversion::{
        inter_type::lexical_fold::TryFoldInto,
        string::impl_enum::{format_instances::FORMAT_ASCII as FORMAT_ASCII_ENUM, NarseseFormat},
    },
    enum_narsese::{
        Budget as EnumBudget, Punctuation as EnumPunctuation, Stamp as EnumStamp,
        Truth as EnumTruth,
    },
    lexical::{Narsese, Sentence as LexicalSentence, Task as LexicalTask, Term},
};
use navm::output::Operation;
use util::macro_once;

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
    // * 🚩折叠剩余部分，并开始判断
    let fold = PartialFoldResult::try_from;
    match (fold(expected), fold(out)) {
        // * 🚩若均解析成功⇒进一步判等
        (Ok(expected), Ok(out)) => out.is_expected_out(&expected),
        // * 🚩任一解析失败⇒直接失败
        _ => false,
    }
}

/// 临时的「部分折叠结果」
/// * 📌用于非词项判等
/// * 🎯性能提升：避免重复折叠词项
#[derive(Debug, Clone, Default)]
struct PartialFoldResult {
    truth: Option<EnumTruth>,
    stamp: Option<EnumStamp>,
    budget: Option<EnumBudget>,
    punctuation: Option<EnumPunctuation>,
}

/// ! 判等即「预期判断」
/// * 🎯判断「输出是否（在Narsese层面）符合预期」
/// * 🚩【2024-06-11 16:02:10】目前对「词项比对」使用特殊逻辑，而对其它结构照常比较
/// * ✅均已经考虑「没有值可判断」的情况
impl PartialFoldResult {
    fn is_expected_out(&self, out: &Self) -> bool {
        macro_once! {
            /// 一系列针对Option解包的条件判断：
            /// * 🚩均为Some⇒展开内部代码逻辑
            /// * 🚩均为None⇒直接返回true
            /// * 🚩其它情况⇒直接返回false
            macro both_and {
                ($( { $($code:tt)* } ) && *) => {
                    $(
                        both_and!(@SINGLE $($code)*)
                    )&&*
                };
                (@SINGLE $l_i:ident @ $l:expr, $r_i:ident @ $r:expr => $($code:tt)*) => {
                    match ($l.as_ref(), $r.as_ref()) {
                        (Some($l_i), Some($r_i)) => {
                            $($code)*
                        },
                        (None, None) => true,
                        _ => false,
                    }
                };
            }
            // * 🚩开始判等逻辑
            {
                // 标点一致
                expected @ self.punctuation,
                out @ out.punctuation =>
                expected == out // * 🚩简单枚举类型：直接判等
            } && {
                // 时间戳一致
                expected @ self.stamp,
                out @ out.stamp =>
                expected == out // * 🚩简单枚举类型：直接判等
            } && {
                // 真值一致
                expected @ self.truth,
                out @ out.truth =>
                is_expected_truth(expected, out) // * 🚩特殊情况（需兼容）特殊处理
            } && {
                // 预算值一致
                expected @ self.budget,
                out @ out.budget =>
                is_expected_budget(expected, out) // * 🚩特殊情况（需兼容）特殊处理
            }
        }
    }
}

impl TryFrom<Narsese> for PartialFoldResult {
    type Error = ();
    /// 从「词法Narsese」中折叠
    /// * 🚩折叠除词项以外的其它字段
    /// * 🚩【2024-06-12 01:54:13】转换失败⇒判等失败⇒返回false「不符预期」
    ///
    fn try_from(narsese: Narsese) -> Result<Self, Self::Error> {
        // * 🚩缩减代码长度的常量
        const FORMAT: &NarseseFormat<&str> = &FORMAT_ASCII_ENUM;
        /// * 🚩工具宏：封装「尝试做，不行就抛Err」的逻辑
        macro_rules! some_try {
            ($v:expr) => {
                Some(match $v {
                    Ok(v) => v,
                    Err(..) => return Err(()),
                })
            };
        }
        // * 🚩批量匹配折叠
        let value = match narsese {
            // * 🚩词项⇒全空
            NarseseValue::Term(..) => Self::default(),
            // * 🚩语句⇒真值、时间戳、标点
            NarseseValue::Sentence(LexicalSentence {
                punctuation,
                stamp,
                truth,
                ..
            }) => Self {
                truth: some_try!(truth.try_fold_into(FORMAT)),
                stamp: some_try!(FORMAT.parse(&stamp)),
                budget: None,
                punctuation: some_try!(FORMAT.parse(&punctuation)),
            },
            // * 🚩任务⇒语句+预算值
            NarseseValue::Task(LexicalTask {
                budget,
                sentence:
                    LexicalSentence {
                        punctuation,
                        stamp,
                        truth,
                        ..
                    },
            }) => Self {
                truth: some_try!(truth.try_fold_into(FORMAT)),
                stamp: some_try!(FORMAT.parse(&stamp)),
                budget: some_try!(budget.try_fold_into(FORMAT)),
                punctuation: some_try!(FORMAT.parse(&punctuation)),
            },
        };
        Ok(value)
    }
}

/// 判断「输出是否在真值层面符合预期」
/// * 🎯空真值的语句，应该符合「固定真值的语句」的预期——相当于「通配符」
#[inline]
fn is_expected_truth(expected: &EnumTruth, out: &EnumTruth) -> bool {
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
#[inline]
fn is_expected_budget(expected: &EnumBudget, out: &EnumBudget) -> bool {
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
