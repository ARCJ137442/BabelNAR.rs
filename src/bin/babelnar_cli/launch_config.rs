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
//!     translators?: LaunchConfigTranslators;
//!     command?: LaunchConfigCommand;
//!     websocket?: LaunchConfigWebsocket;
//!     prelude_nal?: LaunchConfigPreludeNAL;
//! }
//!
//! type LaunchConfigTranslators = string | {
//!     // ↓虽然`in`是JavaScript/TypeScript/Rust的关键字，但仍可在此直接使用
//!     in: string;
//!     out: string;
//! };
//!
//! type LaunchConfigCommand = {
//!     cmd: string;
//!     cmd_args?: string[];
//!     current_dir?: string;
//! }
//! type LaunchConfigWebsocket = {
//!     host: string;
//!     port: number;
//! }
//! // ↓ 文件、纯文本 二选一
//! type LaunchConfigPreludeNAL = {
//!     file?: string;
//!     text?: string;
//! }
//! ```

use nar_dev_utils::OptionBoost;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaunchConfig {
    /// 转译器组合（可选）
    /// * 🚩使用字符串模糊匹配
    pub translators: Option<LaunchConfigTranslators>,

    /// 启动命令（可选）
    pub command: Option<LaunchConfigCommand>,

    /// Websocket参数（可选）
    pub websocket: Option<LaunchConfigWebsocket>,

    /// 预置NAL（可选）
    pub prelude_nal: Option<LaunchConfigPreludeNAL>,
}

/// 转译器组合
/// * 🚩【2024-04-01 11:20:36】目前使用「字符串+内置模糊匹配」进行有限的「转译器支持」
///   * 🚧尚不支持自定义转译器
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)] // 🔗参考：<https://serde.rs/enum-representations.html#untagged>
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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LaunchConfigPreludeNAL {
    /// 从文件路径导入
    /// * 📌键名：`file`
    #[serde(rename = "file")]
    File(String),
    /// 从文本解析
    /// * 📌键名：`text`
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
    /// * 🚩合并逻辑：`Some(..)` => `None`
    ///   * 当并入者为`Some`，自身为`None`时，合并`Some`中的值
    /// * ✨对【内部含有可选键】的值，会**递归深入**
    pub fn merge_from(&mut self, other: &Self) {
        // 合并所有【不含可选键】的值
        self.translators.coalesce_clone(&other.translators);
        self.prelude_nal.coalesce_clone(&other.prelude_nal);
        self.websocket.coalesce_clone(&other.websocket);
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
                    "cmd_args": ["-Xmx1024m", "-jar", "nars.jar"],
                    "current_dir": "root/nars/test"
                },
                "websocket": {
                    "host": "localhost",
                    "port": 8080
                },
                "prelude_nal": {
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
                prelude_nal: Some(LaunchConfigPreludeNAL::Text("'/VOL 0".into()))
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
                "prelude_nal": {
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
        }
        /*
        "file": "root/path/to/file"
        */
    }
}
