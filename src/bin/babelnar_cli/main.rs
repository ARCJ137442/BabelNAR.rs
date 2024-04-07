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
    use vm_config;
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
    let mut config = load_config(&args);
    // 用户填充配置项
    polyfill_config_from_user(&mut config);
    // 从配置项启动 | 复制一个新配置，不会附带任何非基础类型开销
    let (runtime, config) = match launch_by_config(config.clone()) {
        // 启动成功⇒返回
        Ok((r, c)) => (r, c),
        // 启动失败⇒打印错误信息，等待并退出
        Err(e) => {
            println_cli!([Error] "NARS运行时启动错误：{e}");
            // 启用用户输入时延时提示
            if let Some(true) = config.user_input {
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
    use babel_nar::tests::config_paths::*;
    use nar_dev_utils::list;

    /// 测试入口/ONA
    /// * 🎯通用、可复用的启动代码
    ///   * 🎯跨不同CIN通用
    ///   * 🎯跨同CIN不同测试通用
    pub fn main(cin_config_path: &str, other_args: &[&str]) -> Result<()> {
        babel_nar::exists_or_exit!("./executables");
        // 以默认参数启动
        main_args(
            env::current_dir(),
            [
                &["BabelNAR-cli.exe", "-d", "-c", cin_config_path],
                other_args,
            ]
            .concat()
            .into_iter()
            .map(str::to_string),
        )
    }

    /// 测试入口/多配置加载
    /// * 🎯多「虚拟机启动配置」合并
    /// * 🎯预引入NAL
    pub fn main_configs(cin_config_path: &str, other_config_paths: &[&str]) -> Result<()> {
        let args = list![
            [
                // 第二个文件，搭建测试环境
                "-c",
                config_path,
                // 第三个文件，指示预加载
                "-c",
                config_path,
            ]
            for config_path in (other_config_paths)
        ]
        .concat();
        main(cin_config_path, &args)
    }

    /// 批量生成「预引入NAL」
    macro_rules! cin_tests {
        (
            $cin_path:expr;
            $(
                $(#[$attr:meta])*
                $name:ident => $config_path:expr $(;)?
            )*
        ) => {
            /// 主Shell
            /// * 🎯正常BabelNAR CLI shell启动
            /// * 🎯正常用户命令行交互体验
            #[test]
            pub fn main_shell() -> Result<()> {
                main($cin_path, &[])
            }

            $(
                $(#[$attr])*
                #[test]
                pub fn $name() -> Result<()> {
                    main_configs($cin_path, &[PRELUDE_TEST, $config_path])
                }
            )*
        };
    }

    /// 测试/ONA
    mod ona {
        use super::*;

        cin_tests! {
            ONA;

            /// 简单演绎
            /// * 📝✅【2024-04-07 14:56:04】成功
            nal_de => NAL_SIMPLE_DEDUCTION

            /// 高阶演绎
            /// * 📝✅【2024-04-07 14:56:04】成功
            nal_hi => NAL_HIGHER_DEDUCTION

            /// 自变量消除
            /// * 📝✅【2024-04-07 16:03:47】成功
            nal_ie => NAL_I_VAR_ELIMINATION

            /// 时间归纳
            /// * 📝✅【2024-04-07 15:22:28】成功
            nal_te => NAL_TEMPORAL_INDUCTION

            /// 简单操作
            /// * 📝❌【2024-04-07 16:15:53】失败：推理不出任何内容
            nal_so => NAL_SIMPLE_OPERATION

            /// 操作
            /// * 📝✅【2024-04-07 14:57:50】成功，但少许问题
            ///   * 📝【2024-04-07 14:17:21】目前ONA面对其中的「经验问句」没有回答
            ///   * ⚠️在启用`REG left`注册操作后，反而从成功变为失败
            nal_op => NAL_OPERATION
        }
    }

    /// 测试/OpenNARS
    mod opennars {
        use super::*;

        cin_tests! {
            OPENNARS;

            /// 简单演绎
            /// * 📝✅【2024-04-07 14:59:37】成功
            nal_de => NAL_SIMPLE_DEDUCTION

            /// 高阶演绎
            /// * 📝✅【2024-04-07 14:59:44】成功
            nal_hi => NAL_HIGHER_DEDUCTION

            /// 自变量消除
            /// * 📝✅【2024-04-07 16:01:15】成功
            nal_ie => NAL_I_VAR_ELIMINATION

            /// 时间归纳
            /// * 📝✅【2024-04-07 15:22:28】成功
            nal_te => NAL_TEMPORAL_INDUCTION

            /// 简单操作
            /// * 📝✅【2024-04-07 16:13:39】成功
            nal_so => NAL_SIMPLE_OPERATION

            /// 操作
            /// * 📝✅【2024-04-07 14:59:53】成功
            nal_op => NAL_OPERATION
        }
    }

    /// 测试/PyNARS
    mod pynars {
        use super::*;

        cin_tests! {
            PYNARS;

            /// 简单演绎
            nal_de => NAL_SIMPLE_DEDUCTION

            /// 高阶演绎
            nal_hi => NAL_HIGHER_DEDUCTION

            /// 自变量消除
            /// * 📝❌【2024-04-07 16:01:15】失败：啥推理都没有
            nal_ie => NAL_I_VAR_ELIMINATION

            /// 时间归纳
            /// * 📝❌【2024-04-07 16:13:52】失败：只会回答`<C-->D>. :\: %1.000;0.900%`
            nal_te => NAL_TEMPORAL_INDUCTION

            /// 简单操作
            /// * 📝❌【2024-04-07 16:13:42】失败：没有任何回答
            nal_so => NAL_SIMPLE_OPERATION

            /// 操作
            /// * 📝❌【2024-04-07 14:39:49】目前仍测试失败
            ///   * 📌PyNARS自身对NAL-7、NAL-8支持尚不完善
            ///   * 📌PyNARS中操作`left`并非默认已注册
            ///     * ❌【2024-04-07 14:41:54】补充：追加了也不行
            nal_op => NAL_OPERATION
        }
    }

    /// 测试/CXinJS
    mod cxin_js {
        use super::*;

        cin_tests! {
            CXIN_JS;

            /// 简单演绎
            /// * 📝❌【2024-04-07 14:37:49】失败：导出了结论，但没法回答
            nal_de => NAL_SIMPLE_DEDUCTION

            /// 高阶演绎
            /// * 📝❌【2024-04-07 14:37:49】失败：只能导出到`<A-->B>?`
            ///   * 📌即便是五百步，也推不出来
            nal_hi => NAL_HIGHER_DEDUCTION

            /// 自变量消除
            /// * 📝❌【2024-04-07 16:01:15】失败：仅推理到`<A-->C>?`，并且遇到「XXX is not a function」错误
            nal_ie => NAL_I_VAR_ELIMINATION

            /// 时间归纳
            /// * 📝❌失败：解析即报错——不支持`=/>`
            nal_te => NAL_TEMPORAL_INDUCTION

            /// 简单操作
            /// * 📝❌【2024-04-07 16:16:24】失败：推理不出任何内容
            ///   * 💭还会把「目标」解析成「判断」……
            nal_so => NAL_SIMPLE_OPERATION

            /// 操作
            /// * 📝❌目前仍测试失败
            ///   * 📌PyNARS自身对NAL-7、NAL-8支持尚不完善
            ///   * 📌PyNARS中操作`left`并非默认已注册
            /// * 📝❌【2024-04-07 14:37:49】失败：自身就不支持
            nal_op => NAL_OPERATION
        }
    }

    // ! ❌【2024-04-07 14:39:20】接口完成度不高的NARS-Python、OpenJunars暂不进行测试

    /// 测试入口/带Websocket Shell
    /// * 🎯正常BabelNAR CLI shell启动
    /// * 🎯用户命令行交互体验（并存）
    /// * 🎯Websocket通信
    #[test]
    pub fn main_websocket() -> Result<()> {
        // 以默认参数启动
        main_args(
            env::current_dir(),
            ["test.exe", "-d", "-c", ONA, "-c", WEBSOCKET]
                .into_iter()
                .map(str::to_string),
        )
    }
}
