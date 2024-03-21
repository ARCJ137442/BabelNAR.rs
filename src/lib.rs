//! 主模块
//! * ✨进程IO库
//! * ✨通用运行时
//! * ✨运行时的各类实现（可选）

// 实用库别名
pub extern crate nar_dev_utils as util;

// 必选模块
util::pub_mod_and_pub_use! {
    // 进程IO
    process_io
    // 运行时
    runtime
}

// 可选模块
util::feature_pub_mod_and_reexport!{
    // 运行时实现
    "implements" => impl_runtime
}