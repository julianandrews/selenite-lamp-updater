use anyhow::Result;
use serde::{Deserialize, Deserializer};

pub fn load(file: &std::path::Path) -> Result<Config> {
    let config: Config = toml::from_str(&std::fs::read_to_string(file)?)?;
    let defined_modes: std::collections::BTreeSet<&str> =
        config.modes.iter().map(|mode| mode.name.as_str()).collect();
    let default_modes = std::iter::once(config.default_mode.as_str());
    let timer_modes = config.timers.iter().map(|timer| timer.mode.as_str());
    let count_modes = config.count_files.iter().map(|timer| timer.mode.as_str());
    for mode in default_modes.chain(timer_modes).chain(count_modes) {
        if !defined_modes.contains(mode) {
            return Err(anyhow::anyhow!(format!("Mode {} not defined.", mode,)));
        }
    }
    Ok(config)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Config {
    pub output_file: Option<std::path::PathBuf>,
    pub default_mode: String,
    pub modes: Vec<LampMode>,
    pub timers: Vec<TimerConfig>,
    pub count_files: Vec<CountFileConfig>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct LampMode {
    pub name: String,
    pub command: selenite_lamp_commands::Command,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TimerConfig {
    pub mode: String,
    pub schedule: ScheduleConfig,
    pub duration: u64,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct CountFileConfig {
    pub mode: String,
    pub file: std::path::PathBuf,
}

#[derive(Debug, Clone)]
pub struct ScheduleConfig(pub cron::Schedule);

impl std::str::FromStr for ScheduleConfig {
    type Err = cron::error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let schedule = std::str::FromStr::from_str(&s)?;
        Ok(Self(schedule))
    }
}

impl<'de> Deserialize<'de> for ScheduleConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        std::str::FromStr::from_str(&s).map_err(serde::de::Error::custom)
    }
}
