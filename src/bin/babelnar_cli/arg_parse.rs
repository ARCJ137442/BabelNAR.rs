//! BabelNAR CLI的命令行（参数 & 配置）解析支持
//! * ⚠️【2024-04-01 14:31:09】特定于二进制crate，目前不要并入[`babel_nar`]
//! * 🚩【2024-04-04 03:03:58】现在移出所有与「启动配置」相关的逻辑到[`super::vm_config`]

use crate::{load_config_extern, read_config_extern, LaunchConfig};
use babel_nar::println_cli;
use clap::Parser;
use std::{
    env::{current_dir, current_exe},
    path::PathBuf,
};

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
    // ! 🚩【2024-04-02 11:36:18】目前除了「配置加载」外，莫将任何「NAVM实现特定，可以内置到『虚拟机配置』的字段放这儿」
}

/// 默认的「启动配置」关键词
/// * 🎯在「自动追加扩展名」的机制下，可以进行自动补全
/// * 🚩【2024-04-04 05:28:45】目前仍然难以直接在[`PathBuf`]中直接追加字符串
///   * 多词如`BabelNAR-launch`需要使用`-`而非`.`：后者会被识别为「`.launch`扩展名」，导致无法进行「自动补全」
pub const DEFAULT_CONFIG_KEYWORD: &str = "BabelNAR-launch";

/// 获取「默认启动配置」文件
/// * 🎯更灵活地寻找可用的配置文件
///   * exe当前目录下 | 工作目录下
///   * `BabelNAR.launch.(h)json`
pub fn try_load_default_config() -> Option<LaunchConfig> {
    // 检查一个目录
    #[inline(always)]
    fn in_one_root(root: PathBuf) -> Option<LaunchConfig> {
        // 计算路径：同目录下
        let path = match root.is_dir() {
            true => root.join(DEFAULT_CONFIG_KEYWORD),
            false => root.with_file_name(DEFAULT_CONFIG_KEYWORD),
        };
        // 尝试读取，静默失败
        read_config_extern(&path).ok()
    }
    // 寻找第一个可用的配置文件
    [current_dir(), current_exe()]
        // 转换为迭代器
        .into_iter()
        // 筛去转换失败的
        .flatten()
        // 尝试获取其中的一个有效配置，然后（惰性）返回「有效配置」
        .filter_map(in_one_root)
        // 只取第一个（最先遍历的根路径优先）
        .next()
}

/// 加载配置
/// * 🚩按照一定优先级顺序进行覆盖（从先到后）
///   * 命令行参数中指定的配置文件
///   * 默认配置文件路径 | 可以在`disable_default = true`的情况下传入任意字串作占位符
pub fn load_config(args: &CliArgs) -> LaunchConfig {
    // 构建返回值 | 全`None`
    let mut result = LaunchConfig::new();
    // 尝试从命令行参数中读取再合并配置 | 仅提取出其中`Some`的项
    args.config
        // 尝试加载配置文件，对错误采取「警告并抛掉」的策略
        .iter()
        .map(PathBuf::as_ref)
        .filter_map(load_config_extern)
        // 逐个从「命令行参数指定的配置文件」中合并
        .for_each(|config| result.merge_from(&config));
    // 若未禁用，尝试读取再合并默认启动配置
    if !args.disable_default {
        // * 🚩读取失败⇒警告&无动作 | 避免多次空合并
        try_load_default_config().inspect(|config_extern| result.merge_from(config_extern));
    }
    // 展示加载的配置 | 以便调试（以防其它地方意外插入别的配置）
    if result.is_empty() {
        println_cli!([Log] "未加载任何外部配置");
    } else {
        match serde_json::to_string(&result) {
            Ok(json) => println_cli!([Log] "外部配置已加载：{json}",),
            Err(e) => println_cli!([Warn] "展示加载的配置时出现预期之外的错误: {e}"),
        }
    }
    // 返回
    result
}

/// 单元测试
#[cfg(test)]
mod tests {
    use super::*;
    use babel_nar::tests::*;
    use nar_dev_utils::fail_tests;

    /// 测试/参数解析
    mod arg_parse {
        use super::*;
        use config_paths::*;

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
        #[ignore = "【2024-06-12 23:46:44】会导致集成测试无法正常运行"]
        fn test_arg_parse_help() {
            _test_arg_parse(&["--help"], &CliArgs::default());
        }

        #[test]
        #[ignore = "【2024-06-12 23:46:44】会导致集成测试无法正常运行"]
        fn test_arg_parse_help2() {
            _test_arg_parse(&["-h"], &CliArgs::default());
        }

