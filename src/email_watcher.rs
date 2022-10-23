use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use notify::{RecommendedWatcher, Watcher};

use crate::config::EmailConfig;
use crate::lamp_controller::LampController;

type NotifyResult = notify::Result<notify::Event>;

#[derive(Debug, Clone)]
pub struct EmailFileWatcher {
    modes: BTreeMap<PathBuf, String>,
}

impl EmailFileWatcher {
    pub fn new(configs: Vec<EmailConfig>) -> Self {
        let mut modes = BTreeMap::new();
        for config in configs {
            modes.insert(config.file, config.mode);
        }

        Self { modes }
    }

    pub async fn run(&self, lamp_controller: Arc<Mutex<LampController>>) -> Result<()> {
        let (tx, rx) = std::sync::mpsc::channel();
        let _watcher = self.build_watcher(tx)?;

        // Update all watched files on first run
        let mut touched_paths: BTreeSet<PathBuf> = self.modes.keys().cloned().collect();
        loop {
            for path in &touched_paths {
                if let Err(error) = self.update_path(path, &lamp_controller) {
                    eprintln!("Failed to update {:?}: {:?}", path, error);
                }
            }
            touched_paths = self.wait_for_changes(&rx)?;
        }
    }

    /// Wait for changes to the watched files and then return the set of changed files.
    fn wait_for_changes(&self, rx: &Receiver<NotifyResult>) -> Result<BTreeSet<PathBuf>> {
        let mut results = vec![rx.recv()?];
        std::thread::sleep(std::time::Duration::from_millis(10));
        results.extend(rx.try_iter());
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

    fn build_watcher(&self, tx: Sender<NotifyResult>) -> Result<RecommendedWatcher> {
        // If we just watch the paths, then if the file is deleted and recreated we'll no longer be
        // watching the inodes. By watching the containing directory instead this will continue to
        // work unless the directory itself is removed.
        let mut watch_dirs = BTreeSet::new();
        for path in self.modes.keys() {
            watch_dirs.insert(path.parent().unwrap_or_else(|| std::path::Path::new("/")));
        }

        let mut watcher = RecommendedWatcher::new(tx, notify::Config::default())?;
        for dir in watch_dirs {
            watcher.watch(dir, notify::RecursiveMode::NonRecursive)?;
        }
        Ok(watcher)
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
