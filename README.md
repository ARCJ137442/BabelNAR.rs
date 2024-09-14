# BabelNAR.rs

|**简体中文** | [English](README.en.md)|
|:-:|:-:|

    🏗️项目的**英文文档**尚在筹建，有意者欢迎提交PR

<!-- 徽章安排参考：https://daily.dev/blog/readme-badges-github-best-practices#organizing-badges-in-your-readme -->

![License](https://img.shields.io/crates/l/babel_nar?style=for-the-badge&color=ff7043)
![Code Size](https://img.shields.io/github/languages/code-size/ARCJ137442/BabelNAR.rs?style=for-the-badge&color=ff7043)
![Lines of Code](https://www.aschey.tech/tokei/github.com/ARCJ137442/BabelNAR.rs?style=for-the-badge&color=ff7043)
[![Language](https://img.shields.io/badge/language-Rust-orange?style=for-the-badge&color=ff7043)](https://www.rust-lang.org)

<!-- 面向用户 -->

Cargo状态：

[![crates.io](https://img.shields.io/crates/v/babel_nar?style=for-the-badge)](https://crates.io/crates/babel_nar)
[![docs.rs](https://img.shields.io/docsrs/babel_nar?style=for-the-badge)](https://docs.rs/babel_nar)
![Crate Size](https://img.shields.io/crates/size/babel_nar?style=for-the-badge)

![Recent Downloads](https://img.shields.io/crates/dr/babel_nar?style=for-the-badge)
![Downloads](https://img.shields.io/crates/d/babel_nar?style=for-the-badge)
![Crates.io Dependents](https://img.shields.io/crates/dependents/babel_nar?style=for-the-badge)

<!-- 面向开发者 -->

开发状态：

[![CI status](https://img.shields.io/github/actions/workflow/status/ARCJ137442/BabelNAR.rs/ci.yml?style=for-the-badge)](https://github.com/ARCJ137442/BabelNAR.rs/actions/workflows/ci.yml)
[![Conventional Commits](https://img.shields.io/badge/Conventional%20Commits-2.0.0-%23FE5196?style=for-the-badge)](https://conventionalcommits.org)
![GitHub commits since latest release](https://img.shields.io/github/commits-since/ARCJ137442/BabelNAR.rs/latest?style=for-the-badge)

![Created At](https://img.shields.io/github/created-at/ARCJ137442/BabelNAR.rs?style=for-the-badge)
![Last Commit](https://img.shields.io/github/last-commit/ARCJ137442/BabelNAR.rs?style=for-the-badge)

## 简介

[**NAVM.rs**](https://github.com/ARCJ137442/NAVM.rs)对[CIN](#cin-computer-implement-of-nars)的**启动器**、**运行时**及应用程序实现

- 前身为[**BabelNAR.jl**](https://github.com/ARCJ137442/BabelNAR.jl)
- ✨为「非公理虚拟机模型」提供程序实现
- ✨统一各[CIN](#cin-computer-implement-of-nars)的**输入输出**形式，聚合使用各大NARS实现
- ✨可由此进一步建立各类基于「NAVM模型」的工具
  - 📄命令行接口 [**BabelNAR-CLI**](https://github.com/ARCJ137442/BabelNAR-CLI.rs)

<!-- ## 安装 -->

<!-- * 📌【2024-04-10 10:19:40】有关具体环节，在crates.io中已经完善 -->

## 使用

🏗️TODO（接受贡献）

### CLI

参见[BabelNAR-CLI](https://github.com/ARCJ137442/BabelNAR-CLI.rs)

### 构建上游Rust项目

亦可参见[BabelNAR-CLI](https://github.com/ARCJ137442/BabelNAR-CLI.rs)（源码）

## 概念

### CIN (Computer Implement of NARS)

- 「NARS计算机实现」之英文缩写
- 指代所有**实现NARS**的计算机软件系统
  - 不要求完整实现NAL 1~9

### ***CommonNarsese***

🔗参考[**NAVM.jl**的对应部分](https://github.com/ARCJ137442/navm.jl?tab=readme-ov-file#commonnarsese)

## 各CIN对接情况

🕒最后更新时间：【2024-03-26 01:43:28】

| CIN         |    实现方法     | 进程安全 | 输入转译 | 输出转译 |
| :---------- | :---------: | :--: | :--: | :--: |
| OpenNARS    | `java -jar` |  ✅   |  ✅   |  🚧  |
| ONA         |   直接启动exe   |  ✅   |  ✅   |  🚧  |
| PyNARS      | `python -m` |  ✅   |  🚧  |  🚧  |
| NARS-Python |   直接启动exe   |  ❓   |  ✅  |  ❌  |
| OpenJunars  |   `julia`   |  ✅   |  ❌   |  ❌   |

注：

- 🚧输入输出转译功能仍然在从[BabelNAR_Implements](https://github.com/ARCJ137442/BabelNAR_Implements.jl)迁移
- ❓NARS-Python的exe界面可能会在终止后延时关闭
- ❌基于`julia`启动OpenJunars脚本`launch.jl`时，对「输出捕获」尚未有成功记录
- ❌目前对NARS-Python的「输出捕获」尚未有成功记录

## 参考

- [BabelNAR](https://github.com/ARCJ137442/BabelNAR.jl)
- [BabelNAR_Implements](https://github.com/ARCJ137442/BabelNAR_Implements.jl)
- [NAVM.rs](https://github.com/ARCJ137442/NAVM.rs)
