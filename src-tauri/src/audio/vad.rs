//! Prosta detekcja aktywności głosowej (VAD) oparta na RMS energii.
//!
//! W produkcyjnym użyciu warto by podmienić to na Silero VAD (via ONNX),
//! ale RMS działa wystarczająco dobrze dla MVP i nie wymaga dodatkowych modeli.

const ENERGY_THRESHOLD: f32 = 0.01;
const MIN_SPEECH_FRAMES: usize = 3;

#[derive(Debug, Clone, Copy, Default)]
pub struct VadResult {
    pub is_speech: bool,
    pub energy: f32,
}

pub struct VadDetector {
    frame_size: usize,
    history: Vec<bool>,
}

impl VadDetector {
    pub fn new(sample_rate: u32) -> Self {
        // ramki ~30ms
        let frame_size = (sample_rate as usize * 30) / 1000;
        Self {
            frame_size,
            history: Vec::new(),
        }
    }

    pub fn process_frame(&mut self, frame: &[f32]) -> VadResult {
        if frame.is_empty() {
            return VadResult::default();
        }
        let sum_sq: f32 = frame.iter().map(|s| s * s).sum();
        let rms = (sum_sq / frame.len() as f32).sqrt();
        let is_speech = rms > ENERGY_THRESHOLD;

        self.history.push(is_speech);
        if self.history.len() > 5 {
            self.history.remove(0);
        }

        // histereza: speech, jeśli min. MIN_SPEECH_FRAMES z ostatnich 5 było aktywne
        let speech_count = self.history.iter().filter(|x| **x).count();
        VadResult {
            is_speech: speech_count >= MIN_SPEECH_FRAMES,
            energy: rms,
        }
    }

    pub fn frame_size(&self) -> usize {
        self.frame_size
    }
}

/// Filtruje próbki - zwraca tylko fragmenty z aktywnym głosem
pub fn trim_silence(samples: &[f32], sample_rate: u32) -> Vec<f32> {
    let mut vad = VadDetector::new(sample_rate);
    let frame = vad.frame_size();
    let mut out = Vec::with_capacity(samples.len());

    for chunk in samples.chunks(frame) {
        let r = vad.process_frame(chunk);
        if r.is_speech {
            out.extend_from_slice(chunk);
        }
    }
    out
}
