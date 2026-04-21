use serde::{Deserialize, Serialize};

/// Persistent Pomodoro configuration expressed in minutes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    /// Duration of one work phase in minutes.
    pub work_minutes: u64,
    /// Duration of one short break phase in minutes.
    pub short_break_minutes: u64,
    /// Duration of one long break phase in minutes.
    pub long_break_minutes: u64,
    /// Insert a long break after every N completed work sessions.
    pub long_break_every: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            work_minutes: 25,
            short_break_minutes: 5,
            long_break_minutes: 15,
            long_break_every: 4,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ConfigOverrides {
    /// Optional replacement for [`AppConfig::work_minutes`].
    pub work_minutes: Option<u64>,
    /// Optional replacement for [`AppConfig::short_break_minutes`].
    pub short_break_minutes: Option<u64>,
    /// Optional replacement for [`AppConfig::long_break_minutes`].
    pub long_break_minutes: Option<u64>,
    /// Optional replacement for [`AppConfig::long_break_every`].
    pub long_break_every: Option<u32>,
}

impl AppConfig {
    /// Returns a copy of this config with provided overrides applied.
    pub fn apply_overrides(&self, overrides: &ConfigOverrides) -> Self {
        Self {
            work_minutes: overrides.work_minutes.unwrap_or(self.work_minutes),
            short_break_minutes: overrides
                .short_break_minutes
                .unwrap_or(self.short_break_minutes),
            long_break_minutes: overrides
                .long_break_minutes
                .unwrap_or(self.long_break_minutes),
            long_break_every: overrides.long_break_every.unwrap_or(self.long_break_every),
        }
    }

    /// Validates that all configured durations and frequency are non-zero.
    ///
    /// Returns a user-facing error message describing the invalid field.
    pub fn validate(&self) -> Result<(), String> {
        if self.work_minutes == 0 {
            return Err("work_minutes must be greater than 0".to_string());
        }
        if self.short_break_minutes == 0 {
            return Err("short_break_minutes must be greater than 0".to_string());
        }
        if self.long_break_minutes == 0 {
            return Err("long_break_minutes must be greater than 0".to_string());
        }
        if self.long_break_every == 0 {
            return Err("long_break_every must be greater than 0".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::AppConfig;

    #[test]
    fn parses_config_from_toml() {
        let content = r#"
work_minutes = 30
short_break_minutes = 6
long_break_minutes = 20
long_break_every = 5
"#;

        let parsed: AppConfig = toml::from_str(content).expect("toml should parse");
        assert_eq!(parsed.work_minutes, 30);
        assert_eq!(parsed.short_break_minutes, 6);
        assert_eq!(parsed.long_break_minutes, 20);
        assert_eq!(parsed.long_break_every, 5);
    }
}
