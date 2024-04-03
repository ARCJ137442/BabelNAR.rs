# BabelNAR.rs

[English](README.md) | 简体中文

[![Conventional Commits](https://img.shields.io/badge/Conventional%20Commits-1.0.0-%23FE5196?logo=conventionalcommits&logoColor=white)](https://conventionalcommits.org)

该项目使用[语义化版本 2.0.0](https://semver.org/)进行版本号管理。

[**NAVM.rs**](https://github.com/ARCJ137442/NAVM.rs)对[CIN](#cin-computer-implement-of-nars)的**启动器**、**运行时**及应用程序实现

- 前身为[**BabelNAR.jl**](https://github.com/ARCJ137442/BabelNAR.jl)
- ✨为「非公理虚拟机模型」提供程序实现
- ✨统一各[CIN](#cin-computer-implement-of-nars)的**输入输出**形式，聚合使用各大NARS实现

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
