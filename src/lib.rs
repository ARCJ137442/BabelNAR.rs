//! 主模块
//! * ✨进程IO库
//! * ✨通用运行时
//! * ✨运行时的各类实现（可选）

// 实用库别名
pub extern crate nar_dev_utils as util;

// 必选模块 //
// 进程IO
pub mod process_io;
// 运行时
pub mod runtime;
// （可选的实用）工具
pub mod tools;

// 可选模块 //
util::feature_pub_mod_and_reexport! {
    // 运行时实现
    "cin_implements" => cin_implements
}
