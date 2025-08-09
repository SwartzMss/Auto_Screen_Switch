#![windows_subsystem = "windows"] // 隐藏控制台窗口

use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use serde::Deserialize;
use std::fs::{self, OpenOptions};
use std::io::{Write, ErrorKind};
use std::path::Path;
use std::sync::{mpsc as std_mpsc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIconBuilder};
use winit::event_loop::{ControlFlow, EventLoop};
use single_instance::SingleInstance;

mod screen;
mod autostart;
mod icon;

/// MQTT 配置结构体，从 `config.toml` 文件加载
#[derive(Debug, Deserialize)]
struct Config {
    /// MQTT Broker 的 IP 地址
    broker_ip: String,
    /// MQTT Broker 的端口号
    broker_port: u16,
    /// MQTT 用户名（可选）
    username: Option<String>,
    /// MQTT 密码（可选）
    password: Option<String>,
}

/// 日志记录器结构体
struct Logger {
    file: std::fs::File,
}

impl Logger {
    /// 创建新的日志记录器
    fn new() -> Result<Self, std::io::Error> {
        let exe_path = std::env::current_exe()?;
        let log_dir = exe_path.parent().unwrap_or(Path::new("."));
        let log_file = log_dir.join("auto_screen_switch.log");
        
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)?;
        
        Ok(Logger { file })
    }
    
    /// 写入日志
    fn log(&mut self, level: &str, message: &str) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let datetime = chrono::DateTime::from_timestamp(timestamp as i64, 0)
            .unwrap()
            .format("%Y-%m-%d %H:%M:%S");
        
        let log_entry = format!("[{}] [{}] {}\n", datetime, level, message);
        let _ = self.file.write_all(log_entry.as_bytes());
        let _ = self.file.flush();
    }
    
    fn info(&mut self, message: &str) {
        self.log("INFO", message);
    }
    
    fn error(&mut self, message: &str) {
        self.log("ERROR", message);
    }
    
    fn warn(&mut self, message: &str) {
        self.log("WARN", message);
    }
}

/// 全局日志记录器
static LOGGER: Mutex<Option<Logger>> = Mutex::new(None);

/// 初始化日志记录器
fn init_logger() -> Result<(), std::io::Error> {
    let logger = Logger::new()?;
    let mut global_logger = LOGGER.lock().unwrap();
    *global_logger = Some(logger);
    Ok(())
}

/// 记录日志的便捷函数
fn log_info(message: &str) {
    if let Ok(mut logger) = LOGGER.lock() {
        if let Some(ref mut l) = *logger {
            l.info(message);
        }
    }
}

fn log_error(message: &str) {
    if let Ok(mut logger) = LOGGER.lock() {
        if let Some(ref mut l) = *logger {
            l.error(message);
        }
    }
}

fn log_warn(message: &str) {
    if let Ok(mut logger) = LOGGER.lock() {
        if let Some(ref mut l) = *logger {
            l.warn(message);
        }
    }
}

