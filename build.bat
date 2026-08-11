@echo off
rem 云笈 - 构建脚本（生成发布版 EXE）
chcp 65001 >nul
cd /d "%~dp0"

echo [1/3] 构建前端 (React + TS + CSS)...
call npm --prefix ui install || goto :error
call npm --prefix ui run build || goto :error

echo [2/3] 构建桌面壳 (Tauri 2 + Rust)...
cd src-tauri
call cargo build --release || goto :error
cd ..

echo [3/3] 完成！
echo.
echo 可执行文件: src-tauri\target\release\CloudSatchel.exe
echo.
echo 如需生成安装包，请运行: cd src-tauri ^&^& cargo tauri build
pause
exit /b 0

:error
echo [错误] 构建失败。
pause
exit /b 1
