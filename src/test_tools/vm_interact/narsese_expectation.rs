//! * 🎯统一存放与「Narsese预期识别」有关的代码
//! * 🚩【2024-04-02 22:49:12】从[`crate::runtimes::command_vm::runtime::tests`]中迁移而来

use super::term_equal::*;
use anyhow::Result;
use nar_dev_utils::if_return;
use nar_dev_utils::macro_once;
use narsese::{
    api::{FloatPrecision, NarseseValue},
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

/// 判断「输出是否（在Narsese语义层面）符合预期」
/// * 🎯词法Narsese⇒枚举Narsese，以便从语义上判断
pub fn is_expected_narsese_lexical(
    expected: &Narsese,
    out: &Narsese,
    precision_epoch: FloatPrecision,
) -> bool {
    _is_expected_narsese(expected.clone(), out.clone(), precision_epoch)
}

fn _is_expected_narsese(
    mut expected: Narsese,
    mut out: Narsese,
    precision_epoch: FloatPrecision,
) -> bool {
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
        !semantical_equal_mut(get_term_mut(&mut expected), get_term_mut(&mut out))
        => false
    };
    // * 🚩折叠剩余部分，并开始判断
    let fold = PartialFoldResult::try_from;
    match (fold(expected), fold(out)) {
        // * 🚩若均解析成功⇒进一步判等
        (Ok(expected), Ok(out)) => expected.is_expected_out(&out, precision_epoch),
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
    fn is_expected_out(&self, out: &Self, precision_epoch: FloatPrecision) -> bool {
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
                // 🚩空值通配
                // * 🎯用于在「真值为空」「预算值为空」时通配
                // * 📌【2024-06-16 16:58:53】「任务」应该与「空预算的语句」通配
                (@SINGLE @EMPTY_WILDCARD $exp_i:ident @ $exp:expr, $out_i:ident @ $out:expr => $($code:tt)*) => {
                    match ($exp.as_ref(), $out.as_ref()) {
                        // * 🚩预期、输出 都有
                        (Some($exp_i), Some($out_i)) => {
                            $($code)*
                        },
                        // * 🚩没预期 ⇒ 通配
                        (None, _) => true,
                        // * 🚩其它⇒否
                        _ => false,
                    }
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
                @EMPTY_WILDCARD // ! 空值通配
                // 真值一致
                expected @ self.truth,
                out @ out.truth =>
                is_expected_truth(expected, out, precision_epoch) // * 🚩特殊情况（需兼容）特殊处理
            } && {
                @EMPTY_WILDCARD // ! 空值通配
                // 预算值一致
                expected @ self.budget,
                out @ out.budget =>
                is_expected_budget(expected, out, precision_epoch) // * 🚩特殊情况（需兼容）特殊处理
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

/// 判断「短浮点之间是否相等」（在指定精度范围内）
/// * 🎯应对不同小数精度的NARS输出，统一在某精度内相等
/// * 🚩【2024-08-01 10:36:31】需要引入配置
/// * 📝|expected - out| ≤ precision_epoch
fn is_expected_float(
    expected: &FloatPrecision,
    out: &FloatPrecision,
    precision_epoch: FloatPrecision,
) -> bool {
    // * 🚩精度=0 ⇒ 直接判等
    if precision_epoch == 0.0 {
        return expected == out;
    }
    // * 🚩其它 ⇒ 绝对值小于等于
    (expected - out).abs() <= precision_epoch
}

/// 判断「输出是否在真值层面符合预期」
/// * 🎯空真值的语句，应该符合「固定真值的语句」的预期——相当于「通配符」
#[inline]
fn is_expected_truth(
    expected: &EnumTruth,
    out: &EnumTruth,
    precision_epoch: FloatPrecision,
) -> bool {
    use EnumTruth::*;
    match [expected, out] {
        // 预期空真值⇒通配
        [Empty, ..] => true,
        // 预期单真值⇒部分通配
        [Single(f_e), Single(f_o) | Double(f_o, ..)] => {
            is_expected_float(f_e, f_o, precision_epoch)
        }
        // 预期双真值
        [Double(f_e, c_e), Double(f_o, c_o)] => {
            is_expected_float(f_e, f_o, precision_epoch)
                && is_expected_float(c_e, c_o, precision_epoch)
        }
        // 其它情况
        _ => false,
    }
}

/// 判断「输出是否在预算值层面符合预期」
/// * 🎯空预算的语句，应该符合「固定预算值的语句」的预期——相当于「通配符」
#[inline]
fn is_expected_budget(
    expected: &EnumBudget,
    out: &EnumBudget,
    precision_epoch: FloatPrecision,
) -> bool {
    use EnumBudget::*;
    match [expected, out] {
        // 预期空预算⇒通配
        [Empty, ..] => true,
        // 预期单预算
        [Single(p_e), Single(p_o) | Double(p_o, ..) | Triple(p_o, ..)] => {
            is_expected_float(p_e, p_o, precision_epoch)
        }
        // 预期双预算
        [Double(p_e, d_e), Double(p_o, d_o) | Triple(p_o, d_o, ..)] => {
            is_expected_float(p_e, p_o, precision_epoch)
                && is_expected_float(d_e, d_o, precision_epoch)
        }
        // 预期三预算
        [Triple(p_e, d_e, q_e), Triple(p_o, d_o, q_o)] => {
            is_expected_float(p_e, p_o, precision_epoch)
                && is_expected_float(d_e, d_o, precision_epoch)
                && is_expected_float(q_e, q_o, precision_epoch)
        }
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
    match [expected.no_params(), out.no_params()] {
        // 预期无⇒通配
        [true, ..] => true,
        // 预期有，输出无⇒直接pass
        [false, true] => false,
        // 预期有，输出有⇒判断参数是否相同
        [false, false] => expected.params == out.params,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use narsese::lexical_nse as nse;
    use navm::operation;

    #[test]
    fn is_expected_narsese_lexical() {
        /// 正例断言带精度
        fn test(expected: Narsese, out: Narsese, precision_epoch: FloatPrecision) {
            assert!(
                super::is_expected_narsese_lexical(&expected, &out, precision_epoch),
                "正例断言失败！\nexpected: {expected:?}\nout: {out:?}\nprecision_epoch: {precision_epoch:?}"
            );
        }
        /// 反例断言带精度
        fn test_negative(expected: Narsese, out: Narsese, precision_epoch: FloatPrecision) {
            assert!(
                !super::is_expected_narsese_lexical(&expected, &out, precision_epoch),
                "反例断言失败！\nexpected: {expected:?}\nout: {out:?}\nprecision_epoch: {precision_epoch:?}"
            );
        }
        // * 🚩正例
        macro_once! {
            macro test {
                ( // 分派&展开
                    $($expected:literal $op:tt $config:tt => $out:literal $(,)?)*
                ) => {
                    $(
                        test!(@SPECIFIC $expected, $out, $op, $config);
                    )*
                }
                ( // * 📝正例语法："预期" ==(精度)=> "输出"
                    @SPECIFIC
                    $expected:literal,
                    $out:literal,
                    ==,
                    ($epoch:expr)
                ) => {
                    test(nse!($expected), nse!($out), $epoch)
                }
                ( // * 📝反例语法："预期" !=(精度)=> "输出"
                    @SPECIFIC
                    $expected:literal,
                    $out:literal,
                    !=,
                    ($epoch:expr)
                ) => {
                    test_negative(nse!($expected), nse!($out), $epoch)
                }
            }
            // * 🚩正例
            // 常规词项、语句、任务
            "A"  ==(0.0)=> "A",
            "A"  !=(0.0)=> "B",
            "A." ==(0.0)=> "A.",
            "A." !=(0.0)=> "A?",
            "A?" ==(0.0)=> "A?",
            "A?" !=(0.0)=> "<A --> B>?",
            "A! %1.0;0.9%" ==(0.0)=> "A! %1.0;0.9%"
            "$0.5;0.5;0.5$ A@" ==(0.0)=> "$0.5;0.5;0.5$ A@",
            "$0.5;0.5;0.5$ A. %1.0;0.9%" ==(0.0)=> "$0.5;0.5;0.5$ A. %1.0;0.9%",
            // 真值通配（反向就不行）
            "A." ==(0.0)=> "A. %1.0;0.9%",
            "A!" ==(0.0)=> "A! %1.0;0.9%",
            "A. %1.0;0.9%" !=(0.0)=> "A.",
            "A! %1.0;0.9%" !=(0.0)=> "A!",
            // 预算值通配（反向就不行）
            "A." ==(0.0)=> "$0.5;0.5;0.5$ A.",
            "A!" ==(0.0)=> "$0.5;0.5;0.5$ A!",
            "A." ==(0.0)=> "$0.5;0.5;0.5$ A. %1.0;0.9%",
            "A!" ==(0.0)=> "$0.5;0.5;0.5$ A! %1.0;0.9%",
            "$0.5;0.5;0.5$ A."           !=(0.0)=> "A.",
            "$0.5;0.5;0.5$ A!"           !=(0.0)=> "A!",
            "$0.5;0.5;0.5$ A. %1.0;0.9%" !=(0.0)=> "A.",
            "$0.5;0.5;0.5$ A! %1.0;0.9%" !=(0.0)=> "A!",
            // 真值精度内匹配
            "A. %0.5;0.9%" ==(0.00)=> "A. %0.5;0.9%",
            "A. %0.5;0.9%" ==(0.10)=> "A. %0.55;0.95%", // +0.10
            "A. %0.5;0.9%" ==(0.10)=> "A. %0.45;0.85%", // -0.10
            "A. %0.5;0.9%" ==(0.10)=> "A. %0.55;0.85%", // ±0.10
            "A. %0.5;0.9%" !=(0.01)=> "A. %0.55;0.85%", // ±0.01
            "A. %0.5%" ==(0.1)=> "A. %0.55;0.85%", // +通配
            "A. %0.5%" !=(-0.1)=> "A. %0.5%", // 负数永不匹配
            "A. %0;1%" ==(FloatPrecision::INFINITY)=> "A. %1;0%", // 正无穷总是匹配
            "A. %0.5%" !=(FloatPrecision::NEG_INFINITY)=> "A. %0.5%", // 负无穷永不匹配
            // 预算值精度内匹配
            "$0.5;0.7;0.9$ A." ==(0.0)=> "$0.5;0.7;0.9$ A.",
            "$0.5;0.9$ A." ==(0.051)=> "$0.55;0.85$ A.", // ±0.05，通配，防止极限`0.050000000000000044`情形
            "$0.5;0.9$ A." !=(0.051)=> "$0.55;0.84$ A.", // ±0.05，通配，防止极限`0.050000000000000044`情形
            "$0.5;0.9$ A." !=(0.051)=> "$0.55;0.96$ A.", // ±0.05，通配，防止极限`0.050000000000000044`情形
            "$0.5;0.7;0.9$ A." ==(0.051)=> "$0.55;0.7058;0.85$ A.", // ±0.050，防止极限`0.050000000000000044`情形
            "$0.5;0.7;0.9$ A." !=(0.041)=> "$0.55;0.7058;0.85$ A.", // ±0.040，防止极限`0.050000000000000044`情形
            "$0.5;0.7;0.9$ A." ==(0.041)=> "$0.54;0.7058;0.86$ A.", // ±0.040，防止极限`0.050000000000000044`情形
            "$0.5;0.7;0.9$ A." !=(0.001)=> "$0.55;0.7058;0.85$ A.", // ±0.001，防止极限`0.050000000000000044`情形
            // 源自实际应用
                      "<(&&,<$1 --> lock>,<$2 --> key>) ==> <$1 --> (/,open,$2,_)>>. %1.00;0.45%"
            ==(0.0)=> "<(&&,<$1 --> key>,<$2 --> lock>) ==> <$2 --> (/,open,$1,_)>>. %1.00;0.45%"
                       "<animal --> robin>. %1.00;0.45%" // 四位⇒两位（位数不一，但值相同）
            ==(0.01)=> "$0.9944;0.7848;0.7238$ <animal --> robin>. %1.0000;0.4500%",
                       "<swimmer --> bird>. %1.00;0.47%" // 四位⇒两位（位数不一，精度不同）
            ==(0.01)=> "$0.8333;0.7200;0.7369$ <swimmer --> bird>. %1.0000;0.4737%",
        }
    }

    #[test]
    fn is_expected_operation() {
        // * 🚩正例
        macro_once! {
            macro test($(
                [$($t_expected:tt)*] => [$($t_out:tt)*]
            )*) {
                $(
                    let expected = operation!($($t_expected)*);
                    let out = operation!($($t_out)*);
                    assert!(
                        super::is_expected_operation(&expected, &out),
                        "正例断言失败！\nexpected: {expected:?}, out: {out:?}"
                    );
                )*
            }
            // * 🚩仅有操作名
            ["left"] => ["left"]
            // * 🚩带参数
            ["left" => "{SELF}"] => ["left" => "{SELF}"]
            ["left" => "{SELF}" "x"] => ["left" => "{SELF}" "x"]
        }
        // * 🚩反例
        macro_once! {
            macro test($(
                [$($t_expected:tt)*] != [$($t_out:tt)*]
            )*) {
                $(
                    let expected = operation!($($t_expected)*);
                    let out = operation!($($t_out)*);
                    assert!(
                        !super::is_expected_operation(&expected, &out),
                        "反例断言失败！\nexpected: {expected:?}, out: {out:?}"
                    );
                )*
            }
            // * 🚩操作名不同
            ["left"] != ["right"]
            ["left" => "{SELF}"] != ["right" => "{SELF}"]
            ["left" => "{SELF}" "x"] != ["right" => "{SELF}" "x"]
            // * 🚩参数数目不同
            ["left" => "{SELF}"] != ["left" => "{SELF}" "x"]
            // * 🚩参数不同
            ["left" => "{SELF}" "x"] != ["left" => "[good]" "x"]
            ["left" => "{SELF}" "x"] != ["left" => "{OTHER}" "x"]
            ["left" => "{SELF}" "x"] != ["left" => "{SELF}" "y"]
        }
    }
}
