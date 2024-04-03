//! BabelNAR CLI的启动配置
//! * ✨格式支持
//!   * ✅JSON
//! * 🎯用于配置表示，❗不用于命令行解析
//! * ⚠️【2024-04-01 14:31:09】特定于二进制crate，目前不要并入[`babel_nar`]
//!
//! ## ⚙️内容
//!
//! Rust结构：
//!
//! * 📌转译器组合?
//!   * （互斥）单个值?（输入输出相同） `opennars` / `ona` / `nars-python` / `pynars` / `openjunars` / `cxin-js`
//!   * （互斥）输入输出单独配置?
//!     * 输入 `opennars` / `ona` / `nars-python` / `pynars` / `openjunars` / `cxin-js`
//!     * 输出 `opennars` / `ona` / `nars-python` / `pynars` / `openjunars` / `cxin-js`
//! * 📌启动命令?
//!   * 命令 `XXX.exe` / `python` / `java` / `node` / ...
//!   * 命令参数? `["-m", 【Python模块】]` / `["-jar", 【Jar路径】]`
//!   * 工作目录? `root/path/to/current_dir` | 🎯用于Python模块
//! * 📌预置NAL?
//!   * （互斥）文件路径? `root/path/to/file` | 与下边「纯文本」互斥
//!   * （互斥）纯文本? `"'/VOL 0"`
//! * 📌Websocket参数? | ✅支持ipv6
//!   * 主机地址 `localhost` `192.168.1.1` `fe80::abcd:fade:dad1`
//!   * 连接端口 `3040`
//!
//! TypeScript声明：
//!
//! ```ts
//! type LaunchConfig = {
//!     translators?: LaunchConfigTranslators,
//!     command?: LaunchConfigCommand,
//!     websocket?: LaunchConfigWebsocket,
//!     preludeNAL?: LaunchConfigPreludeNAL,
//!     userInput?: boolean
//!     inputMode?: InputMode
//!     autoRestart?: boolean
//! }
//!
//! type InputMode = 'cmd' | 'nal'
//!
//! type LaunchConfigTranslators = string | {
//!     // ↓虽然`in`是JavaScript/TypeScript/Rust的关键字，但仍可在此直接使用
//!     in: string,
//!     out: string,
//! }
//!
//! type LaunchConfigCommand = {
//!     cmd: string,
//!     cmdArgs?: string[],
//!     currentDir?: string,
//! }
//! type LaunchConfigWebsocket = {
//!     host: string,
//!     port: number, // Uint16
//! }
//! // ↓ 文件、纯文本 二选一
//! type LaunchConfigPreludeNAL = {
//!     file?: string,
//!     text?: string,
//! }
//! ```

use std::path::PathBuf;

use nar_dev_utils::OptionBoost;
use serde::{Deserialize, Serialize};

/// NAVM虚拟机（运行时）启动配置
/// * 🎯启动完整的NAVM实例，并附带相关运行时配置
///   * ✨启动时数据提供
///   * ✨运行时数据提供
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")] // 🔗参考：<https://serde.rs/container-attrs.html>
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchConfig {
    /// 转译器组合（可选）
    /// * 🚩使用字符串模糊匹配
    pub translators: Option<LaunchConfigTranslators>,

    /// 启动命令（可选）
    pub command: Option<LaunchConfigCommand>,

    /// Websocket参数（可选）
    /// * 🚩【2024-04-03 18:21:00】目前对客户端输出JSON
    pub websocket: Option<LaunchConfigWebsocket>,

    /// 预置NAL（可选）
    #[serde(rename = "preludeNAL")] // * 📝serde配置中，`rename`优先于`rename_all`
    pub prelude_nal: Option<LaunchConfigPreludeNAL>,

    /// 启用用户输入（可选）
    /// * 🎯控制该实例是否需要（来自用户的）交互式输入
    /// * 📜默认值：`true`
    /// * 📝serde中，若不使用`bool::default`(false)，需要指定一个函数来初始化
    ///   * ⚠️即便在[`LaunchConfig`]中定义了[`default`]，也会使用[`bool::default`]
    #[serde(default = "bool_true")]
    pub user_input: bool,

    /// 输入模式
    /// * 🚩对输入（不论交互还是Websocket）采用的解析模式
    ///   * 📄用于纯NAVM指令（可选）的解析
    /// * 🎯用于兼容旧`BabelNAR.jl`服务端
    /// * 📜默认为`"nal"`
    /// Disable the user's ability to interact with the program
    #[serde(default)]
    pub input_mode: InputMode,

    /// 自动重启
    /// * 🎯程序健壮性：用户的意外输入，不会随意让程序崩溃
    /// * 🚩在虚拟机终止（收到「终止」输出）时，自动用配置重启虚拟机
    /// * 📜默认为`false`（关闭）
    #[serde(default = "bool_false")]
    pub auto_restart: bool,

    /// 严格模式
    /// * 🎯测试敏感性：测试中的「预期失败」可以让程序上报异常
    /// * 🚩在虚拟机终止（收到「终止」输出）时，自动用配置重启虚拟机
    /// * 📜默认为`false`（关闭）
    #[serde(default = "bool_false")]
    pub strict_mode: bool,
}

