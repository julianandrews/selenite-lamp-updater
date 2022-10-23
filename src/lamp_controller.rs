use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result};
use selenite_lamp_commands::Command;

use crate::config::LampMode;

type Priority = usize;

#[derive(Debug, Clone)]
pub struct LampController {
    file: std::path::PathBuf,
    commands: Vec<Command>,
    priorities: BTreeMap<String, Priority>,
    active_modes: BTreeSet<Priority>,
}

impl LampController {
    pub fn new(file: std::path::PathBuf, modes: &[LampMode]) -> Self {
        let commands = modes.iter().map(|mode| mode.command.clone()).collect();
        let priorities = modes
            .iter()
            .enumerate()
            .map(|(i, mode)| (mode.name.clone(), i))
            .collect();
        let active_modes = BTreeSet::new();
        Self {
            file,
            commands,
            priorities,
            active_modes,
        }
    }

    pub fn enable(&mut self, mode: &str) -> Result<()> {
        self.active_modes.insert(self.priority(mode)?);
        println!("Enabling {} mode", mode);
        self.update_lamp()?;
        Ok(())
    }

    pub fn disable(&mut self, mode: &str) -> Result<()> {
        self.active_modes.remove(&self.priority(mode)?);
        println!("Disabling {} mode", mode);
        self.update_lamp()?;
        Ok(())
    }

    fn update_lamp(&self) -> Result<()> {
        let old_command = match std::fs::read_to_string(&self.file) {
            Ok(data) => Some(serde_json::de::from_str(&data)?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => Err(error)?,
        };
        let new_command = self.top_command()?;
        if Some(new_command) != old_command.as_ref() {
            let json = serde_json::to_string(&new_command)?;
            std::fs::write(&self.file, &json).context("Failed to write file")?;
        }
        Ok(())
    }

    fn priority(&self, mode: &str) -> Result<Priority> {
        let priority = self
            .priorities
            .get(mode)
            .ok_or_else(|| anyhow::anyhow!("Unrecognized mode"))?;
        Ok(*priority)
    }

    fn top_command(&self) -> Result<&Command> {
        let priority = self
            .active_modes
            .iter()
            .max()
            .ok_or_else(|| anyhow::anyhow!("No modes present"))?;
        self.commands
            .get(*priority)
            .ok_or_else(|| anyhow::anyhow!("Missing command"))
    }
}