/// 加载配置文件
fn load_config() -> Result<Config, String> {
    log_info("开始加载配置文件");
    
    let exe_path = match std::env::current_exe() {
        Ok(path) => {
            let path_msg = format!("获取可执行文件路径成功: {:?}", path);
            log_info(&path_msg);
            path
        },
        Err(e) => {
            let error_msg = format!("无法获取可执行文件路径: {}", e);
            log_error(&error_msg);
            return Err(error_msg);
        }
    };
    let config_dir = exe_path.parent().unwrap_or(Path::new("."));
    let config_file = config_dir.join("config.toml");
    
    let file_msg = format!("配置文件路径: {:?}", config_file);
    log_info(&file_msg);
    
    let content = match fs::read_to_string(&config_file) {
        Ok(content) => {
            let success_msg = format!("配置文件读取成功 (大小: {} 字节)", content.len());
            log_info(&success_msg);
            content
        },
        Err(e) => {
            if e.kind() == ErrorKind::NotFound {
                // 自动生成默认配置文件
                let default_content = r#"# MQTT Broker 的 IP 地址
broker_ip = "localhost"

# MQTT Broker 的端口号（通常为 1883）
broker_port = 1883

# MQTT 用户名（可选，如果不需要认证请注释掉）
# username = "your_username"

# MQTT 密码（可选，如果不需要认证请注释掉）
# password = "your_password"
"#;
                match fs::write(&config_file, default_content) {
                    Ok(_) => {
                        let msg = format!("未找到配置文件，已生成默认配置文件: {:?}", config_file);
                        log_warn(&msg);
                        return Err("已生成默认 config.toml，请修改后再启动 MQTT".to_string());
                    }
                    Err(write_err) => {
                        let error_msg = format!(
                            "未找到配置文件，尝试生成默认文件失败: {} (路径: {:?})",
                            write_err, config_file
                        );
                        log_error(&error_msg);
                        return Err(error_msg);
                    }
                }
            } else {
                let error_msg = format!("无法读取 config.toml 文件: {} (路径: {:?})", e, config_file);
                log_error(&error_msg);
                return Err(error_msg);
            }
        }
    };
    
    let config: Config = match toml::from_str(&content) {
        Ok(config) => {
            log_info("配置文件格式解析成功");
            config
        },
        Err(e) => {
            let error_msg = format!("config.toml 文件格式错误: {}", e);
            log_error(&error_msg);
            return Err(error_msg);
        }
    };
    
    if config.broker_ip.is_empty() {
        let msg = "MQTT Broker IP 地址不能为空".to_string();
        log_error(&msg);
        return Err(msg);
    }
    if config.broker_port == 0 {
        let msg = "MQTT Broker 端口号不能为 0".to_string();
        log_error(&msg);
        return Err(msg);
    }
    
    let info_msg = format!("📋 配置加载完成 - Broker: {}:{}", config.broker_ip, config.broker_port);
    log_info(&info_msg);
    
    Ok(config)
}

/// MQTT 消息处理
enum MqttCommand {
    Start,
    Stop,
}

/// MQTT 运行状态（从后台任务回传到主线程，用于同步托盘按钮状态）
enum MqttStatus {
    Started,
    Stopped,
    Error(String),
}

