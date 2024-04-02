//! 输出打印
//! * 🎯用于规范化、统一、美化CLI输出
//!   * 📌不仅仅是NAVM的输出
//!
//! ## 输出美化参考
//!
//! 输出美化逻辑参考了如下Julia代码：
//!
//! ```julia
//! """
//! 用于高亮「输出颜色」的字典
//! """
//! const output_color_dict = Dict([
//!     NARSOutputType.IN => :light_white
//!     NARSOutputType.OUT => :light_white
//!     NARSOutputType.EXE => :light_cyan
//!     NARSOutputType.ANTICIPATE => :light_yellow
//!     NARSOutputType.ANSWER => :light_green
//!     NARSOutputType.ACHIEVED => :light_green
//!     NARSOutputType.INFO => :white
//!     NARSOutputType.COMMENT => :white
//!     NARSOutputType.ERROR => :light_red
//!     NARSOutputType.OTHER => :light_black # * 未识别的信息
//!     # ! ↓这俩是OpenNARS附加的
//!     "CONFIRM" => :light_blue
//!     "DISAPPOINT" => :light_magenta
//! ])
//!
//! """
//! 用于分派「颜色反转」的集合
//! """
//! const output_reverse_color_dict = Set([
//!     NARSOutputType.EXE
//!     NARSOutputType.ANSWER
//!     NARSOutputType.ACHIEVED
//! ])
//! ```
//!
//! * 最后更新：【2024-04-02 15:54:23】
//! * 参考链接：<https://github.com/ARCJ137442/BabelNAR_Implements/blob/master/scripts/console.jl#L160>

use colored::Colorize;
use navm::output::Output;
use std::fmt::Display;

/// 统一的「CLI输出类型」
#[derive(Debug, Clone, Copy)]
pub enum OutputType<'a> {
    /// NAVM输出
    /// * 🚩【2024-04-02 15:42:44】目前因NAVM的[`Output`]仅有`enum`结构而无「类型」标签，
    ///   * 无法复用NAVM的枚举
    Vm(&'a str),
    /// CLI错误
    Error,
    /// CLI警告
    Warn,
    /// CLI信息
    Info,
    /// CLI日志
    Log,
    /// CLI debug
    Debug,
}

impl OutputType<'_> {
    /// 自身的字符串形式
    /// * 🎯作为输出的「头部」
    pub fn as_str(&self) -> &str {
        match self {
            OutputType::Vm(s) => s,
            OutputType::Error => "ERROR",
            OutputType::Warn => "WARN",
            OutputType::Info => "INFO",
            OutputType::Debug => "DEBUG",
            OutputType::Log => "LOG",
        }
    }

    /// 格式化CLI输出
    /// * 🎯封装标准输出形式：`[类型] 内容`
    /// * 🎯封装命令行美化逻辑
    #[inline(always)]
    pub fn format_line(&self, msg: &str) -> impl Display {
        self.to_colored_str(format!("[{}] {}", self.as_str(), msg))
    }

    /// 从NAVM输出格式化
    /// * 🎯封装「从NAVM输出打印」
    #[inline(always)]
    pub fn format_from_navm_output(out: &Output) -> impl Display {
        OutputType::from(out).format_line(out.raw_content().trim_end())
    }

    /// 基于[`colored`]的输出美化
    /// * 🎯用于CLI的彩色输出
    /// * 🔗参考Julia版本<https://github.com/ARCJ137442/BabelNAR_Implements/blob/master/scripts/console.jl#L160>
    pub fn to_colored_str(&self, message: String) -> impl Display {
        match self.as_str() {
            // CLI独有
            "DEBUG" => message.bright_blue(),
            "WARN" => message.bright_yellow(),
            "LOG" => message.white(),
            // NAVM输出
            "IN" | "OUT" => message.bright_white(),
            "EXE" => message.bright_cyan().reversed(),
            "ANSWER" | "ACHIEVED" => message.bright_green().reversed(),
            "INFO" | "COMMENT" => message.white(),
            "ERROR" => message.red(),
            "TERMINATED" => message.bright_white().reversed().blink(),
            // ↓OpenNARS附加
            "ANTICIPATE" => message.bright_yellow(),
            "CONFIRM" => message.bright_blue(),
            "DISAPPOINT" => message.bright_magenta(),
            // 默认 / 其它
            "OTHER" => message.bright_black(),
            _ => message.bright_white(),
        }
        // 参考Julia，始终加粗
        .bold()
    }

    /// ✨格式化打印CLI输出
    /// * 🎯BabelNAR CLI
    #[inline]
    pub fn print_line(&self, message: &str) {
        println!("{}", self.format_line(message));
    }

    /// ✨格式化打印NAVM输出
    /// * 🎯BabelNAR CLI
    #[inline]
    pub fn print_from_navm_output(out: &Output) {
        println!("{}", Self::format_from_navm_output(out));
    }

    /// ✨格式化打印CLI输出（标准错误）
    /// * 🎯BabelNAR CLI
    #[inline]
    pub fn eprint_line(&self, message: &str) {
        eprintln!("{}", self.format_line(message));
    }

    /// ✨格式化打印NAVM输出（标准错误）
    /// * 🎯BabelNAR CLI
    #[inline]
    pub fn eprint_from_navm_output(out: &Output) {
        eprintln!("{}", Self::format_from_navm_output(out));
    }
}

/// 快捷打印宏
#[macro_export]
macro_rules! println_cli {
    ([$enum_type_name:ident] $($tail:tt)*) => {
        // 调用内部函数
        $crate::cli_support::io::output_print::OutputType::$enum_type_name.print_line(&format!($($tail)*));
    };
    ($navm_output:expr) => {
        // 调用内部函数
        $crate::cli_support::io::output_print::OutputType::print_from_navm_output($navm_output);
    };
}

/// 快捷打印宏/标准错误
#[macro_export]
macro_rules! eprintln_cli {
    ([$enum_type_name:ident] $($tail:tt)*) => {
        // 调用内部函数
        $crate::cli_support::io::output_print::OutputType::$enum_type_name.eprint_line(&format!($($tail)*));
    };
    ($navm_output:expr) => {
        // 调用内部函数
        $crate::cli_support::io::output_print::OutputType::eprint_from_navm_output($navm_output);
    };
}

impl<'a> From<&'a Output> for OutputType<'a> {
    fn from(out: &'a Output) -> Self {
        OutputType::Vm(out.type_name())
    }
}
