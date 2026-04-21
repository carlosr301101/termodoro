# terminal-pomodore

Terminal Pomodoro app built in Rust.

## Commands

```bash
termodoro start
termodoro start --work 50 --short-break 10 --long-break 20 --long-every 4
termodoro start --cycles 2
termodoro status
termodoro stop
termodoro config
termodoro config --work 30 --short-break 5 --long-break 15 --long-every 4
termodoro config --reset
```

## Runtime controls (during `start`)

- `p`: pause timer
- `r`: resume timer
- `q`: stop timer
- `Ctrl+C`: graceful stop

## Defaults

- Work: 25 minutes
- Short break: 5 minutes
- Long break: 15 minutes
- Long break frequency: every 4 completed work sessions

Config is saved in your user config directory under `terminal-pomodore/config.toml`.
Runtime state and history are saved in your user data directory under `terminal-pomodore/`.
