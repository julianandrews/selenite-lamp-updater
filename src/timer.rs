use std::sync::{Arc, Mutex};

use anyhow::Result;

use crate::config::TimerConfig;
use crate::lamp_controller::LampController;

pub struct Timer {
    mode: String,
    schedule: cron::Schedule,
    duration: chrono::Duration,
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
            duration: chrono::Duration::seconds(duration),
        }
    }

    pub async fn run(&self, lamp_controller: Arc<Mutex<LampController>>) -> Result<()> {
        let lookback_time = chrono::Local::now() - self.duration;
        let mut start_times = self.schedule.after(&lookback_time).peekable();
        loop {
            let start_time = match start_times.next() {
                Some(start_time) => start_time,
                None => return Ok(()),
            };
            sleep_until(&start_time).await;
            lamp_controller.lock().unwrap().enable(&self.mode)?;

            let end_time = start_time + self.duration;
            let should_disable = match start_times.peek() {
                Some(next_start_time) => &end_time < next_start_time,
                None => true,
            };
            if should_disable {
                sleep_until(&end_time).await;
                lamp_controller.lock().unwrap().disable(&self.mode)?;
            }
        }
    }
}

/// Sleep until `time`. If `time` is in the past, do nothing.
async fn sleep_until(time: &chrono::DateTime<chrono::Local>) {
    if let Ok(sleep_time) = (*time - chrono::Local::now()).to_std() {
        tokio::time::sleep(sleep_time).await;
    }
}
