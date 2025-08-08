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

项目当前仍在起步阶段，欢迎提出建议或提交 PR。
