//! 一个一站式启动各CIN的启动器
//! * 🎯方便启动、管理各「作为NAVM运行时的CIN」的聚合终端
//! * 📌用于集成原先「BabelNAR」「BabelNAR_Implements」两个库
//! * ✨自动根据可执行文件、配置文件、用户输入猜测CIN类型（字符串匹配）
//! * ✨自动查找（可能）可用的CIN可执行文件（文件搜索）
//!   * 📌可根据「匹配度」排名
//! * ✨自动启动并管理CIN
//!   * 📌可保存/加载「常用CIN」配置
//!
//! * 🚩目前用于敏捷原型开发
#![allow(unused)]

use anyhow::Result;
use babel_nar::{
    cin_implements::{ona::ONA, opennars::OpenNARS, pynars::PyNARS},
    eprintln_cli, println_cli,
    runtimes::CommandVmRuntime,
    tests::cin_paths::{ONA, OPENNARS, PYNARS_ROOT},
};
use nar_dev_utils::*;
use navm::{
    cmd::Cmd,
    output::Output,
    vm::{VmLauncher, VmRuntime},
};
use std::{fmt::Debug, io::stdin};

const TEST_PATH_OPENNARS: &str = OPENNARS;
const TEST_PATH_ONA: &str = ONA;
const TEST_PATH_PYNARS: (&str, &str) = (PYNARS_ROOT, "pynars.ConsolePlus");

/// 启动并获取NARS
/// * 🚩【2024-03-27 18:55:07】目前就返回一个测试用的运行时
/// * 🎯敏捷开发用
fn get_nars() -> impl VmLauncher<CommandVmRuntime> {
    // OpenNARS::new(TEST_PATH_OPENNARS)
    PyNARS::new(TEST_PATH_PYNARS.0, TEST_PATH_PYNARS.1)
    // ONA::new(TEST_PATH_ONA)
}

fn put_cmd_to_nars(nars: &mut impl VmRuntime, cmd: Cmd) -> Result<()> {
    nars.input_cmd(cmd)
}

/// 主函数
/// * 🚩【2024-04-02 20:58:07】现在更完整的支持交给BabelNAR CLI，此文件用于敏捷开发
fn main() {
    // 不断开始🔥
    loop {
        start();
    }
}

/// 开始
fn start() {
    let nars = get_nars().launch().expect("无法启动虚拟机");
    shell(nars);
}

/// 打印错误
fn println_error(e: &impl Debug) {
    println!("{e:?}");
}

/// 交互式命令行
fn shell(mut nars: CommandVmRuntime) {
    let stdin = stdin();
    let mut input = String::new();
    let mut line;

    'main: while stdin.read_line(&mut input).is_ok() {
        // 一行
        line = input.as_str();

        // 非空⇒解析出NAVM指令，作为输入执行
        if !line.trim().is_empty() {
            if let Ok(cmd) = Cmd::parse(line)
                .inspect_err(|e| eprintln_cli!([Error] "解析NAVM指令时发生错误：{e}"))
            {
                let _ = put_cmd_to_nars(&mut nars, cmd)
                    .inspect_err(|e| eprintln_cli!([Error] "执行NAVM指令时发生错误：{e}"));
            }
        }

        // 尝试拉取所有NAVM运行时输出
        while let Ok(Some(output)) = nars
            .try_fetch_output()
            .inspect_err(|e| eprintln_cli!([Error] "拉取NAVM运行时输出时发生错误：{e}"))
        {
            println!("{output:?}");
            if let Output::TERMINATED { description } = output {
                println_cli!([Info] "NAVM已终止运行：{description}");
                nars.terminate();
                break 'main; // ! 这个告诉Rust编译器，循环必将在此结束
            }
        }

        // 清空缓冲区
        input.clear();
    }
}

/// 单元测试
#[cfg(test)]
mod tests {
    use super::*;
    use babel_nar::cin_implements::cxin_js::CXinJS;
    use babel_nar::cin_implements::pynars::PyNARS;
    use narsese::conversion::string::impl_lexical::format_instances::FORMAT_ASCII;
    use navm::cmd::Cmd;
    use navm::vm::VmLauncher;

    fn test_set(mut nars: impl VmRuntime, test_set: Vec<Cmd>) {
        for cmd in test_set {
            nars.input_cmd(cmd);
        }
        std::thread::sleep(std::time::Duration::from_secs(5));
        while let Ok(Some(o)) = nars.try_fetch_output() {
            println!("{}", format_navm_output(o));
        }
    }

    fn format_navm_output(o: Output) -> String {
        // 以「有无Narsese」作区分
        match o.get_narsese() {
            // * 🚩有Narsese⇒包含Narsese
            Some(nse) => format!(
                "[{}] (( {} )) {}",
                o.type_name(),
                FORMAT_ASCII.format_narsese(nse),
                o.raw_content()
            ),
            // * 🚩无⇒仅包含内容
            None => format!("[{}] {}", o.type_name(), o.raw_content()),
        }
    }

    fn parse_cmd_lines(narsese: impl AsRef<str>) -> Vec<Cmd> {
        let narsese = narsese.as_ref();
        let mut result = vec![];

        for line in narsese.split('\n').map(str::trim).filter(|s| !s.is_empty()) {
            match Cmd::parse(line) {
                Ok(cmd) => result.push(cmd),
                Err(e) => println!("{e}"),
            }
        }

        result
    }
}
