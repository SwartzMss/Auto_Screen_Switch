# Auto_Screen_Switch

一个运行于 Windows 的小组件，通过 MQTT 监听 Raspberry Pi 5 发布的指令来控制电脑屏幕的开关。本项目使用 Rust 开发：当 Pi5 检测到有人时发送点亮指令，长时间无人时发送熄灭指令，该组件接收后执行相应操作。

## 功能特性

- ✅ 订阅 Pi5 发布的 MQTT 主题（`pi5/display`）
- ✅ 收到 `on` 指令时点亮屏幕，收到 `off` 指令时关闭屏幕
- ✅ 采用 Rust 实现，可作为 Windows 后台服务运行
- ✅ 支持 CLI 模式调试
- ✅ 完善的错误处理和日志输出
- ✅ 详细的日志文件记录
- ✅ 优雅的服务启动和关闭

## 项目结构

```
Auto_Screen_Switch/
├── src/
│   ├── main.rs          # 主程序入口，包含 MQTT 监听和服务管理
│   └── screen.rs        # 屏幕控制模块，使用 Windows API
├── config.toml.example  # 配置文件示例
├── install.bat          # 服务安装脚本
├── uninstall.bat        # 服务卸载脚本
├── Cargo.toml           # Rust 项目配置
└── README.md           # 项目说明文档
```

## 开发与运行

### 环境要求

1. 安装 Rust（推荐使用 `rustup`）
2. 确保网络中运行着 MQTT Broker（如 Mosquitto）
3. Windows 操作系统

### 构建步骤

1. 克隆本仓库：
   ```bash
   git clone https://github.com/yourname/Auto_Screen_Switch.git
   cd Auto_Screen_Switch
   ```

2. 构建项目：
   ```bash
   cargo build --release
   ```

3. 配置 MQTT 连接：
   ```bash
   # 复制配置文件示例
   copy config.toml.example config.toml
   # 编辑配置文件，设置你的 MQTT Broker 信息
   notepad config.toml
   ```

### 运行模式

#### CLI 模式（调试推荐）

用于开发和调试，可以直接观察程序输出：

```powershell
# 以 CLI 模式运行
auto_screen_switch.exe --mode cli
```

#### 服务模式（生产环境）

作为 Windows 服务在后台运行：

```powershell
# 以管理员权限运行安装脚本
install.bat

# 或手动安装服务
sc create AutoScreenSwitch binPath= "C:\path\to\auto_screen_switch.exe" start= auto
sc start AutoScreenSwitch
```

### 配置文件

程序从 `config.toml` 读取 MQTT 连接信息：

```toml
# MQTT Broker 的 IP 地址
broker_ip = "192.168.1.100"

# MQTT Broker 的端口号（通常为 1883）
broker_port = 1883

# MQTT 用户名（可选）
username = "your_username"

# MQTT 密码（可选）
password = "your_password"
```

## 测试方法

### 使用 MQTT 客户端测试

1. 确保 MQTT Broker 正在运行
2. 使用 `mosquitto_pub` 发送测试消息：

```powershell
# 开启屏幕
mosquitto_pub -h 192.168.1.100 -t pi5/display -m on

# 关闭屏幕
mosquitto_pub -h 192.168.1.100 -t pi5/display -m off
```

### 观察程序输出

在 CLI 模式下，你应该能看到类似输出：

```
以 CLI 模式启动...
正在连接到 MQTT Broker: 192.168.1.100:1883
已订阅主题: pi5/display，等待控制指令...
收到开启屏幕指令
已发送开启屏幕指令
收到关闭屏幕指令
已发送关闭屏幕指令
```

## 服务管理

### 安装服务

```powershell
# 以管理员权限运行
install.bat
```

### 卸载服务

```powershell
# 以管理员权限运行
uninstall.bat
```

### 手动管理服务

```powershell
# 查看服务状态
sc query AutoScreenSwitch

# 启动服务
sc start AutoScreenSwitch

# 停止服务
sc stop AutoScreenSwitch

# 删除服务
sc delete AutoScreenSwitch
```

## 故障排除

### 常见问题

1. **配置文件错误**
   - 确保 `config.toml` 文件存在且格式正确
   - 检查 MQTT Broker IP 和端口是否正确

2. **MQTT 连接失败**
   - 确认 MQTT Broker 正在运行
   - 检查网络连接
   - 验证用户名和密码（如果启用认证）

3. **服务启动失败**
   - 确保以管理员权限运行安装脚本
   - 检查可执行文件路径是否正确
   - 查看 Windows 事件日志获取详细错误信息

4. **屏幕控制无效**
   - 确认程序以管理员权限运行
   - 检查显示器电源管理设置
   - 某些显示器可能不支持软件控制

### 调试技巧

1. **查看日志文件**：
   ```cmd
   # 日志文件位置（与可执行文件同目录）
   auto_screen_switch.log
   
   # 实时查看日志
   type auto_screen_switch.log
   
   # 查看最后几行日志
   tail -n 20 auto_screen_switch.log
   ```

2. **检查服务状态**：
   ```cmd
   sc query AutoScreenSwitch
   ```

3. **查看服务日志**：
   - 打开 Windows 事件查看器
   - 查看应用程序日志
   - 或直接查看 auto_screen_switch.log 文件

## 代码说明

### 主要模块

- **`main.rs`**：程序主入口，包含 MQTT 客户端、服务管理和事件循环
- **`screen.rs`**：屏幕控制模块，使用 Windows API 发送显示器电源控制消息

### 关键功能

1. **MQTT 监听**：订阅 `pi5/display` 主题，处理 `on`/`off` 指令
2. **屏幕控制**：通过 `SendMessageW` API 广播显示器电源控制消息
3. **服务管理**：支持作为 Windows 服务运行，支持优雅关闭
4. **错误处理**：完善的错误检查和日志输出

### 安全考虑

- 程序使用 Windows API 控制显示器，需要适当的权限
- MQTT 连接支持用户名/密码认证
- 服务模式运行提供更好的安全性和稳定性

## 贡献指南

欢迎提交 Issue 和 Pull Request！

### 开发环境设置

1. 安装 Rust 工具链
2. 克隆仓库
3. 运行 `cargo build` 构建项目
4. 使用 `cargo test` 运行测试

### 代码规范

- 遵循 Rust 编码规范
- 添加适当的中文注释
- 使用 Conventional Commits 格式提交代码

## 许可证

本项目采用 MIT 许可证，详见 [LICENSE](LICENSE) 文件。
