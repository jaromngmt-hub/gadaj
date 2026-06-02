//! Trwałe ustawienia użytkownika w JSON w katalogu danych aplikacji.

use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Skrót dyktowania w formacie np. "Alt+Left", "F13", "Alt+Space"
    #[serde(default)]
    pub hotkey: Option<String>,

    /// Identyfikator aktywnego modelu
    #[serde(default = "default_model_id")]
    pub model_id: String,

    /// Kod języka: "auto", "pl", "en", ...
    #[serde(default = "default_language")]
    pub language: String,

    /// "auto" = symuluj Cmd/Ctrl+V, "clipboard" = tylko schowek
    #[serde(default = "default_paste_method")]
    pub paste_method: String,

    /// "pl" lub "en"
    #[serde(default = "default_ui_language")]
    pub ui_language: String,
}

fn default_model_id() -> String {
    "parakeet-tdt-0.6b-v3".to_string()
}
fn default_language() -> String {
    "auto".to_string()
}
fn default_paste_method() -> String {
    "auto".to_string()
}
fn default_ui_language() -> String {
    "pl".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: None,
            model_id: default_model_id(),
            language: default_language(),
            paste_method: default_paste_method(),
            ui_language: default_ui_language(),
        }
    }
}

pub struct SettingsStore {
    path: std::path::PathBuf,
    pub current: parking_lot::RwLock<Settings>,
}

impl SettingsStore {
    pub fn load_or_default(dir: &Path) -> Result<Self, std::io::Error> {
        let path = dir.join("settings.json");
        let current = if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
                Err(_) => Settings::default(),
            }
        } else {
            Settings::default()
        };
        Ok(Self {
            path,
            current: parking_lot::RwLock::new(current),
        })
    }

    pub fn get(&self) -> Settings {
        self.current.read().clone()
    }

    pub fn set(&self, new: Settings) -> Result<(), std::io::Error> {
        {
            let mut w = self.current.write();
            *w = new.clone();
        }
        let s = serde_json::to_string_pretty(&new).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e)
        })?;
        std::fs::write(&self.path, s)
    }
}
