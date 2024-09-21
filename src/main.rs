use anyhow::Result;
use std::fs::File;
use std::io::Write;
use async_minecraft_ping::ConnectionConfig;
use serde::{Serialize, Deserialize};
use std::time::SystemTime;
use libc_strftime;
use std::env;

const TIME_FORMAT: &str = "%b%d %H:%M:%S %Z";
const SERVER_ADDRESS: &str = "ptyonic.dev";

#[derive(Serialize, Deserialize, Debug)]
struct State {
    online_timestamp: Option<SystemTime>,
    players_timestamp: Option<SystemTime>,
}

fn load_json() -> State {
    let res = std::fs::read_to_string("state.json");
    let default_val = State{online_timestamp: None, players_timestamp: None};

    if let Ok(serialized) = res {
        if let Ok(res) = serde_json::from_str(&serialized) {
            res
        } else {default_val}
    } else {default_val}
}

fn save_json(state: State) -> Result<()> {
    let serialized = serde_json::to_string(&state)?;
    std::fs::write("state.json", serialized)?;
    Ok(())
}

fn format_std_system_time(time: SystemTime) -> String {
    let unix_time = time.duration_since(SystemTime::UNIX_EPOCH).expect("went back in time").as_secs();
    libc_strftime::strftime_local(TIME_FORMAT, unix_time as i64)
}

fn init_libc_time_wrapper() {
    env::set_var("TZ", "Europe/Berlin");
    libc_strftime::tz_set();
}

#[tokio::main]
async fn main() -> Result<()> {
    init_libc_time_wrapper();
    let mut status_file = File::create("status.txt")?;

    let mut state = load_json();
    let now = SystemTime::now();

    let config = ConnectionConfig::build(SERVER_ADDRESS);
    let connection_res = config.connect().await;
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
            write!(status_file, "Last activity seen on {}", format_std_system_time(timestamp))?;
        }

        let ping_start_time = SystemTime::now();
        ping_connection.ping(42).await?;
        let ping_ms = ping_start_time.elapsed()?.as_millis();
        write!(status_file, " (ping {}ms)", ping_ms)?;
    } else {
        write!(status_file, "Offline❎ ")?;
        if let Some(timestamp) = state.online_timestamp {
            write!(status_file, "last online at {}", format_std_system_time(timestamp))?;
        }
    }

    writeln!(status_file)?;
    save_json(state)?;
    Ok(())
}
