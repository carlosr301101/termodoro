use serde::{Deserialize, Serialize};

use crate::config::AppConfig;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Phase {
    Work,
    ShortBreak,
    LongBreak,
}

impl Phase {
    pub fn label(self) -> &'static str {
        match self {
            Phase::Work => "Work",
            Phase::ShortBreak => "Short Break",
            Phase::LongBreak => "Long Break",
        }
    }

    pub fn duration_seconds(self, config: &AppConfig) -> u64 {
        match self {
            Phase::Work => config.work_minutes * 60,
            Phase::ShortBreak => config.short_break_minutes * 60,
            Phase::LongBreak => config.long_break_minutes * 60,
        }
    }
}

pub fn next_phase(current: Phase, completed_work_sessions: u32, long_break_every: u32) -> Phase {
    match current {
        Phase::Work => {
            if completed_work_sessions > 0
                && completed_work_sessions.is_multiple_of(long_break_every)
            {
                Phase::LongBreak
            } else {
                Phase::ShortBreak
            }
        }
        Phase::ShortBreak | Phase::LongBreak => Phase::Work,
    }
}

#[cfg(test)]
mod tests {
    use super::{Phase, next_phase};

    #[test]
    fn uses_short_break_before_long_threshold() {
        assert_eq!(next_phase(Phase::Work, 1, 4), Phase::ShortBreak);
        assert_eq!(next_phase(Phase::Work, 2, 4), Phase::ShortBreak);
        assert_eq!(next_phase(Phase::Work, 3, 4), Phase::ShortBreak);
    }

    #[test]
    fn uses_long_break_at_threshold() {
        assert_eq!(next_phase(Phase::Work, 4, 4), Phase::LongBreak);
        assert_eq!(next_phase(Phase::Work, 8, 4), Phase::LongBreak);
    }
}
