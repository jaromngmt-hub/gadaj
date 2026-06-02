//! Przechwytywanie audio z mikrofonu przez cpal.
//!
//! cpal::Stream nie jest Send+Sync na macOS (CoreAudio wymaga stałego wątku),
//! więc stream żyje w dedykowanym wątku audio, a my trzymamy tylko uchwyt
//! (JoinHandle + flaga + bufor próbek) który jest Send+Sync.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleRate, StreamConfig};
use parking_lot::Mutex;

use crate::state::AppState;

#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("Brak dostępnego urządzenia wejściowego")]
    NoInputDevice,
    #[error("Nie udało się uzyskać domyślnej konfiguracji strumienia")]
    NoStreamConfig,
    #[error("Błąd cpal: {0}")]
    Cpal(#[from] cpal::BuildStreamError),
    #[error("Błąd odtwarzania strumienia: {0}")]
    PlayStream(#[from] cpal::PlayStreamError),
    #[error("Już trwa nagrywanie")]
    AlreadyRecording,
    #[error("Nie udało się uruchomić wątku audio: {0}")]
    SpawnThread(String),
}

pub const TARGET_SAMPLE_RATE: u32 = 16_000;
pub const TARGET_CHANNELS: u16 = 1;

/// Uchwyt do sesji nagrywania. `Send + Sync`.
pub struct RecordingSession {
    pub running: Arc<AtomicBool>,
    pub samples: Arc<Mutex<Vec<i16>>>,
    pub sample_rate: u32,
    pub channels: u16,
    pub thread: Option<JoinHandle<()>>,
}

impl RecordingSession {
    /// Sygnalizuje wątkowi audio żeby zakończył strumień i czeka aż to zrobi.
    pub fn stop_and_join(mut self) -> Result<(Vec<i16>, u32, u16), AudioError> {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.thread.take() {
            // Wątek kończy się gdy strumień zostanie dropnięty.
            // Daj mu chwilę na cleanup.
            let _ = handle.join();
        }
        let samples = self.samples.lock().clone();
        Ok((samples, self.sample_rate, self.channels))
    }
}

/// Uruchamia nagrywanie w osobnym wątku i zwraca uchwyt.
pub fn start_recording(state: Arc<AppState>) -> Result<RecordingSession, AudioError> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or(AudioError::NoInputDevice)?;

    let supported = device
        .default_input_config()
        .map_err(|_| AudioError::NoStreamConfig)?;

    let sample_rate = supported.sample_rate().0;
    let channels = supported.channels();
    log::info!("Audio capture: {sample_rate}Hz, {channels} channels");

    let config = StreamConfig {
        channels: channels as u16,
        sample_rate: SampleRate(sample_rate),
        buffer_size: cpal::BufferSize::Default,
    };

    let samples = Arc::new(Mutex::new(Vec::<i16>::with_capacity(
        sample_rate as usize * 60,
    )));
    let samples_cb = samples.clone();
    let mic_level = state.mic_level.clone();
    let running = Arc::new(AtomicBool::new(true));
    let running_cb = running.clone();
    let running_out = running.clone();

    // Kanał do przekazania strumienia do wątku audio (gdzie zostanie zdropnięty).
    let (stream_tx, stream_rx) = std::sync::mpsc::channel::<LocalStream>();

    let thread = thread::Builder::new()
        .name("gadaj-audio-capture".into())
        .spawn(move || {
            let stream_result = device.build_input_stream(
                &config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    if !running_cb.load(Ordering::Relaxed) {
                        return;
                    }
                    let mono: Vec<i16> = if channels > 1 {
                        data.chunks(channels as usize).map(|c| c[0]).collect()
                    } else {
                        data.to_vec()
                    };

                    if !mono.is_empty() {
                        let sum_sq: f64 = mono
                            .iter()
                            .map(|s| (*s as f64 / 32768.0).powi(2))
                            .sum();
                        let rms = (sum_sq / mono.len() as f64).sqrt();
                        let level = (rms * 4.0).min(1.0) as f32;
                        *mic_level.lock() = level;
                    }
                    samples_cb.lock().extend_from_slice(&mono);
                },
                move |err| {
                    log::error!("Błąd strumienia audio: {err}");
                },
                None,
            );

            match stream_result {
                Ok(stream) => {
                    if let Err(e) = stream.play() {
                        log::error!("Nie mogę uruchomić strumienia: {e}");
                        return;
                    }
                    // Opakowujemy stream w Send wrapper, żeby mógł przejść przez kanał.
                    // BEZPIECZNE: stream pozostaje w tym wątku (odbieramy go w tym samym wątku).
                    let _ = stream_tx.send(LocalStream(stream));
                    drop(stream_rx); // zamknij kanał po stronie wysyłającej
                    while running.load(Ordering::Relaxed) {
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                    // LocalStream zostanie zdropnięty tutaj, w tym samym wątku
                }
                Err(e) => {
                    log::error!("build_input_stream: {e}");
                }
            }
        })
        .map_err(|e| AudioError::SpawnThread(e.to_string()))?;

    Ok(RecordingSession {
        running: running_out,
        samples,
        sample_rate,
        channels: channels as u16,
        thread: Some(thread),
    })
}

/// Send wrapper dla cpal::Stream - BEZPIECZNE bo:
/// - Stream nigdy nie opuszcza wątku, w którym został stworzony.
/// - Jest przechowywany lokalnie w scope i dropnięty na końcu.
/// - Unsafe impl Send jest konieczny bo cpal::Stream na macOS (CoreAudio)
///   wewnętrznie zawiera *mut () dla AudioObjectPropertyListener, który nie
///   jest Send. My gwarantujemy, że ten stream nigdy nie jest przenoszony
///   między wątkami.
struct LocalStream(cpal::Stream);
unsafe impl Send for LocalStream {}
unsafe impl Sync for LocalStream {}

/// Konwertuje i16 PCM do f32 normalized [-1.0, 1.0] w TARGET_SAMPLE_RATE mono.
pub fn buffer_to_f32(buf: &[i16], src_rate: u32, src_channels: u16) -> Vec<f32> {
    let mut mono: Vec<f32> = if src_channels > 1 {
        buf.chunks(src_channels as usize)
            .map(|c| c[0] as f32 / 32768.0)
            .collect()
    } else {
        buf.iter().map(|s| *s as f32 / 32768.0).collect()
    };

    if src_rate != TARGET_SAMPLE_RATE && !mono.is_empty() {
        mono = crate::stt::resample::resample_linear(&mono, src_rate, TARGET_SAMPLE_RATE);
    }
    mono
}