/// MQTT 监听和屏幕控制逻辑
async fn run_mqtt_client(
    mut command_rx: mpsc::Receiver<MqttCommand>,
    status_tx: std_mpsc::Sender<MqttStatus>,
) {
    log_info("MQTT 客户端启动");
    let mut retry_count = 0;
    const MAX_RETRIES: u32 = 5;
    const RETRY_DELAY: Duration = Duration::from_secs(5);
    let mut mqtt_running = false;

    loop {
        tokio::select! {
            // 处理托盘命令
            command = command_rx.recv() => {
                match command {
                    Some(MqttCommand::Start) => {
                        if !mqtt_running {
                            log_info("收到启动 MQTT 连接命令");
                            mqtt_running = true;
                        }
                    }
                    Some(MqttCommand::Stop) => {
                        log_info("收到停止 MQTT 连接命令");
                        mqtt_running = false;
                        let _ = status_tx.send(MqttStatus::Stopped);
                        // 不退出任务，继续等待下一次启动
                    }
                    None => {
                        log_info("命令通道关闭，停止 MQTT 客户端");
                        let _ = status_tx.send(MqttStatus::Stopped);
                        break;
                    }
                }
            }
            // MQTT 连接逻辑
            _ = async {
                if !mqtt_running {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    return;
                }

                // 每次启动连接前重新加载配置
                let cfg = match load_config() {
                    Ok(cfg) => cfg,
                    Err(e) => {
                        let msg = format!("启动 MQTT 连接失败（配置错误）：{}", e);
                        log_error(&msg);
                        // 配置不正确，停止运行状态，等待用户修复后再点“启动”
                        mqtt_running = false;
                        let _ = status_tx.send(MqttStatus::Error(msg));
                        let _ = status_tx.send(MqttStatus::Stopped);
                        return;
                    }
                };

                let _ = status_tx.send(MqttStatus::Started);
                let connect_msg = format!("正在连接到 MQTT Broker: {}:{}", cfg.broker_ip, cfg.broker_port);
                log_info(&connect_msg);

                let mut options = MqttOptions::new("auto_screen_switch", cfg.broker_ip.clone(), cfg.broker_port);
                options.set_keep_alive(Duration::from_secs(30));
                
                if let (Some(u), Some(p)) = (cfg.username.clone(), cfg.password.clone()) {
                    options.set_credentials(u, p);
                    log_info("使用认证信息连接 MQTT");
                } else {
                    log_info("使用匿名连接 MQTT");
                }

                let (client, mut eventloop) = AsyncClient::new(options, 10);
                
                match client.subscribe("pi5/display", QoS::AtMostOnce).await {
                    Ok(_) => {
                        log_info("✅ 主题订阅成功: pi5/display");
                        retry_count = 0;
                        
                        loop {
                            if !mqtt_running {
                                log_info("停止 MQTT 监听");
                                break;
                            }

                            match tokio::time::timeout(Duration::from_millis(500), eventloop.poll()).await {
                                Ok(Ok(Event::Incoming(Incoming::Publish(p)))) => {
                                    let payload_str = String::from_utf8_lossy(&p.payload);
                                    let cmd_msg = format!("📨 收到控制指令: '{}'", payload_str);
                                    log_info(&cmd_msg);
                                    
                                    match p.payload.as_ref() {
                                        b"on" => {
                                            log_info("执行操作: 开启屏幕");
                                            screen::set_display(true);
                                            log_info("✅ 屏幕开启操作完成");
                                        }
                                        b"off" => {
                                            log_info("执行操作: 关闭屏幕");
                                            screen::set_display(false);
                                            log_info("✅ 屏幕关闭操作完成");
                                        }
                                        _ => {
                                            let unknown_msg = format!("❌ 收到未知指令: '{}'", payload_str);
                                            log_warn(&unknown_msg);
                                        }
                                    }
                                }
                                Ok(Ok(_)) => {} // 忽略其他 MQTT 事件
                                Ok(Err(e)) => {
                                    let error_msg = format!("MQTT 连接错误: {}", e);
                                    log_error(&error_msg);
                                    break;
                                }
                                Err(_) => {} // 超时，继续循环
                            }
                        }
                    }
                    Err(e) => {
                        let error_msg = format!("MQTT 订阅失败: {}", e);
                        log_error(&error_msg);
                        retry_count += 1;
                        if retry_count >= MAX_RETRIES {
                            log_error("达到最大重试次数");
                            mqtt_running = false;
                            let _ = status_tx.send(MqttStatus::Error(error_msg));
                            let _ = status_tx.send(MqttStatus::Stopped);
                        } else {
                            tokio::time::sleep(RETRY_DELAY).await;
                        }
                    }
                }
            } => {}
        }
    }
}

