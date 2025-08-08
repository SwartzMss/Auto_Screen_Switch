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

/// Command line arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Run mode, `service` or `cli`
    #[arg(long, default_value = "service")]
    mode: String,
}

/// MQTT configuration loaded from `config.toml`
#[derive(Debug, Deserialize)]
struct Config {
    broker_ip: String,
    broker_port: u16,
    username: Option<String>,
    password: Option<String>,
}

fn load_config() -> Config {
    let content = fs::read_to_string("config.toml").expect("failed to read config.toml");
    toml::from_str(&content).expect("invalid config file")
}

const SERVICE_NAME: &str = "AutoScreenSwitch";

define_windows_service!(ffi_service_main, my_service_main);

fn my_service_main(_arguments: Vec<OsString>) {
    if let Err(e) = run_service() {
        eprintln!("service error: {e:?}");
    }
}

fn run_service() -> windows_service::Result<()> {
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let tx = Arc::new(Mutex::new(Some(shutdown_tx)));
    let status_handle = service_control_handler::register(SERVICE_NAME, move |control_event| {
        match control_event {
            ServiceControl::Stop | ServiceControl::Interrogate => {
                if let Some(tx) = tx.lock().unwrap().take() {
                    let _ = tx.send(());
                }
                ServiceControlHandlerResult::NoError
            }
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    })?;

    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::StartPending,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 1,
        wait_hint: Duration::from_secs(10),
        process_id: None,
    })?;

    status_handle.set_service_status(ServiceStatus {
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        ..Default::default()
    })?;

    let rt = Runtime::new().unwrap();
    rt.block_on(run(Some(shutdown_rx)));

    status_handle.set_service_status(ServiceStatus {
        current_state: ServiceState::Stopped,
        exit_code: ServiceExitCode::Win32(0),
        ..Default::default()
    })?;
    Ok(())
}

async fn run(mut shutdown: Option<oneshot::Receiver<()>>) {
    let cfg = load_config();

    let mut options = MqttOptions::new("auto_screen_switch", cfg.broker_ip, cfg.broker_port);
    if let (Some(u), Some(p)) = (cfg.username, cfg.password) {
        options.set_credentials(u, p);
    }

    let (client, mut eventloop) = AsyncClient::new(options, 10);
    client
        .subscribe("pi5/display", QoS::AtMostOnce)
        .await
        .expect("subscribe failed");

    if let Some(mut shutdown) = shutdown.take() {
        loop {
            tokio::select! {
                _ = &mut shutdown => break,
                ev = eventloop.poll() => match ev {
                    Ok(Event::Incoming(Incoming::Publish(p))) => {
                        match p.payload.as_ref() {
                            b"on" => screen::set_display(true),
                            b"off" => screen::set_display(false),
                            _ => {}
                        }
                    }
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("MQTT error: {e}");
                        break;
                    }
                }
            }
        }
    } else {
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Incoming::Publish(p))) => match p.payload.as_ref() {
                    b"on" => screen::set_display(true),
                    b"off" => screen::set_display(false),
                    _ => {}
                },
                Ok(_) => {}
                Err(e) => {
                    eprintln!("MQTT error: {e}");
                    break;
                }
            }
        }
    }
}

fn main() -> windows_service::Result<()> {
    let args = Args::parse();
    if args.mode == "cli" {
        let rt = Runtime::new().unwrap();
        rt.block_on(run(None));
        Ok(())
    } else {
        service_dispatcher::start(SERVICE_NAME, ffi_service_main)
    }
}

