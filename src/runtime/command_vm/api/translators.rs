use navm::{cmd::Cmd, output::Output};
use util::ResultS;

/// [`Cmd`]→进程输入 转译器
/// * 🚩现在不再使用特征，以便在`Option<Box<InputTranslator>>`中推断类型
///   * 📝若给上边类型传入值`None`，编译器无法自动推导合适的类型
/// * 📌要求线程稳定
///   * 只有转译功能，没有其它涉及外部的操作（纯函数）
/// TODO: 在后续的「NSE指令输入」时，需要通过「自动将『空预算任务』作为语句输入」应对「`$$ A.`→`A.`」的情况
/// * ⚠️转译有可能失败：此时返回并上报错误信息
pub type InputTranslator = dyn Fn(Cmd) -> Result<String, String> + Send + Sync;

/// 进程输出→[`Output`]转译器
/// * 🚩现在不再使用特征，以便在`Option<Box<OutputTranslator>>`中推断类型
///   * 📝若给上边类型传入值`None`，编译器无法自动推导合适的类型
/// * 📌要求线程稳定
///   * 只有转译功能，没有其它涉及外部的操作（纯函数）
pub type OutputTranslator = dyn Fn(String) -> Result<Output, String> + Send + Sync;

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
        I: Fn(Cmd) -> ResultS<String> + Send + Sync + 'static,
        O: Fn(String) -> ResultS<Output> + Send + Sync + 'static,
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
    I: Fn(Cmd) -> ResultS<String> + Send + Sync + 'static,
    O: Fn(String) -> ResultS<Output> + Send + Sync + 'static,
{
    fn from(value: (I, O)) -> Self {
        Self::new(value.0, value.1)
    }
}

/// 单元测试
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let _t1 = IoTranslators::default();
    }
}
