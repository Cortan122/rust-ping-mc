use anyhow::Result;
use std::fs::File;
use std::io::Write;
use async_minecraft_ping::ConnectionConfig;
use serde::{Serialize, Deserialize};
use std::time::SystemTime;
use libc_strftime;
use std::env;

struct Config {
    time_format: String,
    time_format_timezone: String,
    server_address: String,
    status_path: String,
    state_path: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct State {
    online_timestamp: Option<SystemTime>,
    players_timestamp: Option<SystemTime>,
}

fn load_json(config: &Config) -> State {
    let res = std::fs::read_to_string(&config.state_path);
    let default_val = State{online_timestamp: None, players_timestamp: None};

    if let Ok(serialized) = res {
        if let Ok(res) = serde_json::from_str(&serialized) {
            res
        } else {default_val}
    } else {default_val}
}

fn save_json(config: &Config, state: &State) -> Result<()> {
    let serialized = serde_json::to_string(state)?;
    std::fs::write(&config.state_path, serialized)?;
    Ok(())
}

fn format_std_system_time(config: &Config, time: SystemTime) -> String {
    let unix_time = time.duration_since(SystemTime::UNIX_EPOCH).expect("went back in time").as_secs();
    libc_strftime::strftime_local(&config.time_format, unix_time as i64)
}

fn init_libc_time_wrapper(config: &Config) {
    env::set_var("TZ", &config.time_format_timezone);
    libc_strftime::tz_set();
}

fn read_config() -> Config {
    fn get_env(name: &str, default: &str) -> String {
        return env::var(name).unwrap_or(default.to_string());
    }

    Config {
        time_format: get_env("TIME_FORMAT", "%b%d %H:%M:%S %Z"),
        time_format_timezone: get_env("TIME_FORMAT_TIMEZONE", "Europe/Berlin"),
        server_address: get_env("SERVER_ADDRESS", "ptyonic.dev"),
        status_path: get_env("STATUS_PATH", "status.txt"),
        state_path: get_env("STATE_PATH", "state.json"),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = read_config();
    init_libc_time_wrapper(&config);
    let mut status_file = File::create(&config.status_path)?;

    let mut state = load_json(&config);
    let now = SystemTime::now();

    let connection_config = ConnectionConfig::build(&config.server_address);
    let connection_res = connection_config.connect().await;
    if let Ok(connection) = connection_res {
        let ping_connection = connection.status().await?;
        let num_players = ping_connection.status.players.online;
        state.online_timestamp = Some(now);

        write!(status_file,
            "Online✅ with {}/20 players. ",
            num_players
        )?;

        if num_players > 0 {
            state.players_timestamp = Some(now);
            write!(status_file, "yay!")?;
        } else if let Some(timestamp) = state.players_timestamp {
            write!(status_file, "Last activity seen on {}", format_std_system_time(&config, timestamp))?;
        }

        let ping_start_time = SystemTime::now();
        ping_connection.ping(42).await?;
        let ping_ms = ping_start_time.elapsed()?.as_millis();
        write!(status_file, " (ping {}ms)", ping_ms)?;
    } else {
        write!(status_file, "Offline❎ ")?;
        if let Some(timestamp) = state.online_timestamp {
            write!(status_file, "last online at {}", format_std_system_time(&config, timestamp))?;
        }
    }

    writeln!(status_file)?;
    save_json(&config, &state)?;
    Ok(())
}
