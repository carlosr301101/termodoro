use std::io::{self, Write};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

use crate::config::AppConfig;
use crate::domain::{Phase, next_phase};
use crate::persistence::{
    AppResult, HistoryEntry, RuntimeState, append_history, clear_state, now_epoch_secs, save_state,
};

struct RawModeGuard;

impl RawModeGuard {
    fn new() -> AppResult<Self> {
        enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

#[derive(Debug, Clone)]
struct PhaseTimer {
    phase: Phase,
    phase_started_epoch_secs: u64,
    phase_duration_secs: u64,
    paused_accumulated_secs: u64,
    pause_started_epoch_secs: Option<u64>,
}

impl PhaseTimer {
    fn new(phase: Phase, phase_duration_secs: u64) -> AppResult<Self> {
        Ok(Self {
            phase,
            phase_started_epoch_secs: now_epoch_secs()?,
            phase_duration_secs,
            paused_accumulated_secs: 0,
            pause_started_epoch_secs: None,
        })
    }

    fn is_paused(&self) -> bool {
        self.pause_started_epoch_secs.is_some()
    }

    fn pause(&mut self, now_epoch_secs: u64) {
        if self.pause_started_epoch_secs.is_none() {
            self.pause_started_epoch_secs = Some(now_epoch_secs);
        }
    }

    fn resume(&mut self, now_epoch_secs: u64) {
        if let Some(paused_at) = self.pause_started_epoch_secs.take() {
            self.paused_accumulated_secs += now_epoch_secs.saturating_sub(paused_at);
        }
    }

    fn remaining_secs(&self, now_epoch_secs: u64) -> u64 {
        compute_remaining_secs(
            self.phase_duration_secs,
            self.phase_started_epoch_secs,
            self.paused_accumulated_secs,
            self.pause_started_epoch_secs,
            now_epoch_secs,
        )
    }
}

pub fn compute_remaining_secs(
    duration_secs: u64,
    phase_started_epoch_secs: u64,
    paused_accumulated_secs: u64,
    pause_started_epoch_secs: Option<u64>,
    now_epoch_secs: u64,
) -> u64 {
    let elapsed_total = now_epoch_secs.saturating_sub(phase_started_epoch_secs);
    let current_pause = pause_started_epoch_secs
        .map(|paused_at| now_epoch_secs.saturating_sub(paused_at))
        .unwrap_or(0);
    let active_elapsed = elapsed_total.saturating_sub(paused_accumulated_secs + current_pause);
    duration_secs.saturating_sub(active_elapsed)
}

fn render_countdown_line(
    phase: Phase,
    total_secs: u64,
    remaining_secs: u64,
    paused: bool,
) -> AppResult<()> {
    let elapsed_secs = total_secs.saturating_sub(remaining_secs);
    let progress = if total_secs == 0 {
        1.0
    } else {
        elapsed_secs as f64 / total_secs as f64
    };
    let filled = (progress * 20.0).round() as usize;
    let filled = filled.min(20);
    let bar = format!("{}{}", "#".repeat(filled), "-".repeat(20 - filled));
    let mins = remaining_secs / 60;
    let secs = remaining_secs % 60;
    let state = if paused { "paused" } else { "running" };

    print!(
        "\r{} {:02}:{:02} [{}] {}  (p=pause r=resume q=quit)",
        phase.label(),
        mins,
        secs,
        bar,
        state
    );
    io::stdout().flush()?;
    Ok(())
}

fn build_runtime_state(
    timer: &PhaseTimer,
    completed_work_sessions: u32,
) -> AppResult<RuntimeState> {
    Ok(RuntimeState {
        pid: std::process::id(),
        phase: timer.phase,
        phase_started_epoch_secs: timer.phase_started_epoch_secs,
        phase_duration_secs: timer.phase_duration_secs,
        paused: timer.is_paused(),
        pause_started_epoch_secs: timer.pause_started_epoch_secs,
        paused_accumulated_secs: timer.paused_accumulated_secs,
        completed_work_sessions,
        updated_epoch_secs: now_epoch_secs()?,
    })
}

fn append_phase_history(timer: &PhaseTimer, interrupted: bool) -> AppResult<()> {
    let now = now_epoch_secs()?;
    append_history(&HistoryEntry {
        phase: timer.phase,
        started_epoch_secs: timer.phase_started_epoch_secs,
        ended_epoch_secs: now,
        interrupted,
    })
}

pub fn run_pomodoro(config: AppConfig, cycles: Option<u32>) -> AppResult<()> {
    let _raw_mode = RawModeGuard::new()?;
    let stop_requested = Arc::new(AtomicBool::new(false));
    let stop_for_handler = Arc::clone(&stop_requested);

    ctrlc::set_handler(move || {
        stop_for_handler.store(true, Ordering::SeqCst);
    })?;

    println!(
        "Starting Pomodoro. Work={}m ShortBreak={}m LongBreak={}m LongEvery={} (Ctrl+C or q to stop)",
        config.work_minutes,
        config.short_break_minutes,
        config.long_break_minutes,
        config.long_break_every
    );

    let mut phase = Phase::Work;
    let mut completed_work_sessions = 0_u32;
    let mut completed_work_for_target = 0_u32;

    loop {
        let mut timer = PhaseTimer::new(phase, phase.duration_seconds(&config))?;
        save_state(&build_runtime_state(&timer, completed_work_sessions)?)?;

        let mut last_draw_epoch = 0_u64;

        loop {
            let now = now_epoch_secs()?;

            if event::poll(Duration::from_millis(200))? {
                if let Event::Key(key_event) = event::read()? {
                    match key_event.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => {
                            stop_requested.store(true, Ordering::SeqCst);
                        }
                        KeyCode::Char('p') | KeyCode::Char('P') if !timer.is_paused() => {
                            timer.pause(now);
                            save_state(&build_runtime_state(&timer, completed_work_sessions)?)?;
                        }
                        KeyCode::Char('r') | KeyCode::Char('R') if timer.is_paused() => {
                            timer.resume(now);
                            save_state(&build_runtime_state(&timer, completed_work_sessions)?)?;
                        }
                        _ => {}
                    }
                }
            }

            if stop_requested.load(Ordering::SeqCst) {
                println!();
                append_phase_history(&timer, true)?;
                clear_state()?;
                println!("Pomodoro stopped.");
                return Ok(());
            }

            let remaining = timer.remaining_secs(now);

            if now != last_draw_epoch {
                render_countdown_line(
                    phase,
                    timer.phase_duration_secs,
                    remaining,
                    timer.is_paused(),
                )?;
                save_state(&build_runtime_state(&timer, completed_work_sessions)?)?;
                last_draw_epoch = now;
            }

            if remaining == 0 {
                break;
            }

            thread::sleep(Duration::from_millis(40));
        }

        println!();
        print!("\x07");
        io::stdout().flush()?;

        append_phase_history(&timer, false)?;

        if phase == Phase::Work {
            completed_work_sessions += 1;
            completed_work_for_target += 1;

            if cycles.is_some_and(|target| completed_work_for_target >= target) {
                clear_state()?;
                println!("Completed {completed_work_for_target} work session(s).");
                return Ok(());
            }
        }

        phase = next_phase(phase, completed_work_sessions, config.long_break_every);
    }
}

pub fn format_remaining(remaining_secs: u64) -> String {
    let mins = remaining_secs / 60;
    let secs = remaining_secs % 60;
    format!("{mins:02}:{secs:02}")
}

#[cfg(test)]
mod tests {
    use super::compute_remaining_secs;

    #[test]
    fn remaining_reduces_without_pause() {
        let remaining = compute_remaining_secs(1500, 100, 0, None, 160);
        assert_eq!(remaining, 1440);
    }

    #[test]
    fn remaining_freezes_during_pause() {
        let remaining = compute_remaining_secs(1500, 100, 0, Some(120), 150);
        assert_eq!(remaining, 1480);
    }
}
