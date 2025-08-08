use clap::Parser;
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use serde::Deserialize;
use std::ffi::OsString;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::sync::oneshot;
use windows_service::{
    define_windows_service,
    service::{ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType},
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
};

mod screen;

/// 命令行参数结构体
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 运行模式：`service`（服务模式）或 `cli`（命令行模式）
    #[arg(long, default_value = "service")]
    mode: String,
}

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

/// 加载并验证配置文件
/// 
/// # Returns
/// 返回解析后的配置对象
/// 
/// # Panics
/// 如果配置文件不存在或格式错误，程序会 panic
fn load_config() -> Config {
    // 读取配置文件
    let content = fs::read_to_string("config.toml")
        .expect("无法读取 config.toml 文件，请确保配置文件存在");
    
    // 解析 TOML 格式的配置
    let config: Config = toml::from_str(&content)
        .expect("config.toml 文件格式错误，请检查配置语法");
    
    // 验证配置的合理性
    if config.broker_ip.is_empty() {
        panic!("MQTT Broker IP 地址不能为空");
    }
    if config.broker_port == 0 {
        panic!("MQTT Broker 端口号不能为 0");
    }
    
    config
}

/// Windows 服务名称常量
const SERVICE_NAME: &str = "AutoScreenSwitch";

define_windows_service!(ffi_service_main, my_service_main);

/// Windows 服务主函数
/// 
/// # Arguments
/// * `_arguments` - 服务启动参数（当前未使用）
fn my_service_main(_arguments: Vec<OsString>) {
    if let Err(e) = run_service() {
        eprintln!("服务运行错误: {e:?}");
    }
}

/// 运行 Windows 服务的核心逻辑
/// 
/// # Returns
/// 返回服务运行结果
fn run_service() -> windows_service::Result<()> {
    // 创建关闭信号通道
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let tx = Arc::new(Mutex::new(Some(shutdown_tx)));
    
    // 注册服务控制处理器
    let status_handle = service_control_handler::register(SERVICE_NAME, move |control_event| {
        match control_event {
            ServiceControl::Stop | ServiceControl::Interrogate => {
                // 收到停止信号时，发送关闭信号
                if let Some(tx) = tx.lock().unwrap().take() {
                    let _ = tx.send(());
                }
                ServiceControlHandlerResult::NoError
            }
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    })?;

    // 设置服务状态为启动中
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::StartPending,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 1,
        wait_hint: Duration::from_secs(10),
        process_id: None,
    })?;

    // 设置服务状态为运行中
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::from_secs(0),
        process_id: None,
    })?;

    // 创建 Tokio 运行时并运行主逻辑
    let rt = Runtime::new().unwrap();
    rt.block_on(run(Some(shutdown_rx)));

    // 设置服务状态为已停止
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::from_secs(0),
        process_id: None,
    })?;
    Ok(())
}

/// 主要的 MQTT 监听和屏幕控制逻辑
/// 
/// # Arguments
/// * `shutdown` - 可选的关闭信号接收器，用于优雅关闭
async fn run(mut shutdown: Option<oneshot::Receiver<()>>) {
    // 加载配置
    let cfg = load_config();
    println!("正在连接到 MQTT Broker: {}:{}", cfg.broker_ip, cfg.broker_port);

    // 配置 MQTT 连接选项
    let mut options = MqttOptions::new("auto_screen_switch", cfg.broker_ip, cfg.broker_port);
    
    // 如果配置了用户名和密码，则设置认证信息
    if let (Some(u), Some(p)) = (cfg.username, cfg.password) {
        options.set_credentials(u, p);
    }

    // 创建 MQTT 客户端和事件循环
    let (client, mut eventloop) = AsyncClient::new(options, 10);
    
    // 订阅屏幕控制主题
    client
        .subscribe("pi5/display", QoS::AtMostOnce)
        .await
        .expect("MQTT 订阅失败");

    println!("已订阅主题: pi5/display，等待控制指令...");

    // 根据是否有关闭信号选择不同的运行模式
    if let Some(mut shutdown) = shutdown.take() {
        // 服务模式：支持优雅关闭
        loop {
            tokio::select! {
                // 处理关闭信号
                _ = &mut shutdown => {
                    println!("收到关闭信号，正在停止服务...");
                    break;
                }
                // 处理 MQTT 事件
                ev = eventloop.poll() => match ev {
                    Ok(Event::Incoming(Incoming::Publish(p))) => {
                        // 处理屏幕控制指令
                        match p.payload.as_ref() {
                            b"on" => {
                                println!("收到开启屏幕指令");
                                screen::set_display(true);
                            }
                            b"off" => {
                                println!("收到关闭屏幕指令");
                                screen::set_display(false);
                            }
                            _ => {
                                println!("收到未知指令: {:?}", String::from_utf8_lossy(&p.payload));
                            }
                        }
                    }
                    Ok(_) => {} // 忽略其他 MQTT 事件
                    Err(e) => {
                        eprintln!("MQTT 连接错误: {e}");
                        break;
                    }
                }
            }
        }
    } else {
        // CLI 模式：简单循环
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Incoming::Publish(p))) => match p.payload.as_ref() {
                    b"on" => {
                        println!("收到开启屏幕指令");
                        screen::set_display(true);
                    }
                    b"off" => {
                        println!("收到关闭屏幕指令");
                        screen::set_display(false);
                    }
                    _ => {
                        println!("收到未知指令: {:?}", String::from_utf8_lossy(&p.payload));
                    }
                },
                Ok(_) => {} // 忽略其他 MQTT 事件
                Err(e) => {
                    eprintln!("MQTT 连接错误: {e}");
                    break;
                }
            }
        }
    }
}

/// 程序主入口点
/// 
/// # Returns
/// 返回程序执行结果
fn main() -> windows_service::Result<()> {
    // 解析命令行参数
    let args = Args::parse();
    
    if args.mode == "cli" {
        // CLI 模式：直接运行 MQTT 监听逻辑
        println!("以 CLI 模式启动...");
        let rt = Runtime::new().unwrap();
        rt.block_on(run(None));
        Ok(())
    } else {
        // 服务模式：启动 Windows 服务
        println!("以服务模式启动...");
        service_dispatcher::start(SERVICE_NAME, ffi_service_main)
    }
}

