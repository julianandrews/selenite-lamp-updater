mod config;
mod count_watcher;
mod lamp_controller;
mod timer;

use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use clap::{AppSettings, Parser};

use count_watcher::CountWatcher;
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
    controller.lock().unwrap().enable(&config.default_mode)?;

    // TODO: print a message when futures complete, and maybe refactor this mess.
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

    let count_watcher = CountWatcher::new(config.count_files.clone());
    let count_watcher_future = count_watcher.run(controller.clone());

    let (timer_results, count_watcher_result) =
        futures::future::join(timer_futures, count_watcher_future).await;

    for result in timer_results {
        result?;
    }
    count_watcher_result?;

    Ok(())
}

#[derive(Parser, Debug, Clone)]
#[clap(version, setting=AppSettings::DeriveDisplayOrder)]
pub struct Args {
    pub file: Option<std::path::PathBuf>,
    #[clap(short, long)]
    pub configfile: std::path::PathBuf,
}
