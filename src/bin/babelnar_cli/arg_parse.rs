//! BabelNAR CLI的命令行（参数 & 配置）解析支持
//! * ⚠️【2024-04-01 14:31:09】特定于二进制crate，目前不要并入[`babel_nar`]

use crate::launch_config::LaunchConfig;
use anyhow::Result;
use clap::Parser;
use nar_dev_utils::{pipe, ResultBoost};
use std::{fs::read_to_string, path::PathBuf};

/// 默认的「外部JSON」路径
pub const DEFAULT_CONFIG_PATH: &str = "BabelNAR.launch.json";

/// 基于[`clap`]的命令行参数数据
// 配置命令行解析器
#[derive(Parser)]
#[command(name = "BabelNAR CLI")]
#[command(about = "BabelNAR's Cmdline Interface", long_about = None)]
#[command(version, about, long_about = None)]
// 其它
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CliArgs {
    // 配置文件路径
    // * ✨可支持加载多个配置
    //   * ⚠️需要重复使用`-c`
    //   * ✅会以「重复使用`-c`」的顺序被载入
    // * 🚩【2024-04-01 13:07:18】具有最高加载优先级
    //   * 📌剩余的是和exe同目录的`json`文件
    // ! 📝此处的文档字符串会被用作`-h`的说明
    /// Configuration file path in JSON (multiple supported by call it multiple times)
    #[arg(short, long, value_name = "FILE")]
    pub config: Vec<PathBuf>,

    // 禁用默认配置
    // * 禁用与exe同目录的配置文件
    // * 📜默认为`false`
    // * 📌行为
    //   * 没有 ⇒ `false`
    //   * 有　 ⇒ `true`
    /// Disable the default configuration file in the same directory as exe
    #[arg(short, long)]
    pub disable_default: bool,

    // 禁用用户输入
    // * 禁用用户对程序的交互式输入
    // * 📜默认为`false`
    // * 📌行为
    //   * 没有 ⇒ `false`
    //   * 有　 ⇒ `true`
    /// Disable the user's ability to interact with the program
    #[arg(short, long)]
    pub no_user_input: bool,
}

/// 加载配置
/// * 🚩按照一定优先级顺序进行覆盖（从先到后）
///   * 命令行参数中指定的配置文件
///   * 默认的JSON文件 | 可以在`disable_default = true`的情况下传入任意字串作占位符
pub fn load_config(args: &CliArgs, default_config_path: impl Into<PathBuf>) -> LaunchConfig {
    // 构建返回值 | 全`None`
    let mut result = LaunchConfig::new();
    // 尝试从命令行参数中读取再合并配置 | 仅提取出其中`Some`的项
    args.config
        // 尝试加载配置文件，对错误采取「警告并抛掉」的策略
        .iter()
        .filter_map(load_config_extern)
        // 逐个从「命令行参数指定的配置文件」中合并
        .for_each(|config| result.merge_from(&config));
    // 若未禁用，尝试读取再合并默认启动配置
    if !args.disable_default {
        // * 🚩读取失败⇒警告&无动作 | 避免多次空合并
        load_config_extern(&default_config_path.into())
            .inspect(|config_extern| result.merge_from(config_extern));
    }
    // 展示加载的配置 | 以便调试（以防其它地方意外插入别的配置）
    match serde_json::to_string(&result) {
        Ok(json) => println!("[INFO] 加载的配置: {json}",),
        Err(e) => println!("[WARN] 展示加载的配置时出现预期之外的错误: {e}"),
    }
    // 返回
    result
}

/// 从外部JSON文件中加载启动配置
/// * 🎯错误处理 & 错误⇒空置
/// * 🚩在遇到错误时会发出警告
pub fn load_config_extern(path: &PathBuf) -> Option<LaunchConfig> {
    // Ok⇒Some，Err⇒警告+None
    read_config_extern(path).ok_or_run(|e| {
        // 根据错误类型进行分派
        if let Some(e) = e.downcast_ref::<std::io::Error>() {
            match e.kind() {
                std::io::ErrorKind::NotFound => {
                    println!("[WARN] 未找到外部配置，使用空配置……");
                }
                _ => println!("[WARN] 读取外部配置时出现预期之外的错误: {}", e),
            }
        } else if let Some(e) = e.downcast_ref::<serde_json::Error>() {
            match e.classify() {
                serde_json::error::Category::Syntax => {
                    println!("[WARN] 外部配置文件格式错误，使用空配置……");
                }
                _ => println!("[WARN] 解析外部配置时出现预期之外的错误: {}", e),
            }
        } else {
            println!("[WARN] 加载外部配置时出现预期之外的错误: {}", e)
        }
        // 空置
    })
}

