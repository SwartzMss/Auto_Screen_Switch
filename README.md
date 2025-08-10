# Auto_Screen_Switch

一个运行于 Windows 的小组件，通过 MQTT 监听 Raspberry Pi 5 发布的指令来控制电脑屏幕的开关。本项目使用 Rust 开发：当 Pi5 检测到有人时发送点亮指令，长时间无人时发送熄灭指令，该组件接收后执行相应操作。

## 功能特性

- ✅ 订阅 Pi5 发布的 MQTT 主题（`actuator/autoScreenSwitch`）
- ✅ 收到 `on` 指令时点亮屏幕，收到 `off` 指令时关闭屏幕
- ✅ 采用 Rust 实现，以系统托盘程序在后台运行
- ✅ 支持开机自启（托盘菜单可开关）
- ✅ 完善的错误处理和日志输出
- ✅ 详细的日志文件记录
- ✅ 单实例运行，优雅退出

## 项目结构

```
Auto_Screen_Switch/
├── src/
│   ├── main.rs          # 主程序入口，系统托盘 + MQTT 监听
│   ├── autostart.rs     # 开机自启（注册表）
│   ├── icon.rs          # 托盘图标生成
│   └── screen.rs        # 屏幕控制模块（Windows API）
├── config.toml          # 配置文件（如不存在将自动生成示例）
├── Cargo.toml           # Rust 项目配置
└── README.md            # 项目说明文档
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
   # 编辑配置文件（首次运行若不存在会自动生成默认 config.toml）
   notepad config.toml
   ```

### 运行方式

作为系统托盘程序运行：

```powershell
# 双击启动 auto_screen_switch.exe（或从命令行启动）
# 启动后系统托盘会出现 “Auto Screen Switch” 图标
```

- 首次运行若没有 `config.toml`，会在同目录自动生成默认配置；请按需修改后，在托盘菜单点击“启动 MQTT 连接”。
- 如需随 Windows 开机自启，可在托盘菜单点击“启用开机启动”（再次点击可关闭）。

### 配置文件

程序从同目录的 `config.toml` 读取 MQTT 连接信息（若不存在会在首次运行时自动生成默认文件）：

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
# 开启屏幕（有人检测）
mosquitto_pub -h 192.168.1.100 -t actuator/autoScreenSwitch -m '{"action":"on","params":{"source":"pir_motion"}}'

# 关闭屏幕（超时无人）
mosquitto_pub -h 192.168.1.100 -t actuator/autoScreenSwitch -m '{"action":"off","params":{"source":"idle_timeout"}}'
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

## 系统托盘与开机自启

- 启动 MQTT 连接：通过托盘菜单“启动 MQTT 连接”
- 停止 MQTT 连接：通过托盘菜单“停止 MQTT 连接”
- 开机自启：通过托盘菜单“启用/禁用开机启动”
- 退出程序：通过托盘菜单“退出”

## 故障排除

### 常见问题

1. **配置文件错误**
   - 确保 `config.toml` 文件存在且格式正确
   - 检查 MQTT Broker IP 和端口是否正确

2. **MQTT 连接失败**
   - 确认 MQTT Broker 正在运行
   - 检查网络连接
   - 验证用户名和密码（如果启用认证）

3. **程序未正常启动或托盘未显示**
   - 检查是否已存在正在运行的实例（本程序为单实例）
   - 检查是否被安全软件拦截或最小化到托盘
   - 查看日志文件定位原因

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

2. **检查 MQTT 是否已启动**：
   - 托盘菜单中“启动 MQTT 连接”按钮是否被禁用（被禁用表示已启动）
   - 确认 `config.toml` 中的 Broker 地址与端口可达

## 代码说明

### 主要模块

- **`main.rs`**：程序主入口，系统托盘、事件循环与 MQTT 客户端
- **`autostart.rs`**：开机自启开关（Windows 注册表）
- **`icon.rs`**：系统托盘图标生成
- **`screen.rs`**：屏幕控制模块，使用 Windows API 发送显示器电源控制消息

### 关键功能

1. **MQTT 监听**：订阅 `actuator/autoScreenSwitch` 主题，处理 JSON 格式的 `on`/`off` 指令
2. **屏幕控制**：通过 `SendMessageW` API 广播显示器电源控制消息
3. **系统托盘与自启**：托盘菜单控制 MQTT 启停，可切换开机自启
4. **错误处理**：完善的错误检查和日志输出

### 安全考虑

- 程序使用 Windows API 控制显示器，需要适当的权限
- MQTT 连接支持用户名/密码认证

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
