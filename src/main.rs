mod config;
mod email_watcher;
mod lamp_controller;
mod timer;

use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use clap::{AppSettings, Parser};

use email_watcher::EmailFileWatcher;
use lamp_controller::LampController;
use timer::Timer;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let config = config::load(&args.configfile).context("Failed to load config")?;
    let file = match args.file.or(config.output_file) {
        Some(file) => file,
        None => {
            return Err(anyhow::anyhow!(
                "Must specify output file in either arguments or config."
            ))
        }
    };
    let controller = Arc::new(Mutex::new(LampController::new(file, &config.modes)));
    controller
        .lock()
        .expect("Failed to unlock mutex")
        .enable(&config.default_mode)?;

    let timers: Vec<_> = config
        .timers
        .into_iter()
        .map(|timer| Timer::new(timer))
        .collect();
    let timer_futures: Vec<_> = timers
        .iter()
        .map(|timer| timer.run(controller.clone()))
        .collect();
    let timer_futures = futures::future::join_all(timer_futures);

    let email_watcher = EmailFileWatcher::new(config.email_files.clone());
    let email_watcher_future = email_watcher.run(controller.clone());

    let (timer_results, email_watcher_result) =
        futures::future::join(timer_futures, email_watcher_future).await;

    for result in timer_results {
        result?;
    }
    email_watcher_result?;

    Ok(())
}

#[derive(Parser, Debug, Clone)]
#[clap(version, setting=AppSettings::DeriveDisplayOrder)]
pub struct Args {
    pub file: Option<std::path::PathBuf>,
    #[clap(short, long)]
    pub configfile: std::path::PathBuf,
}
