use crate::storage::Store;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct AppState {
    pub store: Mutex<Store>,
    pub watcher: Mutex<Option<notify::RecommendedWatcher>>,
    pub data_dir: PathBuf,
}

impl AppState {
    pub fn new(store: Store, data_dir: PathBuf) -> Self {
        Self {
            store: Mutex::new(store),
            watcher: Mutex::new(None),
            data_dir,
        }
    }
}