/// 程序主入口点
fn main() {
    // 初始化日志记录器
    if let Err(e) = init_logger() {
        eprintln!("无法初始化日志记录器: {}", e);
        std::process::exit(1);
    }
    
    // 单实例（基于命名互斥量，跨会话 Global 范围）
    let instance = SingleInstance::new("Global_AutoScreenSwitchMutex").expect("创建单实例句柄失败");
    if !instance.is_single() {
        log_warn("检测到已有实例在运行，当前进程将退出");
        std::process::exit(0);
    }
    
    log_info("🚀 Auto Screen Switch 托盘程序启动");

    // 创建事件循环
    let event_loop = EventLoop::new().expect("无法创建事件循环");
    
    // 创建托盘图标
    let icon_rgba = icon::generate_icon_rgba();
    let icon = Icon::from_rgba(icon_rgba, 16, 16).expect("无法加载托盘图标");

    // 创建菜单项
    let start_item = MenuItem::new("启动 MQTT 连接", true, None);
    let stop_item = MenuItem::new("停止 MQTT 连接", false, None);
    let separator1 = PredefinedMenuItem::separator();
    let autostart_item = MenuItem::new(
        if autostart::is_autostart_enabled() { "禁用开机启动" } else { "启用开机启动" },
        true,
        None
    );
    let separator2 = PredefinedMenuItem::separator();
    let quit_item = MenuItem::new("退出", true, None);

    let menu = Menu::new();
    menu.append(&start_item).unwrap();
    menu.append(&stop_item).unwrap();
    menu.append(&separator1).unwrap();
    menu.append(&autostart_item).unwrap();
    menu.append(&separator2).unwrap();
    menu.append(&quit_item).unwrap();

    // 创建系统托盘
    let _tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Auto Screen Switch - MQTT 屏幕控制器")
        .with_icon(icon)
        .build()
        .expect("无法创建系统托盘");

    log_info("系统托盘创建成功");

    // 创建 MQTT 命令通道与状态通道
    let (command_tx, command_rx) = mpsc::channel(10);
    let (status_tx, status_rx) = std_mpsc::channel::<MqttStatus>();
    
    // 启动 MQTT 客户端（创建 tokio 运行时）
    let runtime = tokio::runtime::Runtime::new().expect("无法创建Tokio运行时");
    let mqtt_handle = runtime.spawn(run_mqtt_client(command_rx, status_tx.clone()));
    
    // 默认启动 MQTT 连接（状态变化由后台任务回传）
    let _ = command_tx.blocking_send(MqttCommand::Start);

    // 监听菜单事件
    let menu_channel = MenuEvent::receiver();
    
    event_loop.run(move |_event, _target| {
        _target.set_control_flow(ControlFlow::Wait);

        // 处理托盘菜单事件
        if let Ok(event) = menu_channel.try_recv() {
            if event.id == start_item.id() {
                log_info("用户点击: 启动 MQTT 连接");
                let _ = command_tx.blocking_send(MqttCommand::Start);
            } else if event.id == stop_item.id() {
                log_info("用户点击: 停止 MQTT 连接");
                let _ = command_tx.blocking_send(MqttCommand::Stop);
            } else if event.id == autostart_item.id() {
                log_info("用户点击: 切换开机启动");
                match autostart::toggle_autostart() {
                    Ok(enabled) => {
                        let status = if enabled { "启用" } else { "禁用" };
                        let msg = format!("开机启动已{}", status);
                        log_info(&msg);
                        
                        let new_text = if enabled { "禁用开机启动" } else { "启用开机启动" };
                        autostart_item.set_text(new_text);
                    }
                    Err(e) => {
                        let error_msg = format!("切换开机启动失败: {}", e);
                        log_error(&error_msg);
                    }
                }
            } else if event.id == quit_item.id() {
                log_info("用户点击: 退出程序");
                _target.exit();
            }
        }

        // 处理 MQTT 状态更新，驱动按钮状态
        while let Ok(status) = status_rx.try_recv() {
            match status {
                MqttStatus::Started => {
                    start_item.set_enabled(false);
                    stop_item.set_enabled(true);
                }
                MqttStatus::Stopped => {
                    start_item.set_enabled(true);
                    stop_item.set_enabled(false);
                }
                MqttStatus::Error(msg) => {
                    let log_msg = format!("MQTT 状态错误: {}", msg);
                    log_error(&log_msg);
                }
            }
        }
    }).expect("事件循环运行失败");

    // 停止 MQTT 客户端
    mqtt_handle.abort();
    log_info("👋 程序已退出");
}