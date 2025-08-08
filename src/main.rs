use clap::Parser;
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use serde::Deserialize;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
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

/// 日志记录器结构体
struct Logger {
    file: std::fs::File,
}

impl Logger {
    /// 创建新的日志记录器
    fn new() -> Result<Self, std::io::Error> {
        // 获取可执行文件所在目录
        let exe_path = std::env::current_exe()?;
        let log_dir = exe_path.parent().unwrap_or(Path::new("."));
        let log_file = log_dir.join("auto_screen_switch.log");
        
        // 创建日志文件
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
        
        // 同时输出到控制台和文件
        print!("{}", log_entry);
        let _ = self.file.write_all(log_entry.as_bytes());
        let _ = self.file.flush();
    }
    
    /// 记录信息日志
    fn info(&mut self, message: &str) {
        self.log("INFO", message);
    }
    
    /// 记录错误日志
    fn error(&mut self, message: &str) {
        self.log("ERROR", message);
    }
    
    /// 记录警告日志
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

/// 记录信息日志
fn log_info(message: &str) {
    if let Ok(mut logger) = LOGGER.lock() {
        if let Some(ref mut l) = *logger {
            l.info(message);
        }
    }
}

/// 记录错误日志
fn log_error(message: &str) {
    if let Ok(mut logger) = LOGGER.lock() {
        if let Some(ref mut l) = *logger {
            l.error(message);
        }
    }
}

/// 记录警告日志
fn log_warn(message: &str) {
    if let Ok(mut logger) = LOGGER.lock() {
        if let Some(ref mut l) = *logger {
            l.warn(message);
        }
    }
}

/// 加载并验证配置文件
/// 
/// # Returns
/// 返回解析后的配置对象
/// 
/// # Panics
/// 如果配置文件不存在或格式错误，程序会 panic
fn load_config() -> Config {
    // 获取可执行文件所在目录
    let exe_path = match std::env::current_exe() {
        Ok(path) => path,
        Err(e) => {
            let error_msg = format!("无法获取可执行文件路径: {}", e);
            log_error(&error_msg);
            eprintln!("❌ {}", error_msg);
            panic!("无法获取可执行文件路径");
        }
    };
    let config_dir = exe_path.parent().unwrap_or(Path::new("."));
    let config_file = config_dir.join("config.toml");
    
    // 读取配置文件
    let content = match fs::read_to_string(&config_file) {
        Ok(content) => content,
        Err(e) => {
            let error_msg = format!("无法读取 config.toml 文件: {} (路径: {:?})", e, config_file);
            log_error(&error_msg);
            eprintln!("❌ {}", error_msg);
            eprintln!("请确保 config.toml 文件存在于程序目录中");
            panic!("无法读取 config.toml 文件");
        }
    };
    
    // 解析 TOML 格式的配置
    let config: Config = match toml::from_str(&content) {
        Ok(config) => config,
        Err(e) => {
            let error_msg = format!("config.toml 文件格式错误: {}", e);
            log_error(&error_msg);
            eprintln!("❌ {}", error_msg);
            eprintln!("请检查 config.toml 文件的语法");
            panic!("config.toml 文件格式错误");
        }
    };
    
    // 验证配置的合理性
    if config.broker_ip.is_empty() {
        let error_msg = "MQTT Broker IP 地址不能为空";
        log_error(error_msg);
        eprintln!("❌ 配置错误: {}", error_msg);
        panic!("MQTT Broker IP 地址不能为空");
    }
    if config.broker_port == 0 {
        let error_msg = "MQTT Broker 端口号不能为 0";
        log_error(error_msg);
        eprintln!("❌ 配置错误: {}", error_msg);
        panic!("MQTT Broker 端口号不能为 0");
    }
    
    let info_msg = format!("配置文件加载成功 - Broker: {}:{}", config.broker_ip, config.broker_port);
    log_info(&info_msg);
    println!("✅ {}", info_msg);
    
    if config.username.is_some() {
        log_info("认证: 已配置用户名和密码");
        println!("   认证: 已配置用户名和密码");
    } else {
        log_info("认证: 未配置");
        println!("   认证: 未配置");
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
        let error_msg = format!("服务运行错误: {:?}", e);
        log_error(&error_msg);
        eprintln!("❌ {}", error_msg);
    }
}

/// 运行 Windows 服务的核心逻辑
/// 
/// # Returns
/// 返回服务运行结果
fn run_service() -> windows_service::Result<()> {
    // 初始化日志记录器
    if let Err(e) = init_logger() {
        eprintln!("❌ 初始化日志记录器失败: {}", e);
        return Err(windows_service::Error::Winapi(e));
    }
    
    log_info("Windows 服务启动");
    
    // 创建关闭信号通道
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let tx = Arc::new(Mutex::new(Some(shutdown_tx)));
    
    // 注册服务控制处理器
    let status_handle = service_control_handler::register(SERVICE_NAME, move |control_event| {
        match control_event {
            ServiceControl::Stop => {
                log_info("收到服务停止命令");
                println!("🛑 收到服务停止命令");
                // 收到停止信号时，发送关闭信号
                if let Some(tx) = tx.lock().unwrap().take() {
                    match tx.send(()) {
                        Ok(_) => {
                            log_info("已发送关闭信号");
                            println!("✅ 已发送关闭信号");
                        }
                        Err(e) => {
                            let error_msg = format!("发送关闭信号失败: {:?}", e);
                            log_error(&error_msg);
                            eprintln!("❌ {}", error_msg);
                        }
                    }
                } else {
                    log_warn("关闭信号通道已关闭");
                    println!("⚠️  关闭信号通道已关闭");
                }
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => {
                log_info("收到服务查询命令");
                ServiceControlHandlerResult::NoError
            }
            _ => {
                log_warn("收到未实现的服务控制命令");
                ServiceControlHandlerResult::NotImplemented
            }
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

    log_info("服务状态设置为运行中");

    // 创建 Tokio 运行时并运行主逻辑
    let rt = Runtime::new().unwrap();
    
    // 设置服务停止超时（增加超时时间）
    let stop_timeout = Duration::from_secs(300); // 5分钟超时
    let shutdown_future = run(Some(shutdown_rx));
    
    // 使用超时机制运行服务
    match rt.block_on(tokio::time::timeout(stop_timeout, shutdown_future)) {
        Ok(_) => {
            log_info("服务正常停止");
            println!("✅ 服务正常停止");
        }
        Err(_) => {
            log_warn("服务停止超时，强制退出");
            println!("⚠️  服务停止超时，强制退出");
        }
    }

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
    
    log_info("Windows 服务停止");
    Ok(())
}

/// 主要的 MQTT 监听和屏幕控制逻辑
/// 
/// # Arguments
/// * `shutdown` - 可选的关闭信号接收器，用于优雅关闭
async fn run(mut shutdown: Option<oneshot::Receiver<()>>) {
    // 初始化日志记录器（如果不是服务模式）
    if shutdown.is_none() {
        if let Err(e) = init_logger() {
            eprintln!("❌ 初始化日志记录器失败: {}", e);
            return;
        }
    }
    
    let cfg = load_config();
    let mut retry_count = 0;
    const MAX_RETRIES: u32 = 5;
    const RETRY_DELAY: Duration = Duration::from_secs(5);

    loop {
        let connect_msg = format!("正在连接到 MQTT Broker: {}:{}", cfg.broker_ip, cfg.broker_port);
        log_info(&connect_msg);
        println!("🔄 {}", connect_msg);
        
        // 检查网络连接
        log_info("检查网络连接...");
        println!("🌐 检查网络连接...");

        // 配置 MQTT 连接选项
        let mut options = MqttOptions::new("auto_screen_switch", cfg.broker_ip.clone(), cfg.broker_port);
        
        // 设置保活时间
        options.set_keep_alive(Duration::from_secs(30));
        
        // 如果配置了用户名和密码，则设置认证信息
        if let (Some(u), Some(p)) = (cfg.username.clone(), cfg.password.clone()) {
            options.set_credentials(u, p);
            log_info("使用认证信息连接 MQTT");
            println!("🔐 使用认证信息连接 MQTT");
        } else {
            log_info("使用匿名连接 MQTT");
            println!("🔓 使用匿名连接 MQTT");
        }

        // 创建 MQTT 客户端和事件循环
        let (client, mut eventloop) = AsyncClient::new(options, 10);
        
        log_info("MQTT 客户端创建成功，开始连接...");
        println!("🔌 MQTT 客户端创建成功，开始连接...");
        
        // 订阅屏幕控制主题
        match client.subscribe("pi5/display", QoS::AtMostOnce).await {
            Ok(_) => {
                log_info("已成功订阅主题: pi5/display");
                println!("✅ 已成功订阅主题: pi5/display");
                println!("📡 等待控制指令...");
                
                // 添加连接验证
                log_info("验证连接状态...");
                println!("🔍 验证连接状态...");
                match client.publish("test/connection", QoS::AtMostOnce, false, b"ping").await {
                    Ok(_) => {
                        log_info("连接验证成功 - MQTT Broker 正在运行");
                        println!("✅ 连接验证成功 - MQTT Broker 正在运行");
                        retry_count = 0; // 重置重试计数
                    }
                    Err(e) => {
                        let error_msg = format!("连接验证失败: {}", e);
                        log_error(&error_msg);
                        eprintln!("❌ {}", error_msg);
                        eprintln!("⚠️  虽然订阅成功，但无法发送消息，可能 MQTT Broker 未正常运行");
                        retry_count += 1;
                        if retry_count >= MAX_RETRIES {
                            let error_msg = "达到最大重试次数，程序退出";
                            log_error(error_msg);
                            eprintln!("❌ {}", error_msg);
                            break;
                        }
                        let retry_msg = format!("{} 秒后重试... ({}/{})", RETRY_DELAY.as_secs(), retry_count, MAX_RETRIES);
                        log_info(&retry_msg);
                        println!("⏳ {}", retry_msg);
                        tokio::time::sleep(RETRY_DELAY).await;
                        continue;
                    }
                }
            }
            Err(e) => {
                let error_msg = format!("MQTT 订阅失败: {}", e);
                log_error(&error_msg);
                eprintln!("❌ {}", error_msg);
                retry_count += 1;
                if retry_count >= MAX_RETRIES {
                    let error_msg = "达到最大重试次数，程序退出";
                    log_error(error_msg);
                    eprintln!("❌ {}", error_msg);
                    break;
                }
                let retry_msg = format!("{} 秒后重试... ({}/{})", RETRY_DELAY.as_secs(), retry_count, MAX_RETRIES);
                log_info(&retry_msg);
                println!("⏳ {}", retry_msg);
                tokio::time::sleep(RETRY_DELAY).await;
                continue;
            }
        }

        // 根据是否有关闭信号选择不同的运行模式
        if let Some(mut shutdown) = shutdown.take() {
            // 服务模式：支持优雅关闭
            loop {
                tokio::select! {
                    // 处理关闭信号
                    _ = &mut shutdown => {
                        log_info("收到关闭信号，正在停止服务...");
                        println!("🛑 收到关闭信号，正在停止服务...");
                        return;
                    }
                    // 处理 MQTT 事件，添加更短的超时
                    ev = tokio::time::timeout(Duration::from_millis(500), eventloop.poll()) => match ev {
                        Ok(Ok(Event::Incoming(Incoming::Publish(p)))) => {
                            // 处理屏幕控制指令
                            match p.payload.as_ref() {
                                b"on" => {
                                    log_info("收到开启屏幕指令");
                                    println!("📺 收到开启屏幕指令");
                                    screen::set_display(true);
                                }
                                b"off" => {
                                    log_info("收到关闭屏幕指令");
                                    println!("📺 收到关闭屏幕指令");
                                    screen::set_display(false);
                                }
                                _ => {
                                    let unknown_msg = format!("收到未知指令: {:?}", String::from_utf8_lossy(&p.payload));
                                    log_warn(&unknown_msg);
                                    println!("⚠️  {}", unknown_msg);
                                }
                            }
                        }
                        Ok(Ok(_)) => {} // 忽略其他 MQTT 事件
                        Ok(Err(e)) => {
                            let error_msg = format!("MQTT 连接错误: {}", e);
                            log_error(&error_msg);
                            eprintln!("❌ {}", error_msg);
                            eprintln!("🔄 尝试重新连接...");
                            break; // 跳出内层循环，重新连接
                        }
                        Err(_) => {
                            // 超时，继续循环以检查关闭信号
                            continue;
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
                            log_info("收到开启屏幕指令");
                            println!("📺 收到开启屏幕指令");
                            screen::set_display(true);
                        }
                        b"off" => {
                            log_info("收到关闭屏幕指令");
                            println!("📺 收到关闭屏幕指令");
                            screen::set_display(false);
                        }
                        _ => {
                            let unknown_msg = format!("收到未知指令: {:?}", String::from_utf8_lossy(&p.payload));
                            log_warn(&unknown_msg);
                            println!("⚠️  {}", unknown_msg);
                        }
                    },
                    Ok(_) => {} // 忽略其他 MQTT 事件
                    Err(e) => {
                        let error_msg = format!("MQTT 连接错误: {}", e);
                        log_error(&error_msg);
                        eprintln!("❌ {}", error_msg);
                        eprintln!("🔄 尝试重新连接...");
                        break; // 跳出内层循环，重新连接
                    }
                }
            }
        }

        // 连接断开后的重试逻辑
        retry_count += 1;
        if retry_count >= MAX_RETRIES {
            let error_msg = format!("达到最大重试次数 ({}), 程序退出", MAX_RETRIES);
            log_error(&error_msg);
            eprintln!("❌ {}", error_msg);
            eprintln!("💡 请检查以下项目：");
            eprintln!("   1. MQTT Broker (如 Mosquitto) 是否已启动");
            eprintln!("   2. 端口 {} 是否可访问", cfg.broker_port);
            eprintln!("   3. 防火墙是否阻止了连接");
            eprintln!("   4. config.toml 中的 broker_ip 是否正确");
            break;
        }
        
        let retry_msg = format!("{} 秒后重试连接... ({}/{})", RETRY_DELAY.as_secs(), retry_count, MAX_RETRIES);
        log_info(&retry_msg);
        println!("⏳ {}", retry_msg);
        tokio::time::sleep(RETRY_DELAY).await;
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
        log_info("以 CLI 模式启动");
        println!("🚀 以 CLI 模式启动...");
        let rt = Runtime::new().unwrap();
        rt.block_on(run(None));
        Ok(())
    } else {
        // 服务模式：启动 Windows 服务
        log_info("以服务模式启动");
        println!("🚀 以服务模式启动...");
        service_dispatcher::start(SERVICE_NAME, ffi_service_main)
    }
}

