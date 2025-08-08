@echo off
REM ========================================
REM Auto Screen Switch 服务安装脚本
REM ========================================

REM 设置服务名称和可执行文件路径
set SERVICE_NAME=AutoScreenSwitch
set EXE_PATH=%~dp0auto_screen_switch.exe

echo 正在安装 Auto Screen Switch 服务...

REM 检查可执行文件是否存在
if not exist "%EXE_PATH%" (
    echo 错误：找不到 auto_screen_switch.exe 文件
    echo 请确保可执行文件位于脚本同一目录下
    pause
    exit /b 1
)

REM 检查配置文件是否存在
if not exist "%~dp0config.toml" (
    echo 警告：找不到 config.toml 配置文件
    echo 请复制 config.toml.example 为 config.toml 并配置 MQTT 连接信息
    echo.
)

REM 创建 Windows 服务
echo 正在创建服务...
sc create %SERVICE_NAME% binPath= "%EXE_PATH%" start= auto
if errorlevel 1 (
    echo 错误：服务创建失败
    echo 请确保以管理员权限运行此脚本
    pause
    exit /b 1
)

REM 启动服务
echo 正在启动服务...
sc start %SERVICE_NAME%
if errorlevel 1 (
    echo 错误：服务启动失败
    echo 请检查配置文件是否正确
    pause
    exit /b 1
)

echo.
echo ========================================
echo 服务安装成功！
echo 服务名称: %SERVICE_NAME%
echo 可执行文件: %EXE_PATH%
echo ========================================
echo.
echo 服务管理命令：
echo   sc start %SERVICE_NAME%    - 启动服务
echo   sc stop %SERVICE_NAME%     - 停止服务
echo   sc query %SERVICE_NAME%    - 查看服务状态
echo.
pause
