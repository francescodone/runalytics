use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use crate::commands::Settings;
use crate::providers::Provider;
use crate::store::Store;

pub struct AppState {
    pub store: Store,
    pub settings: Mutex<Settings>,
    pub providers: Vec<Arc<dyn Provider>>,
    pub data_dir: PathBuf,
    /// True while a sync (manual or auto) is in flight. The frontend shows
    /// a spinner in the nav bar based on this + the sync events.
    pub syncing: AtomicBool,
    /// Held for the duration of a sync; concurrent syncs queue behind it.
    pub sync_lock: tokio::sync::Mutex<()>,
    pub calendar_lock: tokio::sync::Mutex<()>,
}
