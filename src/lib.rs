//! 主模块
//! * ✨进程IO库
//! * ✨通用运行时
//! * ✨运行时的各类实现（可选）

// 实用库别名
pub extern crate nar_dev_utils as util;

util::mods! {
    // 必选模块 //

    // 进程IO
    pub process_io;

    // NAVM运行时
    pub runtimes;

    // 输出处理者
    pub output_handler;

    // 可选模块 //

    // 各CIN的启动器、运行时实现
    "cin_implements" => pub cin_implements;

    // 命令行支持
    "cli_support" => pub cli_support;

    // 测试工具集
    "test_tools" => pub test_tools;
}
