use narsese::{
    conversion::string::impl_enum::format_instances::FORMAT_ASCII as FORMAT_ASCII_ENUM, lexical::*,
};
use std::{cmp::Ordering, collections::HashMap};

/// 简单获取词项的「标识符」
/// * 🎯识别是否为「可交换词项」
/// * ⚠️对「集合词项」只取其中的左括弧
fn get_identifier(term: &Term) -> &str {
    match term {
        Atom { prefix, .. } => prefix,
        Compound { connecter, .. } => connecter,
        Set { left_bracket, .. } => left_bracket,
        Statement { copula, .. } => copula,
    }
}

// 重命名「变量词项」 //

/// 判断一个原子词项前缀是否为「变量词项」
fn is_variable_atom_prefix(prefix: &str) -> bool {
    // 独立变量
    prefix == FORMAT_ASCII_ENUM.atom.prefix_variable_independent
    // 非独变量
        || prefix == FORMAT_ASCII_ENUM.atom.prefix_variable_dependent
        // 查询变量
        || prefix == FORMAT_ASCII_ENUM.atom.prefix_variable_query
}

// /// 判断一个词项是否为变量
// fn is_variable_atom(term: &Term) -> bool {
//     match term {
//         Atom { prefix, .. } => is_variable_atom_prefix(prefix),
//         _ => false,
//     }
// }

// /// 重命名变量原子词项
// fn rename_variable_atom(term: &mut Term, new_name: &str) {
//     if let Atom { name, .. } = term {
//         *name = new_name.to_string()
//     }
// }

type VariableNameMap = HashMap<String, String>;

/// 重命名词项中的所有变量
/// * 🎯给所有词项统一编号
/// * 🚩返回：是否修改，变量映射表
#[allow(unused)]
fn rename_variables_in_term(term: &mut Term) -> (bool, VariableNameMap) {
    let mut map = VariableNameMap::new();
    rename_variables_in_term_with_map(term, &mut map);
    (!map.is_empty(), map)
}

/// 带映射地递归重命名变量
fn rename_variables_in_term_with_map(term: &mut Term, map: &mut VariableNameMap) -> bool {
    find_variables_renaming(term, map);
    let modified = map.iter().any(|(k, v)| k != v);
    if modified {
        apply_name_substitute(term, map);
    }
    modified
}

/// 寻找需要重命名的变量集合
fn find_variables_renaming(term: &Term, map: &mut VariableNameMap) {
    match term {
        // * 🚩原子变量词项⇒尝试命名
        Atom { prefix, name } if is_variable_atom_prefix(prefix) => {
            let new_name = match map.get(name) {
                Some(n) => n.clone(),
                None => (map.len() + 1).to_string(), // ? 避免已有的数字变量干扰 | 2 -> 1, 1 -> 2
            };
            // * 📌插入名称
            // if *name != new_name {
            map.insert(name.clone(), new_name.clone());
            // }
            // rename_variable_atom(term, &new_name);
        }
        // * 🚩复合词项⇒递归深入
        Compound { terms, .. } | Set { terms, .. } => terms
            .iter()
            .for_each(|term| find_variables_renaming(term, map)),
        Statement {
            subject, predicate, ..
        } => [subject, predicate]
            .into_iter()
            .for_each(|term| find_variables_renaming(term, map)),
        // * 🚩其它⇒未改变
        _ => (),
    }
}

fn apply_name_substitute(term: &mut Term, map: &VariableNameMap) {
    match term {
        Atom { name, .. } => {
            if let Some(new_name) = map.get(name) {
                *name = new_name.clone()
            }
        }
        Compound { terms, .. } | Set { terms, .. } => {
            for term in terms {
                apply_name_substitute(term, map)
            }
        }
        Statement {
            subject, predicate, ..
        } => {
            apply_name_substitute(subject, map);
            apply_name_substitute(predicate, map);
        }
    }
}

// 对「可交换词项」排序 //

