use std::sync::{Arc, Mutex};

use anyhow::Result;

use crate::config::TimerConfig;
use crate::lamp_controller::LampController;

pub struct Timer {
    mode: String,
    schedule: cron::Schedule,
    duration: std::time::Duration,
}

impl Timer {
    pub fn new(config: TimerConfig) -> Self {
        let TimerConfig {
            mode,
            schedule,
            duration,
        } = config;
        Self {
            mode,
            schedule: schedule.0,
            duration: std::time::Duration::from_secs(duration),
        }
    }

    pub async fn run(&self, lamp_controller: Arc<Mutex<LampController>>) -> Result<()> {
        // TODO:
        //  - Handle mid-duration start correctly
        //  - Handle start-start-stop-stop sequences correctly
        let mut upcoming = self.schedule.upcoming(chrono::Local).peekable();
        loop {
            let start_time = match upcoming.next() {
                Some(start_time) => start_time,
                None => return Ok(()),
            };
            let sleep_time = (start_time - chrono::Local::now()).to_std()?;
            tokio::time::sleep(sleep_time).await;
            lamp_controller
                .lock()
                .expect("Failed to unlock mutex")
                .enable(&self.mode)?;

            tokio::time::sleep(self.duration).await;
            lamp_controller
                .lock()
                .expect("Failed to unlock mutex")
                .disable(&self.mode)?;
        }
    }
}
