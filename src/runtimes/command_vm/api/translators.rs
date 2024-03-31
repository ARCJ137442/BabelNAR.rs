use anyhow::Result;
use navm::{cmd::Cmd, output::Output};
use std::{error::Error, fmt::Display};

/// [`Cmd`]→进程输入 转译器
/// * 🚩现在不再使用特征，以便在`Option<Box<InputTranslator>>`中推断类型
///   * 📝若给上边类型传入值`None`，编译器无法自动推导合适的类型
/// * 📌要求线程稳定
///   * 只有转译功能，没有其它涉及外部的操作（纯函数）
pub type InputTranslator = dyn Fn(Cmd) -> Result<String> + Send + Sync;

/// 进程输出→[`Output`]转译器
/// * 🚩现在不再使用特征，以便在`Option<Box<OutputTranslator>>`中推断类型
///   * 📝若给上边类型传入值`None`，编译器无法自动推导合适的类型
/// * 📌要求线程稳定
///   * 只有转译功能，没有其它涉及外部的操作（纯函数）
pub type OutputTranslator = dyn Fn(String) -> Result<Output> + Send + Sync;

/// IO转换器配置
/// * 🎯封装并简化其它地方的`translator: impl Fn(...) -> ... + ...`逻辑
/// * 📝【2024-03-27 10:38:41】无论何时都不推荐直接用`impl Fn`作为字段类型
///   * ⚠️直接使用会意味着「需要编译前确定类型」
///   * ❌这会【非必要地】要求一些【不直接传入闭包】的「默认初始化」方法具有类型标注
pub struct IoTranslators {
    pub input_translator: Box<InputTranslator>,
    pub output_translator: Box<OutputTranslator>,
}

impl IoTranslators {
    /// 构造函数
    /// * 🎯基于位置参数构造结构体
    /// * 🎯无需在调用方引入`Box::new`
    /// * 📌需要直接传入闭包（要求全局周期`'static`）
    pub fn new<I, O>(i: I, o: O) -> Self
    where
        I: Fn(Cmd) -> Result<String> + Send + Sync + 'static,
        O: Fn(String) -> Result<Output> + Send + Sync + 'static,
    {
        Self {
            input_translator: Box::new(i),
            output_translator: Box::new(o),
        }
    }
}

impl Default for IoTranslators {
    /// 构造一个默认的「转译器组合」
    /// * 🎯默认生成的转译器
    ///   * 输入：直接将NAVM指令转换为字符串
    ///   * 输出：直接把字符串纳入「其它」输出
    /// * 📝【2024-03-27 10:34:02】下方`IoTranslators`无法换成`Self`
    ///   * `Self`意味着其带有类型约束
    /// * 📝【2024-03-27 10:37:37】不能直接使用裸露的闭包对象
    ///   * 每个闭包都有不同类型⇒必须强迫使用泛型
    ///   * 使用泛型⇒难以定义通用的[`Self::default`]方法
    fn default() -> IoTranslators {
        IoTranslators {
            input_translator: Box::new(|cmd| Ok(cmd.to_string())),
            output_translator: Box::new(|content| Ok(Output::OTHER { content })),
        }
    }
}

/// 从二元组转换
/// * 🎯用于后续参数传入[`IoTranslators`]时，可以用`impl Into<IoTranslators>`，并且仍允许类似位置参数的效果
///   * case: `fn set_translators(translators: impl Into<IoTranslators>)`
///     * call: `set_translators((in_translator, out_translator))`
///     * 📄[`super::super::CommandVm::translators`]
impl<I, O> From<(I, O)> for IoTranslators
where
    I: Fn(Cmd) -> Result<String> + Send + Sync + 'static,
    O: Fn(String) -> Result<Output> + Send + Sync + 'static,
{
    fn from(value: (I, O)) -> Self {
        Self::new(value.0, value.1)
    }
}

/// 错误类型
mod translate_error {
    use super::*;

    /// 统一封装「转译错误」
    /// * 🎯用于在[`anyhow`]下封装字符串，不再使用裸露的[`String`]类型
    /// * 🎯用于可识别的错误，并在打印时直接展示原因
    ///   * ⚠️若直接使用[`anyhow::anyhow`]，会打印一大堆错误堆栈
    #[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
    pub struct TranslateError(pub String);

    // ! ❌【2024-03-27 22:40:22】无法正常使用：不能导出带`format!`的宏
    // * error: macro-expanded `macro_export` macros from the current crate cannot be referred to by absolute paths
    // #[macro_export]
    // macro_rules! translate_error {
    //     ($($t:tt)*) => {
    //         TranslateError(format!($($t)*))
    //     };
    // }

    /// 灵活地从字符串转换为[`TranslateError`]
    impl<S: AsRef<str>> From<S> for TranslateError {
        fn from(value: S) -> Self {
            Self(value.as_ref().to_string())
        }
    }

    /// 灵活地从[`Error`]转换为[`TranslateError`]
    impl TranslateError {
        /// 从[`Error`]转换为[`TranslateError`]
        pub fn from_error(value: impl Error) -> Self {
            Self(value.to_string())
        }
        /// 从[`Error`]转换为[`anyhow::Error`]
        pub fn error_anyhow(value: impl Error) -> anyhow::Error {
            Self::from_error(value).into()
        }

        /// 从「一切可以转换为其自身的值」构建[`anyhow::Result`]
        pub fn err_anyhow<T, S>(from: S) -> anyhow::Result<T>
        where
            Self: From<S>,
        {
            Err(Self::from(from).into())
        }
        /// 从[`Self::from`]转换到[`anyhow::Error`]
        /// * 🚩封装为自身类型
        /// * ❗实际上`.into()`比`::anyhow`短
        ///   * 📌尽可能用前者
        pub fn anyhow(value: impl Into<Self>) -> anyhow::Error {
            // ! ❌【2024-03-27 22:59:51】不能使用`Self::from(value).into`：`AsRef<str>`不一定实现`Into<Self>`
            anyhow::Error::from(value.into())
        }
    }
    /// 展示错误
    impl Display for TranslateError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "TranslateError: {}", self.0)
        }
    }
    /// 实现[`Error`]特征
    impl Error for TranslateError {}
}
pub use translate_error::*;

/// 单元测试
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        // TODO: 【2024-03-27 22:56:26】有待完善
        let _t1 = IoTranslators::default();
    }
}