/// 判断一个词项前缀是否为「可交换词项」
/// * 🚩一元词项不被视作【可交换的】词项：无需交换
fn is_communicative_term(identifier: &str) -> bool {
    // 外延集&内涵集
    identifier == FORMAT_ASCII_ENUM.compound.brackets_set_extension.0
        || identifier == FORMAT_ASCII_ENUM.compound.brackets_set_intension.0
        // 外延交&内涵交
        || identifier == FORMAT_ASCII_ENUM.compound.connecter_intersection_extension
        || identifier == FORMAT_ASCII_ENUM.compound.connecter_intersection_intension
        // 合取&析取
        || identifier == FORMAT_ASCII_ENUM.compound.connecter_conjunction
        || identifier == FORMAT_ASCII_ENUM.compound.connecter_disjunction
        // 平行合取
        || identifier == FORMAT_ASCII_ENUM.compound.connecter_conjunction_parallel
        // 相似&等价
        || identifier == FORMAT_ASCII_ENUM.statement.copula_similarity
        || identifier == FORMAT_ASCII_ENUM.statement.copula_equivalence
        // 并发性等价
        || identifier == FORMAT_ASCII_ENUM.statement.copula_equivalence_concurrent
}

/// 比较两个词项的顺序
/// * 🎯对「均为变量」的情况判断等值
fn term_comparator(term1: &Term, term2: &Term) -> Ordering {
    fn term_comparator_zipped((term1, term2): (&Term, &Term)) -> Ordering {
        term_comparator(term1, term2)
    }
    use Ordering::*;
    match (term1, term2) {
        // * 🚩原子🆚原子：判断变量情况
        (
            Atom {
                prefix: p1,
                name: n1,
            },
            Atom {
                prefix: p2,
                name: n2,
            },
        ) => match (is_variable_atom_prefix(p1), is_variable_atom_prefix(p2)) {
            // * 🚩都是变量⇒判等
            (true, true) => Equal,
            (false, true) => Less,
            (true, false) => Greater,
            // * 🚩其它情况⇒正常按名称判断
            (false, false) => p1.cmp(p2).then(n1.cmp(n2)),
        },
        // * 🚩复合🆚复合 | 集合🆚集合 ⇒ 深入
        (
            Compound {
                connecter: c1,
                terms: t1,
            },
            Compound {
                connecter: c2,
                terms: t2,
            },
        )
        | (
            Set {
                left_bracket: c1,
                terms: t1,
                ..
            },
            Set {
                left_bracket: c2,
                terms: t2,
                ..
            },
        ) => c1.cmp(c2).then(
            t1.iter()
                .zip(t2.iter())
                .map(term_comparator_zipped)
                .fold(Equal, Ordering::then),
        ),
        // * 🚩陈述🆚陈述
        (
            Statement {
                copula: c1,
                subject: s1,
                predicate: p1,
            },
            Statement {
                copula: c2,
                subject: s2,
                predicate: p2,
            },
        ) => c1.cmp(c2).then(
            ([s1, p1].into_iter())
                .zip([s2, p2])
                .map(|(inner1, inner2)| term_comparator(inner1, inner2))
                .fold(Equal, Ordering::then),
        ),
        // * 🚩其它类型不同的情况⇒明显的顺序（无需特别安排）
        _ => term1.cmp(term2),
    }
}

/// 对内部的「可交换词项」排序
/// * 🚩可交换词项⇒排序；不可交换⇒对其内子项排序
fn sort_communicative_terms(term: &mut Term) -> bool {
    // * 🚩尝试对内部词项排序
    let mut modified = match term {
        // * 🚩复合 & 集合
        Compound { terms, .. } | Set { terms, .. } => {
            let mut modified = false;
            for term in terms {
                // * 🚩必须全部执行排序，不能截断了事
                modified = sort_communicative_terms(term) || modified;
            }
            modified
        }
        // * 🚩陈述
        Statement {
            subject, predicate, ..
        } => {
            let modified_subject = sort_communicative_terms(subject);
            let modified_predicate = sort_communicative_terms(predicate);
            modified_subject || modified_predicate
        }
        // * 🚩其它⇒未改变
        _ => false,
    };
    // * 🚩可交换⇒自身直接元素排序
    if is_communicative_term(get_identifier(term)) {
        // * 🚩必须全部执行排序，不能截断了事
        modified = sort_a_communicative_term(term) || modified;
    }
    modified
}

