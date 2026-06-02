//! Współdzielony stan aplikacji przechowywany w Tauri managed state.

use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::Mutex;
use tauri::AppHandle;

use crate::history::HistoryStore;
use crate::models::ModelManager;
use crate::settings::SettingsStore;

pub struct AppState {
    pub app_data_dir: PathBuf,
    pub models_dir: PathBuf,
    pub recordings_dir: PathBuf,
    pub settings: Arc<SettingsStore>,
    pub models: Arc<ModelManager>,
    pub history: Arc<Mutex<HistoryStore>>,
    /// Aktualny poziom głośności mikrofonu (0.0 - 1.0), aktualizowany przez capture thread.
    pub mic_level: Arc<Mutex<f32>>,
}

impl AppState {
    pub fn new(_app: AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let app_data_dir = dirs_app_data()
            .ok_or("Nie udało się ustalić katalogu danych aplikacji")?;
        let models_dir = app_data_dir.join("models");
        let recordings_dir = app_data_dir.join("recordings");
        std::fs::create_dir_all(&app_data_dir)?;
        std::fs::create_dir_all(&models_dir)?;
        std::fs::create_dir_all(&recordings_dir)?;

        let settings = Arc::new(SettingsStore::load_or_default(&app_data_dir)?);
        let models = Arc::new(ModelManager::new(models_dir.clone())?.with_app(_app.clone()));
        let history = Arc::new(Mutex::new(HistoryStore::open(&app_data_dir)?));

        Ok(Self {
            app_data_dir,
            models_dir,
            recordings_dir,
            settings,
            models,
            history,
            mic_level: Arc::new(Mutex::new(0.0)),
        })
    }
}

pub fn dirs_app_data() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        std::env::var("HOME").ok().map(|h| {
            PathBuf::from(h)
                .join("Library")
                .join("Application Support")
                .join("pl.gadaj.app")
        })
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA").ok().map(|p| PathBuf::from(p).join("pl.gadaj.app"))
    }
    #[cfg(target_os = "linux")]
    {
        std::env::var("XDG_DATA_HOME")
            .ok()
            .map(|p| PathBuf::from(p).join("gadaj"))
            .or_else(|| {
                std::env::var("HOME").ok().map(|h| {
                    PathBuf::from(h).join(".local").join("share").join("gadaj")
                })
            })
    }
}
