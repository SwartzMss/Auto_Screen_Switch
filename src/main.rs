use clap::Parser;
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use serde::Deserialize;
use std::fs;

mod screen;

/// Command line arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Run mode, only `cli` is currently supported
    #[arg(long, default_value = "cli")]
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

#[tokio::main]
async fn main() {
    let _args = Args::parse();
    let cfg = load_config();

    let mut options = MqttOptions::new("auto_screen_switch", cfg.broker_ip, cfg.broker_port);
    if let (Some(u), Some(p)) = (cfg.username, cfg.password) {
        options.set_credentials(u, p);
    }

    let (client, mut eventloop) = AsyncClient::new(options, 10);
    client.subscribe("pi5/display", QoS::AtMostOnce).await.expect("subscribe failed");

    loop {
        match eventloop.poll().await {
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