/// 对一个「可交换词项」排序
/// * ⚠️只在当前层中排序
fn sort_a_communicative_term(term: &mut Term) -> bool {
    match term {
        // * 🚩复合 & 集合
        Compound { terms, .. } | Set { terms, .. } => {
            // * 🚩将引用按原先顺序排列
            let mut ref_terms: Vec<&Term> = terms.iter().collect();
            // * 🚩尝试排序词项引用（不改变原词项序列） | ⚠️若直接改变原序列，会存在借用问题
            ref_terms.sort_by(|&t1, &t2| term_comparator(t1, t2));
            // * 🚩迭代判断相等；不相等⇒被修改（此时已经被排序）
            let modified = terms.iter().zip(ref_terms).any(|(t1, t2)| t1 != t2);
            // * 🚩若有修改⇒再排序一次
            if modified {
                terms.sort_by(term_comparator);
            }
            modified
        }
        // * 🚩陈述
        Statement {
            subject, predicate, ..
        } => {
            match term_comparator(subject, predicate) {
                // subject ">" predicate
                Ordering::Greater => {
                    // * 🚩调整顺序
                    std::mem::swap(subject, predicate);
                    true // 被修改
                }
                _ => false,
            }
        }
        // * 🚩其它⇒不作为
        _ => false,
    }
}

// 对外接口 //

const MAX_TRIES_FORMALIZE: usize = 0x100;

/// 规范化一个词项
/// * 📌语义上相等⇒一定会被规范到同一形式
pub fn formalize_term(term: &mut Term) -> &mut Term {
    let mut map = VariableNameMap::new();
    let mut modified;
    // 修改到无法修改为止
    // * 📌循环次数有限，防止死循环
    for _ in 0..MAX_TRIES_FORMALIZE {
        // 命名变量
        modified = rename_variables_in_term_with_map(term, &mut map);
        // 排序 | 🚩放后头避免截断
        modified = sort_communicative_terms(term) || modified;
        // * 🚩若无变化⇒退出
        if !modified {
            return term;
        }
        // 重置
        map.clear();
    }
    // * 🚩尝试多次仍未稳定⇒收集信息，panic | 📝这是个程序漏洞，而非可失败的选项：重命名+排序 必定会收敛（有待论证）
    const N: usize = 0x10;
    let mut stack = Vec::with_capacity(N);
    for _ in 0..N {
        use narsese::conversion::string::impl_lexical::format_instances::FORMAT_ASCII;
        // 命名变量
        rename_variables_in_term_with_map(term, &mut map);
        // 排序
        sort_communicative_terms(term);
        // 打印
        stack.push(format!("modified: {:}", FORMAT_ASCII.format(term)));
        // 重置
        map.clear();
    }
    panic!(
        "异常：程序重复尝试了{MAX_TRIES_FORMALIZE}次，仍未稳定。\n堆栈：\n{}",
        stack.join("\n")
    );
}