/// 布尔值`true`
/// * 🎯配置解析中「默认为`true`」的默认值指定
/// * 📝serde中，`#[serde(default)]`使用的是[`bool::default`]而非容器的`default`
///   * 因此需要指定一个函数来初始化
#[inline(always)]
const fn bool_true() -> bool {
    true
}

#[inline(always)]
const fn bool_false() -> bool {
    false
}

impl Default for LaunchConfig {
    fn default() -> Self {
        Self {
            // [`Option`]全部为[`None`]
            translators: None,
            command: None,
            websocket: None,
            prelude_nal: None,
            // 默认启用用户输入
            user_input: true,
            // 输入模式传递默认值
            input_mode: InputMode::default(),
            // 不自动重启
            auto_restart: false,
            // 不开启严格模式
            strict_mode: false,
        }
    }
}

/// NAVM实例的输入类型
/// * 🎯处理用户输入、Websocket输入的解析方式
/// * 📜默认值：`nal`
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
// #[serde(untagged)] // ! 🚩【2024-04-02 18:14:16】不启用方通过：本质上是几个字符串里选一个
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum InputMode {
    /// （NAVM）指令
    /// * 📄类型：[`navm::cmd::Cmd`]
    #[serde(rename = "cmd")]
    Cmd,
    /// `.nal`输入
    /// * 📜默认值
    /// * 📄类型：[`babel_nar::test_tools::NALInput`]
    #[serde(rename = "nal")]
    #[default]
    Nal,
}

/// 转译器组合
/// * 🚩【2024-04-01 11:20:36】目前使用「字符串+内置模糊匹配」进行有限的「转译器支持」
///   * 🚧尚不支持自定义转译器
#[derive(Serialize, Deserialize)]
#[serde(untagged)] // 🔗参考：<https://serde.rs/enum-representations.html#untagged>
#[serde(rename_all = "camelCase")] // 🔗参考：<https://serde.rs/container-attrs.html>
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchConfigTranslators {
    /// 🚩单个字符串⇒输入输出使用同一个转译配置
    Same(String),

    /// 🚩一个对象⇒输入和输出分别使用不同的转译配置
    Separated {
        #[serde(rename = "in")]
        input: String,
        #[serde(rename = "out")]
        output: String,
    },
}

/// 启动命令
/// * ❓后续可能支持「自动搜索」
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")] // 🔗参考：<https://serde.rs/container-attrs.html>
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LaunchConfigCommand {
    /// 命令
    /// * 直接对应[`std::process::Command`]
    /// * 🚩[`Default`]中默认对应空字串
    pub cmd: String,

    /// 命令的参数（可选）
    pub cmd_args: Option<Vec<String>>,

    /// 工作目录（可选）
    /// * 🎯可用于Python模块
    pub current_dir: Option<String>,
}

/// Websocket参数
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")] // 🔗参考：<https://serde.rs/container-attrs.html>
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LaunchConfigWebsocket {
    /// 主机地址
    /// * 📄`localhost`
    /// * 📄`192.168.0.0`
    /// * 📄`fe80::abcd:fade:dad1`
    pub host: String,

    /// 连接端口
    /// * 🚩采用十六位无符号整数
    ///   * 📄范围：0 ~ 65535
    ///   * 🔗参考：<https://zh.wikipedia.org/wiki/通訊埠>
    pub port: u16,
}

/// 预置NAL
/// * 🚩在CLI启动后自动执行
/// * 📝[`serde`]允许对枚举支持序列化/反序列化
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")] // 🔗参考：<https://serde.rs/container-attrs.html>
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchConfigPreludeNAL {
    /// 从文件路径导入
    /// * 📌键名：`file`
    /// * 📌类型：路径
    #[serde(rename = "file")]
    File(PathBuf),

    /// 从文本解析
    /// * 📌键名：`text`
    /// * 📌类型：纯文本（允许换行等）
    #[serde(rename = "text")]
    Text(String),
}

/// 启动配置
impl LaunchConfig {
    /// 零参构造函数
    /// * 🚩使用[`Default`]提供默认空数据
    pub fn new() -> Self {
        Self::default()
    }

    /// （尝试）从JSON字符串构造
    pub fn from_json_str(json: &str) -> serde_json::Result<Self> {
        serde_json::from_str(json)
    }

    /// 判断其自身是否需要用户填充
    /// * 🎯用于在「启动NAVM运行时」时避免「参数无效」情况
    /// * 🚩判断「启动时必要项」是否为空
    pub fn need_polyfill(&self) -> bool {
        // 启动命令非空
        self.command.is_none() ||
        // 输入输出转译器非空
        self.translators.is_none()
        // ! Websocket为空⇒不启动Websocket服务器
        // ! 预加载NAL为空⇒不预加载NAL
    }

