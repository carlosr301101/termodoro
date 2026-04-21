use clap::{Parser, Subcommand};

use termodoro::config::{AppConfig, ConfigOverrides};
use termodoro::engine::{compute_remaining_secs, format_remaining, run_pomodoro};
use termodoro::persistence::{
    AppResult, clear_state, load_config, load_state, now_epoch_secs, process_exists, save_config,
    send_interrupt,
};

#[derive(Parser, Debug)]
#[command(
    name = "terminal-pomodore",
    version,
    about = "A terminal Pomodoro timer with configurable work/break cycles"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Start {
        #[arg(long)]
        work: Option<u64>,
        #[arg(long = "short-break")]
        short_break: Option<u64>,
        #[arg(long = "long-break")]
        long_break: Option<u64>,
        #[arg(long = "long-every")]
        long_every: Option<u32>,
        #[arg(long, help = "Stop after this many work sessions")]
        cycles: Option<u32>,
    },
    Status,
    Stop,
    Config {
        #[arg(long)]
        work: Option<u64>,
        #[arg(long = "short-break")]
        short_break: Option<u64>,
        #[arg(long = "long-break")]
        long_break: Option<u64>,
        #[arg(long = "long-every")]
        long_every: Option<u32>,
        #[arg(long)]
        reset: bool,
    },
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> AppResult<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Start {
            work,
            short_break,
            long_break,
            long_every,
            cycles,
        } => {
            if cycles == Some(0) {
                return Err("cycles must be greater than 0".into());
            }
            let base = load_config()?;
            let config = base.apply_overrides(&ConfigOverrides {
                work_minutes: work,
                short_break_minutes: short_break,
                long_break_minutes: long_break,
                long_break_every: long_every,
            });
            config.validate()?;
            run_pomodoro(config, cycles)?;
        }
        Commands::Status => print_status()?,
        Commands::Stop => stop_timer()?,
        Commands::Config {
            work,
            short_break,
            long_break,
            long_every,
            reset,
        } => update_or_show_config(work, short_break, long_break, long_every, reset)?,
    }
    Ok(())
}

fn print_status() -> AppResult<()> {
    let Some(state) = load_state()? else {
        println!("No running pomodoro timer.");
        return Ok(());
    };

    if !process_exists(state.pid)? {
        clear_state()?;
        println!("No running pomodoro timer (stale state cleaned).");
        return Ok(());
    }

    let now = now_epoch_secs()?;
    let remaining = compute_remaining_secs(
        state.phase_duration_secs,
        state.phase_started_epoch_secs,
        state.paused_accumulated_secs,
        state.pause_started_epoch_secs,
        now,
    );

    println!("Running: {}", state.phase.label());
    println!("Remaining: {}", format_remaining(remaining));
    println!(
        "Progress: {} / {}",
        state.phase_duration_secs.saturating_sub(remaining),
        state.phase_duration_secs
    );
    println!("Completed work sessions: {}", state.completed_work_sessions);
    println!("Paused: {}", if state.paused { "yes" } else { "no" });
    println!("PID: {}", state.pid);
    Ok(())
}

fn stop_timer() -> AppResult<()> {
    let Some(state) = load_state()? else {
        println!("No running pomodoro timer.");
        return Ok(());
    };

    if !process_exists(state.pid)? {
        clear_state()?;
        println!("No running pomodoro timer (stale state cleaned).");
        return Ok(());
    }

    send_interrupt(state.pid)?;
    println!("Sent stop signal to timer process {}.", state.pid);
    Ok(())
}

fn update_or_show_config(
    work: Option<u64>,
    short_break: Option<u64>,
    long_break: Option<u64>,
    long_every: Option<u32>,
    reset: bool,
) -> AppResult<()> {
    if reset {
        let defaults = AppConfig::default();
        save_config(&defaults)?;
        print_config(&defaults);
        return Ok(());
    }

    if work.is_none() && short_break.is_none() && long_break.is_none() && long_every.is_none() {
        let current = load_config()?;
        print_config(&current);
        return Ok(());
    }

    let current = load_config()?;
    let updated = current.apply_overrides(&ConfigOverrides {
        work_minutes: work,
        short_break_minutes: short_break,
        long_break_minutes: long_break,
        long_break_every: long_every,
    });
    updated.validate()?;
    save_config(&updated)?;
    print_config(&updated);
    Ok(())
}

fn print_config(config: &AppConfig) {
    println!("work_minutes = {}", config.work_minutes);
    println!("short_break_minutes = {}", config.short_break_minutes);
    println!("long_break_minutes = {}", config.long_break_minutes);
    println!("long_break_every = {}", config.long_break_every);
}
