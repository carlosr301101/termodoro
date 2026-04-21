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

/// Persisted runtime snapshot used by `status` and `stop` commands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeState {
    /// PID of the timer process that owns this state.
    pub pid: u32,
    /// Currently active phase.
    pub phase: Phase,
    /// UNIX epoch timestamp when the current phase started.
    pub phase_started_epoch_secs: u64,
    /// Total configured phase duration in seconds.
    pub phase_duration_secs: u64,
    /// Whether the timer is currently paused.
    pub paused: bool,
    /// UNIX epoch timestamp when pause began, when paused.
    pub pause_started_epoch_secs: Option<u64>,
    /// Total paused seconds accumulated before the active pause.
    pub paused_accumulated_secs: u64,
    /// Number of completed work sessions in this run.
    pub completed_work_sessions: u32,
    /// UNIX epoch timestamp of the last state update.
    pub updated_epoch_secs: u64,
}

/// One line in the `history.jsonl` activity log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Phase that just ended.
    pub phase: Phase,
    /// UNIX epoch timestamp when that phase started.
    pub started_epoch_secs: u64,
    /// UNIX epoch timestamp when that phase ended.
    pub ended_epoch_secs: u64,
    /// `true` when the phase ended due to an interruption.
    pub interrupted: bool,
}

/// Crate-wide error type used by the CLI and core modules.
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

/// Loads the app configuration file, creating it with defaults when missing.
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

/// Writes the validated app configuration to disk.
pub fn save_config(config: &AppConfig) -> AppResult<()> {
    config.validate()?;
    let path = config_path()?;
    fs::write(path, toml::to_string_pretty(config)?)?;
    Ok(())
}

/// Loads the runtime state from disk when a timer is running.
pub fn load_state() -> AppResult<Option<RuntimeState>> {
    let path = state_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path)?;
    let state: RuntimeState = serde_json::from_str(&content)?;
    Ok(Some(state))
}

/// Persists runtime state for `status`, pause/resume, and stop operations.
pub fn save_state(state: &RuntimeState) -> AppResult<()> {
    let path = state_path()?;
    fs::write(path, serde_json::to_string_pretty(state)?)?;
    Ok(())
}

/// Removes persisted runtime state, if present.
pub fn clear_state() -> AppResult<()> {
    let path = state_path()?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

/// Appends one JSON record to the timer history log.
pub fn append_history(entry: &HistoryEntry) -> AppResult<()> {
    let path = history_path()?;
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let json_line = serde_json::to_string(entry)?;
    file.write_all(json_line.as_bytes())?;
    file.write_all(b"\n")?;
    Ok(())
}

/// Returns `true` when the provided PID exists or is inaccessible due to permissions.
pub fn process_exists(pid: u32) -> AppResult<bool> {
    let pid = Pid::from_raw(pid as i32);
    match kill(pid, None) {
        Ok(()) => Ok(true),
        Err(Errno::ESRCH) => Ok(false),
        Err(Errno::EPERM) => Ok(true),
        Err(err) => Err(err.into()),
    }
}

/// Sends a `SIGINT` signal to the provided process ID.
pub fn send_interrupt(pid: u32) -> AppResult<()> {
    kill(
        Pid::from_raw(pid as i32),
        Some(nix::sys::signal::Signal::SIGINT),
    )?;
    Ok(())
}

/// Returns current UNIX epoch seconds.
pub fn now_epoch_secs() -> AppResult<u64> {
    unix_now_secs()
}
