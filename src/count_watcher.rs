use futures::channel::mpsc::Receiver;
use futures::{FutureExt, SinkExt, StreamExt};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use notify::{RecommendedWatcher, Watcher};

use crate::config::CountFileConfig;
use crate::lamp_controller::LampController;

type NotifyResult = notify::Result<notify::Event>;

#[derive(Debug, Clone)]
pub struct CountWatcher {
    modes: BTreeMap<PathBuf, String>,
}

impl CountWatcher {
    pub fn new(configs: Vec<CountFileConfig>) -> Self {
        let mut modes = BTreeMap::new();
        for config in configs {
            modes.insert(config.file, config.mode);
        }

        Self { modes }
    }

    pub async fn run(&self, lamp_controller: Arc<Mutex<LampController>>) -> Result<()> {
        let (_watcher, mut rx) = self.build_watcher()?;

        // Update all watched files on first run
        let mut touched_paths: BTreeSet<PathBuf> = self.modes.keys().cloned().collect();
        loop {
            for path in &touched_paths {
                if let Err(error) = self.update_path(path, &lamp_controller) {
                    eprintln!("Failed to update {:?}: {:?}", path, error);
                }
            }
            touched_paths = self.wait_for_changes(&mut rx).await?;
        }
    }

    /// Get a watcher and receiver for the files we want to monitor.
    fn build_watcher(&self) -> Result<(RecommendedWatcher, Receiver<NotifyResult>)> {
        // If we just watch the paths, then if the file is deleted and recreated we'll no longer be
        // watching the inodes. By watching the containing directory instead this will continue to
        // work unless the directory itself is removed.
        let mut watch_dirs = BTreeSet::new();
        for path in self.modes.keys() {
            watch_dirs.insert(path.parent().unwrap_or_else(|| std::path::Path::new("/")));
        }

        let (mut tx, rx) = futures::channel::mpsc::channel(1);
        let mut watcher = RecommendedWatcher::new(
            move |res| {
                futures::executor::block_on(async {
                    tx.send(res).await.unwrap();
                })
            },
            notify::Config::default(),
        )?;
        for dir in watch_dirs {
            watcher.watch(dir, notify::RecursiveMode::NonRecursive)?;
        }
        Ok((watcher, rx))
    }

    /// Wait for changes to the watched files and then return the set of changed files.
    async fn wait_for_changes(&self, rx: &mut Receiver<NotifyResult>) -> Result<BTreeSet<PathBuf>> {
        let mut results = vec![rx.next().await.expect("inotify event stream ended")];
        std::thread::sleep(std::time::Duration::from_millis(10));
        while let Some(result) = rx.next().now_or_never() {
            results.push(result.expect("inotify event stream ended"));
        }

        let mut touched_paths = BTreeSet::new();
        for result in results {
            match result {
                Ok(event) => {
                    for path in event.paths {
                        if self.modes.contains_key(&path) {
                            touched_paths.insert(path);
                        }
                    }
                }
                Err(error) => eprintln!("Error watching files: {:?}", error),
            }
        }
        Ok(touched_paths)
    }

    fn update_path(
        &self,
        path: &std::path::Path,
        lamp_controller: &Arc<Mutex<LampController>>,
    ) -> Result<()> {
        let text = std::fs::read_to_string(path)?;
        let count: u64 = text.trim().parse().unwrap_or(0);
        let mode = self
            .modes
            .get(path)
            .ok_or_else(|| anyhow::anyhow!("Failed to get mode for {:?}", path))?;
        let mut lamp_controller = lamp_controller.lock().expect("Failed to unlock mutex");
        if count > 0 {
            lamp_controller.enable(mode)
        } else {
            lamp_controller.disable(mode)
        }
    }
}
