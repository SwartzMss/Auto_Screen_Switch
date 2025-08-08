# Auto_Screen_Switch

一个运行于 Windows 的小组件，通过 MQTT 监听 Raspberry Pi 5 发布的指令来控制电脑屏幕的开关。本项目计划使用 Rust 开发：当 Pi5 检测到有人时发送点亮指令，长时间无人时发送熄灭指令，该组件接收后执行相应操作。

## 功能设想

- 订阅 Pi5 发布的 MQTT 主题（例如 `pi5/display`）。
- 收到 `on` 指令时点亮屏幕，长时间无人在场或收到 `off` 指令时关闭屏幕。
- 采用 Rust 实现，可作为 Windows 后台服务运行。

## 开发与运行

1. 安装 Rust（推荐使用 `rustup`）。
2. 确保网络中运行着 MQTT Broker（如 Mosquitto）。
3. 克隆本仓库并构建：
   ```bash
   git clone https://github.com/yourname/Auto_Screen_Switch.git
   cd Auto_Screen_Switch
   cargo build --release
   ```
4. 在 Windows 上运行程序并设置 MQTT 连接参数等配置（待实现）。

5. 测试：在 Windows 上使用 MQTT 客户端手动发布消息验证效果，例如使用 `mosquitto_pub`：
   ```powershell
   mosquitto_pub -h <broker_address> -t pi5/display -m on
   mosquitto_pub -h <broker_address> -t pi5/display -m off
   ```
   观察屏幕是否根据指令点亮或关闭。

项目当前仍在起步阶段，欢迎提出建议或提交 PR。

## 安装与调试

### 运行模式

- **系统服务模式（默认）**：直接执行可执行文件会尝试在 Windows 中安装为系统服务，并在后台运行。该模式适合长期驻留使用。
- **CLI 模式**：通过 `--mode cli` 参数以前台方式运行程序，仍从配置文件读取 MQTT 参数，适合调试。


### 配置文件

程序默认从可执行文件所在目录的 `config.toml` 读取 MQTT 连接信息，其中包含 Broker 的 IP、端口以及可选的用户名和密码。例如：

```toml
broker_ip = "192.168.1.10"
broker_port = 1883
username = "user"
password = "pass"
```

无论以系统服务模式还是 CLI 模式运行，程序都会从配置文件读取上述参数。目前命令行仅支持通过 `--mode cli` 选择前台运行模式。

示例命令：

```powershell
auto_screen_switch.exe --mode cli
```

### 调试建议

- 在 CLI 模式下可以直接观察终端输出，检查是否成功连接到 MQTT Broker。
- 如需查看更多日志，可按需设置诸如 `RUST_LOG=debug` 的环境变量（预期功能）。
- 若服务模式运行异常，可使用 `auto_screen_switch.exe --uninstall` 移除服务后重新安装（预期功能）。
