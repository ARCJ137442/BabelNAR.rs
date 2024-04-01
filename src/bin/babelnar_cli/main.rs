//! BabelNAR 命令行接口
//! * ✨提供对BabelNAR的命令行支持
//!
//! ## 命令行参数语法
//!
//! ```
//! usage: BabelNAR [OPTIONS] <INPUT>
//! ```

use clap::Parser;
use std::io::Result as IoResult;
use std::thread::sleep;
use std::time::Duration;
use std::{env, path::PathBuf};

nar_dev_utils::mods! {
    // 启动参数
    use launch_config;
    // 命令行解析
    use arg_parse;
    // 从参数启动
    use config_launcher;
    // 运行时交互、管理
    use runtime_manage;
}

/// 主入口
pub fn main() {
    // 以默认参数启动
    main_args(env::current_dir(), env::args())
}

/// 以特定参数开始命令行主程序
/// * 🚩此处只应该有自[`env`]传入的参数
/// * 🚩【2024-04-01 14:25:38】暂时用不到「当前工作路径」
pub fn main_args(_cwd: IoResult<PathBuf>, args: impl Iterator<Item = String>) {
    let args = CliArgs::parse_from(args);
    // 读取配置 | with 默认配置文件
    let mut config = load_config(&args, DEFAULT_CONFIG_PATH);
    // 用户填充配置项
    polyfill_config_from_user(&mut config);
    // 从配置项启动
    let runtime = match launch_by_config(config) {
        // 启动成功⇒返回
        Ok(runtime) => runtime,
        // 启动失败⇒打印错误信息，等待并退出
        Err(e) => {
            println!("NARS运行时启动错误：{e}");
            println!("程序将在 3 秒后自动退出。。。");
            sleep(Duration::from_secs(3));
            return;
        }
    };
    // 运行时交互、管理
    if let Err(e) = manage(runtime, &args) {
        println!("运行时发生错误：{e}");
    }
    // 最终退出
    println!("程序将在 5 秒后退出");
    sleep(Duration::from_secs(5));
}

/// 单元测试
#[cfg(test)]
mod tests {
    use super::*;

    /// 测试入口
    #[test]
    pub fn main_ona() {
        // 以默认参数启动
        main_args(
            env::current_dir(),
            [
                "test.exe",
                "-d",
                "-c",
                "./src/tests/cli/config_test_ona.json",
            ]
            .into_iter()
            .map(str::to_string),
        )
    }
}
