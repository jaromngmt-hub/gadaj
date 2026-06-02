//! Menedżer modeli: lista dostępnych modeli, pobieranie, walidacja.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::stt::parakeet::{
    PARAKEET_V3_FILENAME, PARAKEET_V3_SIZE_BYTES, PARAKEET_V3_URL,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDescriptor {
    pub id: String,
    pub name: String,
    pub description: String,
    pub size_bytes: u64,
    pub url: String,
    pub filename: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub size_bytes: u64,
    pub downloaded: bool,
    pub downloading: bool,
    pub progress: u8,
}

#[derive(Debug, Clone, Serialize)]
struct DownloadProgress {
    id: String,
    progress: u8,
    downloaded_bytes: u64,
    total_bytes: u64,
}

pub struct ModelManager {
    models_dir: PathBuf,
    progress: Arc<Mutex<std::collections::HashMap<String, u8>>>,
    app: Option<AppHandle>,
}

impl ModelManager {
    pub fn new(models_dir: PathBuf) -> Result<Self, std::io::Error> {
        std::fs::create_dir_all(&models_dir)?;
        Ok(Self {
            models_dir,
            progress: Arc::new(Mutex::new(Default::default())),
            app: None,
        })
    }

    pub fn with_app(mut self, app: AppHandle) -> Self {
        self.app = Some(app);
        self
    }

    pub fn list(&self) -> Vec<ModelInfo> {
        let descriptors = self.descriptors();
        descriptors
            .into_iter()
            .map(|d| {
                let path = self.models_dir.join(&d.filename);
                let downloaded = path.exists() && self.is_valid(&path);
                let progress = *self.progress.lock().get(&d.id).unwrap_or(&0);
                ModelInfo {
                    id: d.id,
                    name: d.name,
                    description: d.description,
                    size_bytes: d.size_bytes,
                    downloaded,
                    downloading: progress > 0 && progress < 100,
                    progress,
                }
            })
            .collect()
    }

    pub fn descriptor(&self, id: &str) -> Option<ModelDescriptor> {
        self.descriptors().into_iter().find(|d| d.id == id)
    }

    pub fn model_path(&self, id: &str) -> Option<PathBuf> {
        self.descriptor(id).map(|d| self.models_dir.join(&d.filename))
    }

    pub fn descriptors(&self) -> Vec<ModelDescriptor> {
        vec![ModelDescriptor {
            id: "parakeet-tdt-0.6b-v3".to_string(),
            name: "Parakeet TDT 0.6B V3 (wielojęzyczny)".to_string(),
            description: "Polski + 25 języków. Wysoka jakość, ~675MB (Q4_K).".to_string(),
            size_bytes: PARAKEET_V3_SIZE_BYTES,
            url: PARAKEET_V3_URL.to_string(),
            filename: PARAKEET_V3_FILENAME.to_string(),
        }]
    }

    fn is_valid(&self, path: &Path) -> bool {
        std::fs::metadata(path)
            .map(|m| m.len() > 1024 * 1024)
            .unwrap_or(false)
    }

    fn emit_progress(&self, id: &str, pct: u8, downloaded: u64, total: u64) {
        if let Some(app) = &self.app {
            let _ = app.emit(
                "model-download-progress",
                DownloadProgress {
                    id: id.to_string(),
                    progress: pct,
                    downloaded_bytes: downloaded,
                    total_bytes: total,
                },
            );
        }
    }

    /// Pobiera model w sposób blokujący. Aktualizuje `progress` i emituje eventy.
    pub async fn download(&self, id: &str) -> Result<(), String> {
        let desc = self
            .descriptor(id)
            .ok_or_else(|| format!("Nieznany model: {id}"))?;
        let dest = self.models_dir.join(&desc.filename);
        let tmp = self.models_dir.join(format!("{}.part", desc.filename));

        log::info!("Pobieram model {} z {}", desc.id, desc.url);
        self.progress.lock().insert(desc.id.clone(), 0);
        self.emit_progress(&desc.id, 0, 0, 0);

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60 * 30))
            .build()
            .map_err(|e| e.to_string())?;

        let response = client
            .get(&desc.url)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {e}"))?;
        if !response.status().is_success() {
            return Err(format!("HTTP {}: {}", response.status(), desc.url));
        }

        let total = response.content_length().unwrap_or(desc.size_bytes);
        let mut file = tokio::fs::File::create(&tmp)
            .await
            .map_err(|e| format!("Nie mogę utworzyć pliku: {e}"))?;
        let mut downloaded: u64 = 0;
        let mut last_report: u8 = 0;

        use futures_util::StreamExt;
        use tokio::io::AsyncWriteExt;
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Błąd pobierania: {e}"))?;
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("Błąd zapisu: {e}"))?;
            downloaded += chunk.len() as u64;
            let pct = ((downloaded as f64 / total as f64) * 100.0) as u8;
            if pct != last_report {
                last_report = pct;
                self.progress.lock().insert(desc.id.clone(), pct);
                self.emit_progress(&desc.id, pct, downloaded, total);
                log::debug!("Download progress: {}% ({} / {} bytes)", pct, downloaded, total);
            }
        }
        file.flush().await.map_err(|e| e.to_string())?;
        drop(file);

        tokio::fs::rename(&tmp, &dest)
            .await
            .map_err(|e| format!("Nie mogę sfinalizować pliku: {e}"))?;

        self.progress.lock().insert(desc.id.clone(), 100);
        self.emit_progress(&desc.id, 100, total, total);
        log::info!("Pobrano model: {}", dest.display());
        Ok(())
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        let path = self
            .model_path(id)
            .ok_or_else(|| format!("Nieznany model: {id}"))?;
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        self.progress.lock().remove(id);
        Ok(())
    }
}

/// Globalny, leniwy model manager.
pub fn shared_manager() -> &'static Arc<ModelManager> {
    use once_cell::sync::Lazy;
    static MGR: Lazy<Arc<ModelManager>> = Lazy::new(|| {
        let dir = crate::state::dirs_app_data()
            .map(|p| p.join("models"))
            .unwrap_or_else(|| std::path::PathBuf::from("./models"));
        Arc::new(
            ModelManager::new(dir).expect("Nie mogę utworzyć katalogu modeli"),
        )
    });
    &MGR
}
