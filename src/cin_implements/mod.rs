//! 对各CIN实现「非公理虚拟机」模型
//! * 🎯基于「NAVM指令/NAVM输出↔字符串」的转换
//!
//! ! ⚠️关键问题：进程残留
//! * ✅单元测试中利用`taskkill`初步解决
//! * ❌【2024-03-25 13:36:30】集成测试`cargo t --all-features`中未能解决
//!   * ❗// ! ↑【少用乃至不用这条命令】
//!
//! ? 【2024-03-25 12:48:08】如何兼顾「复用」「性能」与「简洁」
//! * 📌复用：将OpenNARS、ONA抽象成「基于jar的启动逻辑」「基于exe的启动逻辑」等方式，以便后续重复使用
//!   * 📄case：目前ONA、NARS-Python都是基于exe的启动方式
//! * 📌性能：避免过多的封装、粗暴复合导致的空间浪费
//!   * 📄case：「启动器套启动器」在尝试抽象出「exe启动器」时，因为「没法预先指定转译器」在「复用『设置转译器』函数」时
//!   * ❌不希望在「exe启动器」「jar启动器」中重复套【包含一长串函数闭包】
//! * 📌简洁：代码简明易懂，方便调用方使用
//!   * 📄case：期望能有形如`ONA::new(path).launch()`的语法
//!   * 💭不希望出现「强行模拟」的情况，如`mod ONA {pub fn new(..) {..}}`
//!   * ❌不希望因此再全小写/封装命名空间，如`impls::ona::new`
//! * ❓目前的问题：在Rust基于「特征」的组合式设计哲学下，如何进行兼顾三者的优秀设计

mod common;
mod utils;

// OpenNARS
pub mod opennars;

// ONA
pub mod ona;

// NARS-Python
pub mod nars_python;

// PyNARS
pub mod pynars;

// OpenJunars
pub mod openjunars;
