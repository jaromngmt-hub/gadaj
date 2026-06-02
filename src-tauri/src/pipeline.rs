//! Orkiestrator pipeline'u: stan maszyny, koordynacja audio → STT → paste.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Instant;

use parking_lot::Mutex;
use tauri::{AppHandle, Emitter, Manager};

use crate::audio::capture::{self, RecordingSession, TARGET_SAMPLE_RATE};
use crate::audio::vad;
use crate::input::hotkey::HotkeyListener;
use crate::input::paste;
use crate::state::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Idle,
    Recording,
    Transcribing,
    Pasting,
    Error,
}

impl State {
    fn as_str(self) -> &'static str {
        match self {
            State::Idle => "idle",
            State::Recording => "recording",
            State::Transcribing => "transcribing",
            State::Pasting => "pasting",
            State::Error => "error",
        }
    }
}

pub struct Pipeline {
    state: Arc<Mutex<State>>,
    session: Arc<Mutex<Option<RecordingSession>>>,
    hotkey: Arc<HotkeyListener>,
    enabled: Arc<AtomicBool>,
}

impl Pipeline {
    pub fn new(app: AppHandle) -> Self {
        let state = Arc::new(Mutex::new(State::Idle));
        let session: Arc<Mutex<Option<RecordingSession>>> = Arc::new(Mutex::new(None));
        let hotkey = Arc::new(HotkeyListener::new());

        let app_state = app.state::<Arc<AppState>>();
        let settings = app_state.settings.get();
        if let Some(key) = settings.hotkey.clone() {
            hotkey.set_key(key);
        }

        // Pierwszy callback hotkeya (tworzy własny klon arców)
        let app_h = app.clone();
        let state_h = state.clone();
        let session_h = session.clone();
        let on_change: Arc<dyn Fn(bool) + Send + Sync + 'static> = Arc::new(move |pressed: bool| {
            if pressed {
                if let Err(e) = Self::start_recording(&app_h, &state_h, &session_h) {
                    log::error!("start_recording: {e}");
                    Self::emit_state(&app_h, State::Error);
                }
            } else {
                Self::stop_and_process(&app_h, &state_h, &session_h);
            }
        });

        if settings.hotkey.is_some() {
            if let Err(e) = hotkey.start(on_change) {
                log::warn!("Nie udało się uruchomić hotkey listener: {e}");
            }
        }

        Self {
            state,
            session,
            hotkey,
            enabled: Arc::new(AtomicBool::new(settings.hotkey.is_some())),
        }
    }

    pub fn current_state(&self) -> State {
        *self.state.lock()
    }

    pub fn enable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
    }

    pub fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
        if let Some(s) = self.session.lock().take() {
            s.running.store(false, Ordering::SeqCst);
        }
    }

    pub fn update_hotkey(&self, key: String) -> Result<(), String> {
        self.hotkey.set_key(key);
        Ok(())
    }

    pub fn hotkey_ref(&self) -> &Arc<HotkeyListener> {
        &self.hotkey
    }

    pub fn start_hotkey(&self, on_change: Arc<dyn Fn(bool) + Send + Sync + 'static>) -> Result<(), String> {
        self.hotkey.start(on_change)
    }

    pub fn stop_hotkey(&self) {
        self.hotkey.stop();
    }

    fn start_recording(
        app: &AppHandle,
        state: &Arc<Mutex<State>>,
        session_slot: &Arc<Mutex<Option<RecordingSession>>>,
    ) -> Result<(), String> {
        if *state.lock() != State::Idle {
            log::debug!("Pipeline nie jest Idle - ignoruję start");
            return Ok(());
        }
        let app_state = app.state::<Arc<AppState>>().inner().clone();
        let session = capture::start_recording(app_state)
            .map_err(|e| format!("Audio: {e}"))?;
        *session_slot.lock() = Some(session);
        *state.lock() = State::Recording;
        Self::emit_state(app, State::Recording);
        Ok(())
    }

    fn stop_and_process(
        app: &AppHandle,
        state: &Arc<Mutex<State>>,
        session_slot: &Arc<Mutex<Option<RecordingSession>>>,
    ) {
        let session = match session_slot.lock().take() {
            Some(s) => s,
            None => return,
        };

        *state.lock() = State::Transcribing;
        Self::emit_state(app, State::Transcribing);

        let app_clone = app.clone();
        let state_clone = state.clone();
        thread::spawn(move || {
            // Zatrzymaj strumień i poczekaj na koniec wątku audio
            let (samples, src_rate, src_channels) = match session.stop_and_join() {
                Ok(v) => v,
                Err(e) => {
                    log::error!("stop_and_join: {e}");
                    *state_clone.lock() = State::Error;
                    Self::emit_state(&app_clone, State::Error);
                    return;
                }
            };

            if samples.is_empty() {
                log::info!("Puste nagranie - ignoruję");
                *state_clone.lock() = State::Idle;
                Self::emit_state(&app_clone, State::Idle);
                return;
            }

            let started = Instant::now();

            // 1. Konwersja do f32 mono + resample do 16kHz
            let mut pcm = capture::buffer_to_f32(&samples, src_rate, src_channels);
            if pcm.is_empty() {
                *state_clone.lock() = State::Idle;
                Self::emit_state(&app_clone, State::Idle);
                return;
            }

            // 2. VAD - odcinanie ciszy
            pcm = vad::trim_silence(&pcm, TARGET_SAMPLE_RATE);
            if pcm.is_empty() {
                log::info!("Po VAD pusto - brak mowy");
                *state_clone.lock() = State::Idle;
                Self::emit_state(&app_clone, State::Idle);
                return;
            }

            // 3. Transkrypcja
            let engine_arc = crate::stt::parakeet::shared_engine();
            let app_state = app_clone.state::<Arc<AppState>>().inner().clone();

            // Lazy load modelu
            {
                let mut eng = engine_arc.lock();
                if !eng.is_loaded() {
                    let model_id = app_state.settings.get().model_id;
                    if let Some(path) = app_state.models.model_path(&model_id) {
                        if let Err(e) = eng.load(&path) {
                            log::error!("load model: {e}");
                            *state_clone.lock() = State::Error;
                            Self::emit_state(&app_clone, State::Error);
                            return;
                        }
                    } else {
                        log::error!("Nieznany model: {model_id}");
                        *state_clone.lock() = State::Error;
                        Self::emit_state(&app_clone, State::Error);
                        return;
                    }
                }
            }

            let text = {
                let eng = engine_arc.lock();
                match eng.transcribe(&pcm, TARGET_SAMPLE_RATE) {
                    Ok(t) => t,
                    Err(e) => {
                        log::error!("transcribe: {e}");
                        *state_clone.lock() = State::Error;
                        Self::emit_state(&app_clone, State::Error);
                        return;
                    }
                }
            };

            if text.is_empty() {
                log::info!("Pusta transkrypcja");
                *state_clone.lock() = State::Idle;
                Self::emit_state(&app_clone, State::Idle);
                return;
            }

            // 4. Zapis do historii
            let duration_ms = (pcm.len() as f64 / TARGET_SAMPLE_RATE as f64 * 1000.0) as i64;
            let language = app_state.settings.get().language;
            let lang_opt = if language == "auto" { None } else { Some(language.as_str()) };
            if let Err(e) = app_state.history.lock().insert(&text, None, lang_opt, duration_ms) {
                log::error!("history insert: {e}");
            }

            // 5. Emit do frontendu
            let _ = app_clone.emit("transcription", &text);

            // 6. Paste
            *state_clone.lock() = State::Pasting;
            Self::emit_state(&app_clone, State::Pasting);
            let paste_method = app_state.settings.get().paste_method;
            let result = match paste_method.as_str() {
                "clipboard" => paste::copy_to_clipboard(&text),
                _ => paste::paste_text(&text),
            };
            if let Err(e) = result {
                log::error!("paste: {e}");
            }

            log::info!(
                "Transkrypcja zakończona w {:.2}s ({} znaków): {:?}",
                started.elapsed().as_secs_f64(),
                text.len(),
                text
            );

            *state_clone.lock() = State::Idle;
            Self::emit_state(&app_clone, State::Idle);
        });
    }

    fn emit_state(app: &AppHandle, state: State) {
        let _ = app.emit("pipeline-state", state.as_str());
    }
}