    /// 从另一个配置中并入配置
    /// * 📌优先级：`other` > `self`
    /// * 🚩合并逻辑：`Some(..)` => `None`
    ///   * 当并入者为`Some`，自身为`None`时，合并`Some`中的值
    /// * ✨对【内部含有可选键】的值，会**递归深入**
    ///
    /// TODO: ❓启动时与运行时需要分开：不推荐直接覆盖布尔值
    pub fn merge_from(&mut self, other: &Self) {
        // 合并所有【不含可选键】的值
        self.translators.coalesce_clone(&other.translators);
        self.prelude_nal.coalesce_clone(&other.prelude_nal);
        self.websocket.coalesce_clone(&other.websocket);
        // ! 覆盖所有【必定有】的值 | 如：布尔值
        self.user_input = other.user_input;
        self.input_mode = other.input_mode;
        self.auto_restart = other.auto_restart;
        self.strict_mode = other.strict_mode;
        // 递归合并所有【含有可选键】的值
        LaunchConfigCommand::merge_as_key(&mut self.command, &other.command);
    }
}

impl LaunchConfigCommand {
    /// 从另一个配置中并入配置
    /// * 🚩`Some(..)` => `None`
    pub fn merge_from(&mut self, other: &Self) {
        self.cmd_args.coalesce_clone(&other.cmd_args);
        self.current_dir.coalesce_clone(&other.current_dir);
    }

    /// 作为一个键，从另一个配置中并入配置
    /// * 🚩`Some(..)` => `None`
    /// * 适用于自身为[`Option`]的情况
    pub fn merge_as_key(option: &mut Option<Self>, other: &Option<Self>) {
        // 先处理「自身为`None`」的情况
        option.coalesce_clone(other);
        // 双重`inspect`
        if let (Some(config_self), Some(config_other)) = (option, other) {
            config_self.merge_from(config_other);
        }
    }
}

/// 单元测试
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Result;

    macro_rules! test {
        { $( $data:expr => $expected:expr )* } => {
            $(
                _test(&$data, &$expected).expect("测试失败");
            )*
        };
    }

    fn _test(data: &str, expected: &LaunchConfig) -> Result<()> {
        // Some JSON input data as a &str. Maybe this comes from the user.
        let parsed = LaunchConfig::from_json_str(data)?;

        dbg!(&parsed);
        assert_eq!(parsed, *expected);

        Ok(())
    }

    /// 主测试
    #[test]
    fn main() {
        test! {
            // 平凡情况/空
            "{}" => LaunchConfig::new()
            "{}" => LaunchConfig::default()
            // 完整情况
            r#"
            {
                "translators": "opennars",
                "command": {
                    "cmd": "java",
                    "cmdArgs": ["-Xmx1024m", "-jar", "nars.jar"],
                    "currentDir": "root/nars/test"
                },
                "websocket": {
                    "host": "localhost",
                    "port": 8080
                },
                "preludeNAL": {
                    "text": "'/VOL 0"
                }
            }"# => LaunchConfig {
                translators: Some(LaunchConfigTranslators::Same("opennars".into())),
                command: Some(LaunchConfigCommand {
                    cmd: "java".into(),
                    cmd_args: Some(vec!["-Xmx1024m".into(), "-jar".into(), "nars.jar".into()]),
                    current_dir: Some("root/nars/test".into())
                }),
                websocket: Some(LaunchConfigWebsocket{
                    host: "localhost".into(),
                    port: 8080
                }),
                prelude_nal: Some(LaunchConfigPreludeNAL::Text("'/VOL 0".into())),
                ..Default::default()
            }
            // 测试`translators`、`prelude_nal`的其它枚举
            r#"
            {
                "translators": {
                    "in": "opennars",
                    "out": "ona"
                },
                "command": {
                    "cmd": "root/nars/open_ona.exe"
                },
                "preludeNAL": {
                    "file": "root/nars/prelude.nal"
                }
            }"# => LaunchConfig {
                translators: Some(LaunchConfigTranslators::Separated {
                    input: "opennars".into(),
                    output: "ona".into()
                }),
                command: Some(LaunchConfigCommand {
                    cmd: "root/nars/open_ona.exe".into(),
                    ..Default::default()
                }),
                prelude_nal: Some(LaunchConfigPreludeNAL::File("root/nars/prelude.nal".into())),
                ..Default::default()
            }
            r#"
            {
                "inputMode": "cmd"
            }"# => LaunchConfig {
                input_mode: InputMode::Cmd,
                ..Default::default()
            }
            r#"{
                "autoRestart": true,
                "userInput": false
            }"# => LaunchConfig {
                auto_restart: true,
                user_input: false,
                ..Default::default()
            }
        }
        /*
        "file": "root/path/to/file"
        */
    }
}
