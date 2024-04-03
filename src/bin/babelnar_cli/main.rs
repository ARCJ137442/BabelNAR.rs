//! BabelNAR 命令行接口
//! * ✨提供对BabelNAR的命令行支持
//!
//! ## 命令行参数语法
//!
//! ```
//! usage: BabelNAR [OPTIONS] <INPUT>
//! ```

use anyhow::Result;
use babel_nar::{eprintln_cli, println_cli};
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
    // Websocket服务端
    use websocket_server;
}

/// 主入口
pub fn main() -> Result<()> {
    // 以默认参数启动
    main_args(env::current_dir(), env::args())
}

/// 以特定参数开始命令行主程序
/// * 🚩此处只应该有自[`env`]传入的参数
/// * 🚩【2024-04-01 14:25:38】暂时用不到「当前工作路径」
pub fn main_args(_cwd: IoResult<PathBuf>, args: impl Iterator<Item = String>) -> Result<()> {
    // （Windows下）启用终端颜色
    let _ = colored::control::set_virtual_terminal(true)
        .inspect_err(|_| eprintln_cli!([Error] "无法启动终端彩色显示。。"));
    // 解析命令行参数
    let args = CliArgs::parse_from(args);
    // 读取配置 | with 默认配置文件
    let mut config = load_config(&args, DEFAULT_CONFIG_PATH);
    // 用户填充配置项
    polyfill_config_from_user(&mut config);
    // 从配置项启动 | 复制一个新配置，不会附带任何非基础类型开销
    let runtime = match launch_by_config(config.clone()) {
        // 启动成功⇒返回
        Ok(runtime) => runtime,
        // 启动失败⇒打印错误信息，等待并退出
        Err(e) => {
            println_cli!([Error] "NARS运行时启动错误：{e}");
            // 启用用户输入时延时提示
            if config.user_input {
                println_cli!([Info] "程序将在 3 秒后自动退出。。。");
                sleep(Duration::from_secs(3));
            }
            return Err(e);
        }
    };
    // 运行时交互、管理
    let manager = RuntimeManager::new(runtime, config.clone());
    let result = loop_manage(manager, &config);

    // 启用用户输入时延时提示
    if config.user_input {
        println_cli!([Info] "程序将在 5 秒后自动退出。。。");
        sleep(Duration::from_secs(3));
    }

    // 返回结果
    result
}

/// 单元测试
#[cfg(test)]
mod tests {
    use super::*;

    /// 测试入口/ONA/交互shell
    /// * 🎯正常BabelNAR CLI shell启动
    /// * 🎯正常用户命令行交互体验
    /// * ⚠️使用与项目无关的路径，以定位启动CIN
    #[test]
    pub fn main_ona_shell() -> Result<()> {
        // 以默认参数启动
        main_args(
            env::current_dir(),
            [
                "test.exe",
                "-d",
                "-c",
                "./src/tests/cli/config/test_ona.json",
            ]
            .into_iter()
            .map(str::to_string),
        )
    }

    /// 测试入口/ONA/预加载NAL
    /// * 🎯多「虚拟机启动配置」合并
    /// * 🎯预引入NAL
    /// * ⚠️使用与项目无关的路径，以定位启动CIN
    pub fn main_ona_prelude(prelude_config_path: &str) -> Result<()> {
        // 以默认参数启动
        main_args(
            env::current_dir(),
            [
                "test.exe",
                "-d",
                // 第一个文件，指示ONA
                "-c",
                "./src/tests/cli/config/test_ona.json",
                // 第二个文件，指示预加载
                "-c",
                prelude_config_path,
            ]
            .into_iter()
            .map(str::to_string),
        )
    }

    #[test]
    pub fn test_ona_prelude_de() -> Result<()> {
        main_ona_prelude("./src/tests/cli/config/test_prelude_simple_deduction.json")
    }

    #[test]
    pub fn test_ona_prelude_op() -> Result<()> {
        main_ona_prelude("./src/tests/cli/config/test_prelude_operation.json")
    }
    /// 测试入口/ONA/交互shell
    /// * 🎯正常BabelNAR CLI shell启动
    /// * 🎯正常用户命令行交互体验
    /// * ⚠️使用与项目无关的路径，以定位启动CIN
    #[test]
    pub fn main_ona_websocket() -> Result<()> {
        // 以默认参数启动
        main_args(
            env::current_dir(),
            [
                "test.exe",
                "-d",
                "-c",
                "./src/tests/cli/config/test_ona.json",
                "-c",
                "./src/tests/cli/config/websocket.json",
            ]
            .into_iter()
            .map(str::to_string),
        )
    }
}
