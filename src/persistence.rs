use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use nix::errno::Errno;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;
use crate::domain::Phase;

const APP_DIR: &str = "terminal-pomodore";
const CONFIG_FILE: &str = "config.toml";
const STATE_FILE: &str = "state.json";
const HISTORY_FILE: &str = "history.jsonl";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeState {
    pub pid: u32,
    pub phase: Phase,
    pub phase_started_epoch_secs: u64,
    pub phase_duration_secs: u64,
    pub paused: bool,
    pub pause_started_epoch_secs: Option<u64>,
    pub paused_accumulated_secs: u64,
    pub completed_work_sessions: u32,
    pub updated_epoch_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub phase: Phase,
    pub started_epoch_secs: u64,
    pub ended_epoch_secs: u64,
    pub interrupted: bool,
}

pub type AppResult<T> = Result<T, Box<dyn std::error::Error>>;

fn unix_now_secs() -> AppResult<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

fn config_dir() -> AppResult<PathBuf> {
    let mut dir = dirs::config_dir().ok_or("could not resolve user config directory")?;
    dir.push(APP_DIR);
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn data_dir() -> AppResult<PathBuf> {
    let mut dir = dirs::data_local_dir()
        .or_else(dirs::config_dir)
        .ok_or("could not resolve user data directory")?;
    dir.push(APP_DIR);
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn config_path() -> AppResult<PathBuf> {
    let mut path = config_dir()?;
    path.push(CONFIG_FILE);
    Ok(path)
}

fn state_path() -> AppResult<PathBuf> {
    let mut path = data_dir()?;
    path.push(STATE_FILE);
    Ok(path)
}

fn history_path() -> AppResult<PathBuf> {
    let mut path = data_dir()?;
    path.push(HISTORY_FILE);
    Ok(path)
}

pub fn load_config() -> AppResult<AppConfig> {
    let path = config_path()?;
    if !path.exists() {
        let config = AppConfig::default();
        save_config(&config)?;
        return Ok(config);
    }
    let content = fs::read_to_string(path)?;
    let config: AppConfig = toml::from_str(&content)?;
    config.validate().map_err(|e| e.into()).map(|_| config)
}

pub fn save_config(config: &AppConfig) -> AppResult<()> {
    config.validate()?;
    let path = config_path()?;
    fs::write(path, toml::to_string_pretty(config)?)?;
    Ok(())
}

pub fn load_state() -> AppResult<Option<RuntimeState>> {
    let path = state_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path)?;
    let state: RuntimeState = serde_json::from_str(&content)?;
    Ok(Some(state))
}

pub fn save_state(state: &RuntimeState) -> AppResult<()> {
    let path = state_path()?;
    fs::write(path, serde_json::to_string_pretty(state)?)?;
    Ok(())
}

pub fn clear_state() -> AppResult<()> {
    let path = state_path()?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn append_history(entry: &HistoryEntry) -> AppResult<()> {
    let path = history_path()?;
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let json_line = serde_json::to_string(entry)?;
    file.write_all(json_line.as_bytes())?;
    file.write_all(b"\n")?;
    Ok(())
}

pub fn process_exists(pid: u32) -> AppResult<bool> {
    let pid = Pid::from_raw(pid as i32);
    match kill(pid, None) {
        Ok(()) => Ok(true),
        Err(Errno::ESRCH) => Ok(false),
        Err(Errno::EPERM) => Ok(true),
        Err(err) => Err(err.into()),
    }
}

pub fn send_interrupt(pid: u32) -> AppResult<()> {
    kill(
        Pid::from_raw(pid as i32),
        Some(nix::sys::signal::Signal::SIGINT),
    )?;
    Ok(())
}

pub fn now_epoch_secs() -> AppResult<u64> {
    unix_now_secs()
}
