@REM BabelNAR CLI 轻量级发行版
@REM * 将BabelNAR CLI、预设好的可执行文件和指定CIN集成在一个目录下
@REM * 可实现「一次打包，到处解压，双击运行」
@echo off

@REM 构建BabelNAR CLI（基于cargo）
cargo b -r --bin babelnar_cli

@REM 重置dist文件夹
rm -rf dist
mkdir .\dist

@REM ==拷贝各个文件到指定目录== REM@

@REM 公共配置
xcopy /E /I .\config_public .\dist\nars_config

@REM 拷贝BabelNAR CLI
copy .\target\release\babelnar_cli.exe .\dist

@REM 拷贝指定的可执行文件
xcopy /E /I .\executables\PyNARS .\dist\executables\PyNARS
copy .\executables\cxin-nars-shell.js .\dist\executables\cxin-nars-shell.js
copy .\executables\native-IL-1.exe .\dist\executables\native-IL-1.exe
copy .\executables\ONA.exe .\dist\executables\ONA.exe
copy .\executables\opennars-158-shell.jar .\dist\executables\opennars-158-shell.jar
copy .\executables\opennars-304-T-modified.jar .\dist\executables\opennars-304-T-modified.jar
copy .\executables\opennars-matriangle-test.log.json .\dist\executables\opennars-matriangle-test.log.json

echo Build Successfull!
sleep 1
