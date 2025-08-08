@echo off
chcp 65001 >nul
REM ========================================
REM Auto Screen Switch 服务卸载脚本
REM ========================================

REM 检查管理员权限
net session >nul 2>&1
if errorlevel 1 (
    echo 错误：需要管理员权限才能卸载服务
    echo 请右键点击此脚本，选择"以管理员身份运行"
    pause
    exit /b 1
)

REM 设置服务名称
set SERVICE_NAME=AutoScreenSwitch

echo 正在卸载 Auto Screen Switch 服务...

REM 检查服务是否存在
sc query %SERVICE_NAME% >nul 2>&1
if errorlevel 1 (
    echo 错误：找不到服务 %SERVICE_NAME%
    echo 服务可能已经被卸载或不存在
    pause
    exit /b 1
)

REM 停止服务
echo 正在停止服务...
sc stop %SERVICE_NAME%
if errorlevel 1 (
    echo 警告：服务停止失败，可能服务已经停止
    echo 继续执行卸载...
)

REM 等待服务完全停止
timeout /t 3 /nobreak >nul

REM 删除服务
echo 正在删除服务...
sc delete %SERVICE_NAME%
if errorlevel 1 (
    echo 错误：服务删除失败
    echo 请确保以管理员权限运行此脚本
    pause
    exit /b 1
)

echo.
echo ========================================
echo 服务卸载成功！
echo 服务名称: %SERVICE_NAME%
echo ========================================
echo.
echo 注意：配置文件 config.toml 不会被删除
echo 如需完全清理，请手动删除相关文件
echo.
pause
