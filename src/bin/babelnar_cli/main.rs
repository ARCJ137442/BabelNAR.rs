//! BabelNAR 命令行接口
//! * ✨提供对BabelNAR的命令行支持
//!
//! ## 命令行参数语法
//!
//! ```
//! usage: BabelNAR [OPTIONS] <INPUT>
//! ```
//!
//! TODO: 配置读取、预加载

use clap::Parser;
use std::io::Result as IoResult;
use std::{env, path::PathBuf};

nar_dev_utils::mods! {
    // 启动参数
    use launch_config;
    // 命令行解析
    use arg_parse;
}

pub fn main() {
    // 以默认参数启动
    main_args(env::current_dir(), env::args())
}

/// 以特定参数开始命令行主程序
/// * 🚩此处只应该有自[`env`]传入的参数
/// * 🚩【2024-04-01 14:25:38】暂时用不到「当前工作路径」
pub fn main_args(_cwd: IoResult<PathBuf>, args: impl Iterator<Item = String>) {
    let args = CliArgs::parse_from(args);
    dbg!(&args);
    // 读取配置 | with 默认配置文件
    let config = load_config(&args, DEFAULT_CONFIG_PATH);
    dbg!(config);
}

/// 单元测试
#[cfg(test)]
mod tests {
    // use super::*;
}
