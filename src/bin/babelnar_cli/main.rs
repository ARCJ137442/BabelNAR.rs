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
    // 配置（自动）搜索
    use config_search;
    // 从配置启动
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
pub fn main_args(cwd: IoResult<PathBuf>, args: impl Iterator<Item = String>) -> Result<()> {
    // 解包当前工作目录
    let cwd = cwd
        .inspect_err(|e| println_cli!([Warn] "无法获取当前工作目录：{e}"))
        .ok();

    // （Windows下）启用终端颜色
    let _ = colored::control::set_virtual_terminal(true)
        .inspect_err(|_| eprintln_cli!([Error] "无法启动终端彩色显示。。"));

    // 解析命令行参数
    let args = CliArgs::parse_from(args);

    // 读取配置 | with 默认配置文件
    let mut config = load_config(&args);

    // 是否向用户展示「详细信息」 | 用于等待、提示等
    let user_verbose = config.user_input.is_none() || config.user_input.unwrap();

    // 用户填充配置项 | 需要用户输入、工作路径（🎯自动搜索）
    polyfill_config_from_user(&mut config, cwd);

    // 清屏，预备启动
    if user_verbose {
        println_cli!([Info] "配置加载完毕！程序将在1s后启动。。。");
        sleep(Duration::from_secs(1));
    }
    let _ = clearscreen::clear().inspect_err(|e| eprintln_cli!([Warn] "清屏失败：{e}"));

    // 从配置项启动 | 复制一个新配置，不会附带任何非基础类型开销
    let (runtime, config) = match launch_by_config(config.clone()) {
        // 启动成功⇒返回
        Ok((r, c)) => (r, c),
        // 启动失败⇒打印错误信息，等待并退出
        Err(e) => {
            println_cli!([Error] "NARS运行时启动错误：{e}");
            // 空配置/启用用户输入⇒延时提示
            if user_verbose {
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
#[allow(unused_attributes)]
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
            $(#[$attr_root:meta])*
            $cin_path:ident; // ! ❌若为`expr`，则会和上边的修饰符导致「本地歧义」
            $(
                $(#[$attr:meta])*
                $name:ident => $config_path:expr $(;)?
            )*
        ) => {
            /// 主Shell
            /// * 🎯正常BabelNAR CLI shell启动
            /// * 🎯正常用户命令行交互体验
            $(#[$attr_root])*
            #[test]
            #[ignore = "仅作试运行用，不用于自动化测试"]
            pub fn main_shell() -> Result<()> {
                main($cin_path, &[])
            }


            /// Matriangle服务器
            /// * 🎯复现先前基于Matriangle环境的NARS实验
            $(#[$attr_root])*
            #[test]
            #[ignore = "仅作试运行用，不用于自动化测试"]
            pub fn main_matriangle_server() -> Result<()> {
                // 以默认参数启动
                main_configs($cin_path, &[MATRIANGLE_SERVER])
            }

            $(
                $(#[$attr])*
                #[test]
                #[ignore = "【2024-06-12 23:52:35】不用于自动化测试：会自动清屏影响测试结果呈现"]
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
            /// * 📝✅【2024-04-19 22:49:51】成功
            nal_de => NAL_SIMPLE_DEDUCTION

            /// 高阶演绎
            /// * 📝✅【2024-04-19 22:50:04】成功
            nal_hi => NAL_HIGHER_DEDUCTION

            /// 自变量消除
            /// * 📝✅【2024-04-19 22:50:53】成功
            nal_ie => NAL_I_VAR_ELIMINATION

            /// 时间归纳
            /// * 📝✅【2024-04-19 22:52:45】成功
            nal_te => NAL_TEMPORAL_INDUCTION

            /// 简单操作
            /// * 📝❌【2024-04-19 22:55:35】失败：推理不出任何内容
            nal_so => NAL_SIMPLE_OPERATION

            /// 操作
            /// * 📝✅【2024-04-19 23:01:52】成功，但少许问题
            ///   * 📝【2024-04-07 14:17:21】目前ONA面对其中的「经验问句」没有回答
            ///   * ⚠️在启用`REG left`注册操作后，反而从成功变为失败
            nal_op => NAL_OPERATION
        }
    }

    /// 测试/OpenNARS (3.x)
    mod opennars {
        use super::*;

        cin_tests! {
            #[ignore = "【2024-04-14 20:24:52】会导致残留子进程"]
            OPENNARS;

            /// 简单演绎
            /// * 📝✅【2024-04-19 22:49:02】成功（步数性能上不佳）
            nal_de => NAL_SIMPLE_DEDUCTION

            /// 高阶演绎
            /// * 📝✅【2024-04-19 22:48:56】成功
            nal_hi => NAL_HIGHER_DEDUCTION

            /// 自变量消除
            /// * 📝✅【2024-04-07 16:01:15】成功
            nal_ie => NAL_I_VAR_ELIMINATION

            /// 时间归纳
            /// * 📝✅【2024-04-19 22:52:35】成功（步数性能上不佳）
            nal_te => NAL_TEMPORAL_INDUCTION

            /// 简单操作
            /// * 📝✅【2024-04-07 16:13:39】成功
            nal_so => NAL_SIMPLE_OPERATION

            /// 操作
            /// * 📝✅【2024-04-07 14:59:53】成功
            nal_op => NAL_OPERATION
        }
    }

    /// 测试/OpenNARS (1.5.8)
    mod opennars158 {
        use super::*;

        cin_tests! {
            #[ignore = "【2024-04-14 20:24:52】会导致残留子进程"]
            OPENNARS_158;

            /// 简单演绎
            /// * 📝✅【2024-04-19 23:02:59】成功
            nal_de => NAL_SIMPLE_DEDUCTION

            /// 高阶演绎
            /// * 📝✅【2024-04-19 23:03:06】成功
            nal_hi => NAL_HIGHER_DEDUCTION

            /// 自变量消除
            /// * 📝✅【2024-04-19 23:03:15】成功
            nal_ie => NAL_I_VAR_ELIMINATION

            /// 时间归纳
            /// * 📝❌【2024-04-19 23:03:20】失败：语法层面就不支持
            nal_te => NAL_TEMPORAL_INDUCTION

            /// 简单操作
            /// * 📝❌【2024-04-19 23:03:37】失败：语法层面就不支持
            nal_so => NAL_SIMPLE_OPERATION

            /// 操作
            /// * 📝❌【2024-04-19 23:03:48】失败：语法层面就不支持
            nal_op => NAL_OPERATION
        }
    }

    /// 测试/PyNARS
    mod pynars {
        use super::*;

        cin_tests! {
            PYNARS;

            /// 简单演绎
            /// * 📝✅【2024-04-19 23:04:24】成功
            nal_de => NAL_SIMPLE_DEDUCTION

            /// 高阶演绎
            /// * 📝✅【2024-04-19 23:04:33】成功
            nal_hi => NAL_HIGHER_DEDUCTION

            /// 自变量消除
            /// * 📝❌【2024-04-19 23:05:32】失败：啥推理都没有
            nal_ie => NAL_I_VAR_ELIMINATION

            /// 时间归纳
            /// * 📝❌【2024-04-19 23:06:43】失败：只会回答`<C-->D>. :\: %1.000;0.900%`
            nal_te => NAL_TEMPORAL_INDUCTION

            /// 简单操作
            /// * 📝❌【2024-04-19 23:06:48】失败：没有任何回答
            nal_so => NAL_SIMPLE_OPERATION

            /// 操作
            /// * 📝❌【2024-04-19 23:07:11】目前仍测试失败
            ///   * 📄【2024-04-19 23:07:27】只会回答`ANSWER:<{SELF}-->(/, ^left, _)>. :\: %1.000;0.900%`
            ///   * 📌PyNARS自身对NAL-7、NAL-8支持尚不完善
            ///   * 📌PyNARS中操作`left`并非默认已注册
            ///     * ❌【2024-04-07 14:41:54】补充：追加了也不行
            nal_op => NAL_OPERATION
        }
    }

    /// 测试/CXinJS
    /// * 📝【2024-04-19 23:10:28】用来试探「自动测试脚本」的下限
    mod cxin_js {
        use super::*;

        cin_tests! {
            CXIN_JS;

            /// 简单演绎
            /// * 📝❌【2024-04-07 14:37:49】失败：导出了结论，但没法回答
            /// * 📄只能导出`<B-->C>. %1;0.9%`
            nal_de => NAL_SIMPLE_DEDUCTION

            /// 高阶演绎
            /// * 📝❌【2024-04-19 23:08:44】失败：只能导出到`<A-->B>?`
            ///   * 📌即便是五百步，也推不出来
            nal_hi => NAL_HIGHER_DEDUCTION

            /// 自变量消除
            /// * 📝❌【2024-04-19 23:09:21】失败：仅推理到`<A-->C>?`，并且遇到「XXX is not a function」错误
            nal_ie => NAL_I_VAR_ELIMINATION

            /// 时间归纳
            /// * 📝❌【2024-04-19 23:09:34】失败：解析即报错——不支持`=/>`
            nal_te => NAL_TEMPORAL_INDUCTION

            /// 简单操作
            /// * 📝❌【2024-04-19 23:09:47】失败：推理不出任何内容
            ///   * 💭还会把「目标」解析成「判断」……
            nal_so => NAL_SIMPLE_OPERATION

            /// 操作
            /// * 📝❌【2024-04-19 23:10:21】失败：自身就不支持
            nal_op => NAL_OPERATION
        }
    }

    /// 测试/原生IL-1
    mod native_il_1 {
        use super::*;

        cin_tests! {
            NATIVE_IL_1;

            /// 简单演绎
            /// * 📝✅【2024-04-09 21:12:10】成功
            nal_de => NAL_SIMPLE_DEDUCTION

            /// 高阶演绎
            /// * 📝❌【2024-04-09 21:12:32】失败：尚不支持
            nal_hi => NAL_HIGHER_DEDUCTION

            /// 自变量消除
            /// * 📝❌【2024-04-09 21:12:32】失败：尚不支持
            nal_ie => NAL_I_VAR_ELIMINATION

            /// 时间归纳
            /// * 📝❌【2024-04-09 21:12:32】失败：尚不支持
            nal_te => NAL_TEMPORAL_INDUCTION

            /// 简单操作
            /// * 📝❌【2024-04-09 21:12:32】失败：尚不支持
            nal_so => NAL_SIMPLE_OPERATION

            /// 操作
            /// * 📝❌【2024-04-09 21:12:32】失败：尚不支持
            nal_op => NAL_OPERATION
        }
    }

    // ! ❌【2024-04-07 14:39:20】接口完成度不高的NARS-Python、OpenJunars暂不进行测试

    /// 测试入口/带Websocket Shell
    /// * 🎯正常BabelNAR CLI shell启动
    /// * 🎯用户命令行交互体验（并存）
    /// * 🎯Websocket通信
    #[test]
    #[ignore = "仅作试运行用，不用于自动化测试"]
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