/// 入口：词项判等
/// * 🚩通过「规整化词项」实现判等逻辑
///   * 📌可交换词项「顺序不影响相等」 ⇒ 固定顺序 ⇒ 排序
///   * 📌变量词项「编号不影响相等」 ⇒ 固定顺序 ⇒ 统一重命名
pub fn semantical_equal_mut(term1: &mut Term, term2: &mut Term) -> bool {
    *formalize_term(term1) == *formalize_term(term2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nar_dev_utils::macro_once;
    use narsese::conversion::string::impl_lexical::format_instances::FORMAT_ASCII;

    fn parse_term(s: &str) -> Term {
        FORMAT_ASCII
            .parse(s)
            .expect("Narsese解析失败")
            .try_into_term()
            .unwrap()
    }

    macro_rules! term {
        ($s:expr) => {
            parse_term($s)
        };
    }

    fn fmt_term(term: &Term) -> String {
        FORMAT_ASCII.format(term)
    }

    fn print_term(term: &Term) {
        println!("{:}", fmt_term(term));
    }

    #[test]
    fn rename_variables() {
        fn t(s: &str) {
            let term = parse_term(s);
            // 普通验证
            print_term(&term);
            let mut renamed = term.clone();
            rename_variables_in_term(&mut renamed);
            print_term(&renamed);
            // 幂等性
            let mut r_renamed = renamed.clone();
            rename_variables_in_term(&mut r_renamed);
            print_term(&r_renamed);
            assert_eq!(&renamed, &r_renamed)
        }
        t("<(&&, <$2 --> $1>, <$3 <-> $2>, S, #4) ==> <<A <-> $1> ==> <$3 --> {(/, R, _, $3), $2}>>>");
        t("<(&&,<$the_one --> lock>,<$second --> key>) ==> <$the_one --> (/,open,$second,_)>>");
        t("<(&&,<$the_one --> key>,<$second --> lock>) ==> <$second --> (/,open,$the_one,_)>>");
    }

    #[test]
    fn sort() {
        // 普通验证
        let term = term!("<[#1, B, $1, A, $2, B] <-> (&&, #1, A, {G,E,B}, [C], <F <=> D>)>");
        print_term(&term);
        let mut renamed = term.clone();
        sort_communicative_terms(&mut renamed);
        print_term(&renamed);
        // 幂等性
        let mut r_renamed = renamed.clone();
        sort_communicative_terms(&mut r_renamed);
        print_term(&r_renamed);
        assert_eq!(&renamed, &r_renamed);
        // 排序判等
        fn sort_eq(term1: &mut Term, term2: &mut Term) -> bool {
            sort_communicative_terms(term1);
            sort_communicative_terms(term2);
            term1 == term2
        }
        macro_once! {
            macro test_ {
                ($($s1:literal $t1:tt $s2:literal)*) => {$(
                    // ! ⚠️等号/不等号 算一个标签树（tt）
                    test_!{ @INNER $s1 $t1 $s2 }
                )*}
                (@INNER $s1:literal == $s2:literal) => {
                    let mut t1 = term!($s1);
                    let mut t2 = term!($s2);
                    assert!(sort_eq(&mut t1, &mut t2), "{} != {}", fmt_term(&t1), fmt_term(&t2));
                }
                (@INNER $s1:literal != $s2:literal) => {
                    let mut t1 = term!($s1);
                    let mut t2 = term!($s2);
                    assert!(!sort_eq(&mut t1, &mut t2), "{} == {}", fmt_term(&t1), fmt_term(&t2));
                }
            }
            "A" == "A"
            "<A <-> B>" == "<B <-> A>"
            "<A --> B>" != "<B --> A>"
            "<(&&,<$1 --> lock>,<$2 --> key>) ==> <$1 --> (/,open,$2,_)>>" ==
            "<(&&,<$2 --> key>,<$1 --> lock>) ==> <$1 --> (/,open,$2,_)>>"
        }
    }

    #[test]
    fn term_compare() {
        macro_once! {
            macro test_ {
                ($($s1:literal $t1:tt $s2:literal)*) => {$(
                    test_!{ @INNER $s1 ($t1) $s2 }
                )*}
                (@VALUE $s1:literal $t1:tt $s2:literal $ordering:expr) => {
                    let t1 = term!($s1);
                    let t2 = term!($s2);
                    let cmp_result = term_comparator(&t1, &t2);
                    assert_eq!(cmp_result, $ordering, "{} !{} {}", fmt_term(&t1), $t1, fmt_term(&t2));
                }
                (@INNER $s1:literal (>) $s2:literal) => {
                    test_!{ @VALUE $s1 ">" $s2 Ordering::Greater }
                }
                (@INNER $s1:literal (<) $s2:literal) => {
                    test_!{ @VALUE $s1 "<" $s2 Ordering::Less }
                }
                (@INNER $s1:literal (==) $s2:literal) => {
                    test_!{ @VALUE $s1 "==" $s2 Ordering::Equal }
                }
            }
            "A" < "B"
            "$1" == "$2"
            "$3" == "$2"
            "$3" == "$a"
            "(&&, A)" > "#1"
            "(&&, #2)" == "(&&, $2)"
            "<$1 --> lock>" > "<$2 --> key>"
        }
    }

    #[test]
    fn formalize() {
        fn t(s: &str) {
            let mut term = parse_term(s);
            print_term(&term);
            formalize_term(&mut term);
            print_term(&term);
            // 幂等性
            let term_original = term.clone();
            formalize_term(&mut term);
            print_term(&term);
            assert!(term == term_original)
        }
        t("<(&&, <$2 --> $1>, <$3 <-> $2>, S, #4) ==> <<A <-> $1> ==> <$3 --> {(/, R, _, $3), $2}>>>");
    }

    #[test]
    fn semantical_eq() {
        macro_once! {
            macro test_ {
                ($($s1:literal $t1:tt $s2:literal)*) => {$(
                    // ! ⚠️等号/不等号 算一个标签树（tt）
                    test_!{ @INNER $s1 $t1 $s2 }
                )*}
                // * 📌等号的情况
                (@INNER $s1:literal == $s2:literal) => {
                    let mut t1 = term!($s1);
                    let mut t2 = term!($s2);
                    let eq = semantical_equal_mut(&mut t1, &mut t2);
                    assert!(eq, "{} != {}", fmt_term(&t1), fmt_term(&t2));
                }
                // * 📌不等号的情况
                (@INNER $s1:literal != $s2:literal) => {
                    let mut t1 = term!($s1);
                    let mut t2 = term!($s2);
                    let eq = semantical_equal_mut(&mut t1, &mut t2);
                    assert!(!eq, "{} == {}", fmt_term(&t1), fmt_term(&t2));
                }
            }
            // * 🚩源自实际场景的例子
               "<(&&,<$1 --> lock>,<$2 --> key>) ==> <$1 --> (/,open,$2,_)>>"
            == "<(&&,<$1 --> key>,<$2 --> lock>) ==> <$2 --> (/,open,$1,_)>>"
               "(&&,<$1 --> lock>,<$2 --> key>)" // * 📝先重命名，根据「变量均相等」交换key和lock，最后
            == "(&&,<$1 --> key>,<$2 --> lock>)"
               "(&&,<$1 --> 🔒>,<$2 --> 🔑>)" // * 对emoji也差不多
            == "(&&,<$1 --> 🔑>,<$2 --> 🔒>)"
               "(&&,<#1 --> 🔒>,<$2 --> 🔑>)" // ! 但这样就不行
            != "(&&,<#1 --> 🔑>,<$2 --> 🔒>)"
            "$1" == "$2"
            "$1" != "A"
            "(/,open,$2,_)" == "(/,open,$1,_)"
            "<$1 --> (/,open,$2,_)>" == "<$2 --> (/,open,$1,_)>"
            "<$1 --> $2>" == "<$2 --> $1>"
            "<$1 --> 2>" != "<$2 --> 1>"
            "<1 --> $2>" != "<$2 --> 1>"
            "<$1 --> #2>" != "<#2 --> $1>"
            "<?1 --> #2>" != "<#2 --> ?1>"
            "<$1 --> ?2>" != "<?2 --> $1>"
            "<$1 --> #2>" == "<$2 --> #1>"
            "<?1 --> #2>" == "<?2 --> #1>"
            "<$1 --> ?2>" == "<$2 --> ?1>"
        }
    }
}
