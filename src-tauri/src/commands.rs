//! Tauri commands - punkty wejścia z frontendu React.

use std::sync::Arc;
use tauri::{Emitter, Manager, State};

use crate::history::HistoryEntry;
use crate::input::paste;
use crate::models::ModelInfo;
use crate::pipeline::Pipeline;
use crate::settings::Settings;
use crate::state::AppState;

#[tauri::command]
pub fn get_settings(state: State<'_, Arc<AppState>>) -> Settings {
    state.settings.get()
}

#[tauri::command]
pub fn save_settings(
    state: State<'_, Arc<AppState>>,
    pipeline: State<'_, Pipeline>,
    settings: Settings,
) -> Result<(), String> {
    let prev = state.settings.get();
    state.settings.set(settings.clone()).map_err(|e| e.to_string())?;

    if settings.hotkey != prev.hotkey {
        if let Some(key) = settings.hotkey.clone() {
            pipeline.update_hotkey(key)?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn get_available_models(state: State<'_, Arc<AppState>>) -> Vec<ModelInfo> {
    state.models.list()
}

#[tauri::command]
pub async fn download_model(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    state.models.download(&id).await
}

#[tauri::command]
pub fn delete_model(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    state.models.delete(&id)
}

#[tauri::command]
pub fn get_history_entries(
    state: State<'_, Arc<AppState>>,
    query: Option<String>,
) -> Result<Vec<HistoryEntry>, String> {
    let q = query.as_deref();
    state
        .history
        .lock()
        .list(q, 200)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_history_entry(state: State<'_, Arc<AppState>>, id: i64) -> Result<(), String> {
    state.history.lock().delete(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn copy_to_clipboard(text: String) -> Result<(), String> {
    paste::copy_to_clipboard(&text)
}

#[tauri::command]
pub fn get_mic_level(state: State<'_, Arc<AppState>>) -> f32 {
    *state.mic_level.lock()
}

#[tauri::command]
pub fn is_model_loaded() -> bool {
    crate::stt::parakeet::shared_engine().lock().is_loaded()
}

#[tauri::command]
pub async fn transcribe_file(
    state: State<'_, Arc<AppState>>,
    file_path: String,
) -> Result<String, String> {
    use std::path::Path;

    let path = Path::new(&file_path);
    let bytes = std::fs::read(path).map_err(|e| format!("Nie mogę odczytać pliku: {e}"))?;
    let mut reader = hound::WavReader::new(std::io::Cursor::new(&bytes))
        .map_err(|e| format!("Nieprawidłowy WAV: {e}"))?;
    let spec = reader.spec();
    let samples: Vec<i16> = reader
        .samples::<i16>()
        .collect::<Result<_, _>>()
        .map_err(|e| format!("Błąd dekodowania WAV: {e}"))?;

    let mut mono: Vec<f32> = if spec.channels > 1 {
        samples
            .chunks(spec.channels as usize)
            .map(|c| c[0] as f32 / 32768.0)
            .collect()
    } else {
        samples.iter().map(|s| *s as f32 / 32768.0).collect()
    };

    if spec.sample_rate != crate::audio::capture::TARGET_SAMPLE_RATE {
        mono = crate::stt::resample::resample_linear(
            &mono,
            spec.sample_rate,
            crate::audio::capture::TARGET_SAMPLE_RATE,
        );
    }

    let engine = crate::stt::parakeet::shared_engine();
    let mut eng = engine.lock();

    if !eng.is_loaded() {
        let model_id = state.settings.get().model_id;
        let path = state
            .models
            .model_path(&model_id)
            .ok_or_else(|| format!("Nieznany model: {model_id}"))?;
        eng.load(&path)?;
    }

    eng.transcribe(&mono, crate::audio::capture::TARGET_SAMPLE_RATE)
}

#[tauri::command]
pub fn start_hotkey_listener(
    pipeline: State<'_, Pipeline>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let state = app.state::<Arc<AppState>>();
    let settings = state.settings.get();
    let key = settings.hotkey.ok_or("Nie ustawiono klawisza")?;

    let app_h = app.clone();
    let on_change: Arc<dyn Fn(bool) + Send + Sync> = Arc::new(move |pressed: bool| {
        if pressed {
            log::info!("[hotkey] pressed");
            let _ = app_h.emit("hotkey-press", true);
        } else {
            log::info!("[hotkey] released");
            let _ = app_h.emit("hotkey-press", false);
        }
    });

    pipeline.hotkey_ref().set_key(key);
    pipeline.start_hotkey(on_change)
}

#[tauri::command]
pub fn stop_hotkey_listener(pipeline: State<'_, Pipeline>) {
    pipeline.stop_hotkey();
}

#[tauri::command]
pub fn show_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("main") {
        w.show().map_err(|e| e.to_string())?;
        w.unminimize().map_err(|e| e.to_string())?;
        w.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_app_data_dir(state: State<'_, Arc<AppState>>) -> String {
    state.app_data_dir.to_string_lossy().into_owned()
}
