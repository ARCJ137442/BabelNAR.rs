//! 用于存储NARS-Python的方言格式
//! * 🚩【2024-03-26 01:31:44】本质上就是陈述括弧改变了而已

use narsese::conversion::string::impl_lexical::{
    format_instances::create_format_ascii, NarseseFormat,
};
use narsese::lexical::Narsese;

#[cfg(feature = "lazy_static")]
lazy_static::lazy_static! {
    /// NARS-Python的方言格式
    /// * 🚩仅在`lazy_static`启用时开启
    pub static ref FORMAT: NarseseFormat = create_format_nars_python();
}

pub fn create_format_nars_python() -> NarseseFormat {
    let mut f = create_format_ascii();
    f.statement.brackets = ("(".into(), ")".into());
    f
}

/// 获取NARS-Python的方言格式
/// * 🚩使用`lazy_static`定义的静态常量，无需重复初始化
/// * 🚩否则总是创建一个新的「Narsese格式」
#[cfg(feature = "lazy_static")]
pub fn format_in_nars_python(narsese: &Narsese) -> String {
    FORMAT.format_narsese(narsese)
}

/// 获取NARS-Python的方言格式
/// * 🚩否则总是创建一个新的「Narsese格式」
#[cfg(not(feature = "lazy_static"))]
pub fn format_in_nars_python(narsese: &Narsese) -> String {
    // 创建格式，并立即格式化Narsese
    create_format_nars_python().format_narsese(narsese)
}