        /// 测试/成功的解析
        #[test]
        #[ignore = "【2024-04-14 20:24:52】会导致残留子进程"]
        fn test_arg_parse() {
            test_arg_parse! {
                ["-c", ARG_PARSE_TEST]
                => CliArgs {
                    config: vec![ARG_PARSE_TEST.into()],
                    ..Default::default()
                };
                // 多个配置：重复使用`-c`/`--config`，按使用顺序填充
                ["-c", "1", "--config", "2"]
                => CliArgs {
                    config: vec!["1".into(), "2".into()],
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
            #[ignore = "【2024-06-12 23:47:41】会导致集成测试无法正常运行"]
            fail_缺少参数 test_arg_parse!(["-c"]);

            #[ignore = "【2024-06-12 23:47:41】会导致集成测试无法正常运行"]
            fail_参数名不对 test_arg_parse!(["--c"]);

            #[ignore = "【2024-06-12 23:47:41】会导致集成测试无法正常运行"]
            fail_缺少参数2 test_arg_parse!(["--config"]);

            #[ignore = "【2024-06-12 23:47:41】会导致集成测试无法正常运行"]
            多个参数没各自前缀 test_arg_parse!(["-c", "1", "2"]);
        }
    }

    /// 测试/加载配置
    mod read_config {
        use super::*;
        use crate::vm_config::*;
        use crate::LaunchConfigWebsocket;
        use config_paths::*;
        use nar_dev_utils::manipulate;

        /// 测试/加载配置
        fn load(args: &[&str]) -> LaunchConfig {
            // 读取配置 | 自动填充第一个命令行参数作为「当前程序路径」
            let args = CliArgs::parse_from([&["test.exe"], args].concat());
            let config = load_config(&args);
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
            let expected_current_dir = manipulate!(
                current_dir().unwrap()
                => .push("src")
                => .push("tests")
                => .push("cli")
                => .push("executables")
            );
            // 成功测试
            test! {
                    // 单个配置文件
                    ["-c" ARG_PARSE_TEST "-d"] => LaunchConfig {
                        translators: Some(
                            LaunchConfigTranslators::Same(
                                "opennars".into(),
                            ),
                        ),
                        command: Some(LaunchConfigCommand {
                            cmd: "java".into(),
                            cmd_args: Some(vec![
                                "-Xmx1024m".into(),
                                "-jar".into(),
                                "nars.jar".into()
                            ]),
                            current_dir: Some(expected_current_dir.clone()),
                        }),
                        ..Default::default()
                    };
                    ["-c" WEBSOCKET "-d"] => LaunchConfig {
                        websocket: Some(LaunchConfigWebsocket {
                            host: "localhost".into(),
                            port: 8080,
                        }),
                        ..Default::default()
                    };
                    // 两个配置文件合并
                    [
                        "-d"
                        "-c" ARG_PARSE_TEST
                        "-c" WEBSOCKET
                    ] => LaunchConfig {
                        translators: Some(
                            LaunchConfigTranslators::Same(
                                "opennars".into(),
                            ),
                        ),
                        command: Some(LaunchConfigCommand {
                            cmd: "java".into(),
                            cmd_args: Some(vec![
                                "-Xmx1024m".into(),
                                "-jar".into(),
                                "nars.jar".into()
                            ]),
                            current_dir: Some(expected_current_dir.clone()),
                        }),
                        websocket: Some(LaunchConfigWebsocket {
                            host: "localhost".into(),
                            port: 8080,
                        }),
                        ..Default::default()
                    };
                    // 三个配置文件合并
                    [
                        "-d"
                        "-c" ARG_PARSE_TEST
                        "-c" WEBSOCKET
                        "-c" PRELUDE_TEST
                    ] => LaunchConfig {
                        translators: Some(
                            LaunchConfigTranslators::Same(
                                "opennars".into(),
                            ),
                        ),
                        command: Some(LaunchConfigCommand {
                            cmd: "java".into(),
                            cmd_args: Some(vec![
                                "-Xmx1024m".into(),
                                "-jar".into(),
                                "nars.jar".into()
                            ]),
                            current_dir: Some(expected_current_dir.clone()),
                        }),
                        websocket: Some(LaunchConfigWebsocket {
                            host: "localhost".into(),
                            port: 8080,
                        }),
                        user_input: Some(false),
                        auto_restart: Some(false),
                        strict_mode: Some(true),
                        ..Default::default()
                    }
            }
        }
    }
}
