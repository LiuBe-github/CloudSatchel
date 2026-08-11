@echo off
rem 云笈 - 启动脚本（开发模式）
rem 需要 Node.js + Rust 工具链，首次运行请先执行 build.bat
chcp 65001 >nul
cd /d "%~dp0"

if not exist "ui\dist\index.html" (
    echo [提示] 未找到前端构建产物，正在构建...
    call npm --prefix ui run build || goto :error
)

if not exist "src-tauri\target\debug\CloudSatchel.exe" (
    echo [提示] 未找到 Rust 编译产物，正在编译（首次较慢）...
    cd src-tauri
    call cargo build || goto :error
    cd ..
)

start "" "src-tauri\target\debug\CloudSatchel.exe"
exit /b 0

:error
echo [错误] 构建失败，请检查 Node.js / Rust 环境。
pause
exit /b 1