/// 从外部JSON文件中读取启动配置
/// * 🎯仅涉及具体读取逻辑，不涉及错误处理
pub fn read_config_extern(path: &PathBuf) -> Result<LaunchConfig> {
    // 尝试读取外部启动配置，并尝试解析
    pipe! {
        path
        // 尝试读取文件内容
        => read_to_string
        => {?}#
        // 尝试解析JSON配置
        => #{&}
        => LaunchConfig::from_json_str
        => {?}#
        // 返回Ok（转换为`anyhow::Result`）
        => Ok
    }
    // ! 若需使用`confy`，必须封装
    // * 🚩目前无需使用`confy`：可以自动创建配置文件，但个人希望其路径与exe同目录
    // Ok(confy::load_path(path)?) // ! 必须封装
}

/// 单元测试
#[cfg(test)]
mod tests {
    use super::*;
    use nar_dev_utils::fail_tests;

    /// 测试/参数解析
    mod arg_parse {
        use super::*;

        fn _test_arg_parse(args: &[&str], expected: &CliArgs) {
            // ! 📝此处必须前缀一个「自身程序名」
            let args = CliArgs::parse_from([&["test.exe"], args].concat());
            assert_eq!(dbg!(args), *expected)
        }

        // 快捷测试宏
        macro_rules! test_arg_parse {
            // 成功测试
            {
                $( $args:expr => $expected:expr $(;)? )*
            } => {
                $(
                    _test_arg_parse(&$args, &$expected);
                )*
            };
            // 失败测试
            {
                $args:expr
            } => {
                // 直接使用默认构造，解析成功了大概率报错
                _test_arg_parse(&$args, &CliArgs::default())
            };
        }

        /// 测试/打印帮助
        #[test]
        fn test_arg_parse_help() {
            _test_arg_parse(&["--help"], &CliArgs::default());
        }
        #[test]
        fn test_arg_parse_help2() {
            _test_arg_parse(&["-h"], &CliArgs::default());
        }

        /// 测试/成功的解析
        #[test]
        fn test_arg_parse() {
            test_arg_parse! {
                ["-c", "./src/tests/cli/config_opennars.json"]
                => CliArgs {
                    config: vec!["./src/tests/cli/config_opennars.json".into()],
                    ..Default::default()
                };
                // 多个配置：重复使用`-c`/`--config`，按使用顺序填充
                ["-c", "1.json", "--config", "2.json"]
                => CliArgs {
                    config: vec!["1.json".into(), "2.json".into()],
                    ..Default::default()
                };
                // 禁用默认配置：使用`-d`/`--disable-default`
                ["-d"]
                => CliArgs {
                    disable_default: true,
                    ..Default::default()
                };
            };
        }

        // 失败解析
        fail_tests! {
            fail_缺少参数 test_arg_parse!(["-c"]);
            fail_参数名不对 test_arg_parse!(["--c"]);
            fail_缺少参数2 test_arg_parse!(["--config"]);
            多个参数没各自前缀 test_arg_parse!(["-c", "1.json", "2.json"]);
        }
    }

    /// 测试/加载配置
    mod read_config {
        use super::*;
        use crate::LaunchConfigWebsocket;

        /// 测试/加载配置
        fn load(args: &[&str]) -> LaunchConfig {
            // 读取配置 | 自动填充第一个命令行参数作为「当前程序路径」
            let args = CliArgs::parse_from([&["test.exe"], args].concat());
            let config = load_config(&args, DEFAULT_CONFIG_PATH);
            dbg!(config)
        }

        /// 实用测试宏
        macro_rules! test {
            // 成功测试
            { $( [ $($arg:expr $(,)? )* ] => $expected:expr $(;)? )* } => {
                $( assert_eq!(load(&[ $($arg ),* ]), $expected); )*
            };
            // 失败测试 | 总是返回默认值
            { $( $args:expr $(;)? )* } => {
                $( assert_eq!(load(&$args), LaunchConfig::default()); )*
            };
        }

        /// 测试
        #[test]
        fn test() {
            // 成功测试
            test! {
                // 单个配置文件
                ["-c" "src/tests/cli/config_opennars.json" "-d"] => LaunchConfig {
                    translators: Some(
                        crate::LaunchConfigTranslators::Same(
                            "opennars".into(),
                        ),
                    ),
                    command: None,
                    websocket: None,
                    prelude_nal: None,
                };
                ["-c" "src/tests/cli/config_websocket.json" "-d"] => LaunchConfig {
                    translators: None,
                    command: None,
                    websocket: Some(LaunchConfigWebsocket {
                        host: "localhost".into(),
                        port: 8080,
                    }),
                    prelude_nal: None,
                };
                // 两个配置文件合并
                [
                    "-d"
                    "-c" "src/tests/cli/config_opennars.json"
                    "-c" "src/tests/cli/config_websocket.json"
                ] => LaunchConfig {
                    translators: Some(
                        crate::LaunchConfigTranslators::Same(
                            "opennars".into(),
                        ),
                    ),
                    command: None,
                    websocket: Some(LaunchConfigWebsocket {
                        host: "localhost".into(),
                        port: 8080,
                    }),
                    prelude_nal: None,
                }
            }
        }
    }
}
