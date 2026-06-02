//! Smoke testy integracyjne Gadaj.
//!
//! Sprawdza że poszczególne moduły działają bez uruchamiania całego Tauri runtime.

use gadaj_lib::audio::vad::{trim_silence, VadDetector};
use gadaj_lib::history::HistoryStore;
use gadaj_lib::settings::{Settings, SettingsStore};
use gadaj_lib::stt::parakeet::{MockEngine, SttEngine, PARAKEET_V3_URL};
use gadaj_lib::stt::resample::resample_linear;

use std::path::PathBuf;
use std::time::Duration;

// ======================================================================
// VAD
// ======================================================================

#[test]
fn vad_silence_is_not_speech() {
    let mut vad = VadDetector::new(16000);
    let frame = vec![0.0_f32; 480]; // 30ms ciszy
    for _ in 0..10 {
        let r = vad.process_frame(&frame);
        assert!(!r.is_speech, "cisza nie powinna być speech");
        assert!(r.energy < 0.001, "energy ciszy powinno być ~0");
    }
}

#[test]
fn vad_sine_wave_is_speech() {
    let mut vad = VadDetector::new(16000);
    let frame_size = vad.frame_size();
    // sinusoida 440Hz o amplitudzie 0.5
    let mut frame = Vec::with_capacity(frame_size);
    for i in 0..frame_size {
        let t = i as f32 / 16000.0;
        frame.push(0.5 * (2.0 * std::f32::consts::PI * 440.0 * t).sin());
    }
    // VAD ma histerezę: wymaga 3+ z ostatnich 5 ramek
    let mut last_result = None;
    for _ in 0..5 {
        last_result = Some(vad.process_frame(&frame));
    }
    let r = last_result.unwrap();
    assert!(r.is_speech, "głośny sinus powinien być speech");
    assert!(r.energy > 0.3, "energy głośnego sinusa: {}", r.energy);
}

#[test]
fn vad_trim_silence_removes_silent_blocks() {
    let sample_rate = 16000;
    let silence = vec![0.0_f32; sample_rate as usize]; // 1s ciszy
    let speech: Vec<f32> = (0..sample_rate as usize)
        .map(|i| 0.3 * (2.0 * std::f32::consts::PI * 200.0 * i as f32 / 16000.0).sin())
        .collect();
    let mut samples = Vec::new();
    samples.extend(&silence);
    samples.extend(&speech);
    samples.extend(&silence);

    let trimmed = trim_silence(&samples, sample_rate);
    assert!(trimmed.len() < samples.len(), "trim powinien przyciąć");
    assert!(trimmed.len() > speech.len() / 2, "trim nie powinien wyciąć mowy");
}

// ======================================================================
// Resample
// ======================================================================

#[test]
fn resample_identity_preserves_data() {
    let input: Vec<f32> = (0..1000).map(|i| i as f32 / 1000.0).collect();
    let out = resample_linear(&input, 16000, 16000);
    assert_eq!(out, input);
}

#[test]
fn resample_48k_to_16k_reduces_length() {
    let input: Vec<f32> = (0..48_000).map(|i| (i as f32 / 1000.0).sin()).collect();
    let out = resample_linear(&input, 48000, 16000);
    // 48000 * 16000/48000 = 16000 - 1 (sufit)
    assert!(
        out.len() >= 15900 && out.len() <= 16100,
        "oczekiwana długość ~16000, mam {}",
        out.len()
    );
}

#[test]
fn resample_dc_signal_preserves_value() {
    // DC 0.5 powinno zostać 0.5 po resamplingu
    let input = vec![0.5_f32; 4800];
    let out = resample_linear(&input, 48000, 16000);
    for (i, &s) in out.iter().enumerate() {
        assert!((s - 0.5).abs() < 1e-4, "DC drift @ {i}: {s}");
    }
}

// ======================================================================
// History + FTS5 (z polskimi znakami)
// ======================================================================

fn tempdir() -> PathBuf {
    let base = std::env::temp_dir();
    let unique = format!(
        "gadaj-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let p = base.join(unique);
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn history_fts5_creates_and_searches() {
    let dir = tempdir();
    let store = HistoryStore::open(&dir).unwrap();

    let id1 = store.insert("Cześć, jak się masz?", None, Some("pl"), 1234).unwrap();
    let id2 = store.insert("Dzisiaj jest ładna pogoda", None, Some("pl"), 2000).unwrap();
    let id3 = store.insert("Hello world, English only", None, Some("en"), 1500).unwrap();
    assert!(id1 > 0 && id2 > 0 && id3 > 0);

    // Wszystkie wpisy
    let all = store.list(None, 100).unwrap();
    assert_eq!(all.len(), 3, "powinny być 3 wpisy");

    // Szukaj po polskim - "pogoda"
    let pl_results = store.list(Some("pogoda"), 100).unwrap();
    assert_eq!(pl_results.len(), 1, "powinien znaleźć 1 wpis z 'pogoda'");
    assert!(pl_results[0].text.contains("pogoda"));

    // Szukaj po angielskim
    let en_results = store.list(Some("Hello"), 100).unwrap();
    assert_eq!(en_results.len(), 1);

    // Usuń i sprawdź że zniknęło
    store.delete(id2).unwrap();
    let after = store.list(Some("pogoda"), 100).unwrap();
    assert!(after.is_empty(), "po usunięciu FTS nie powinno nic znaleźć");
}

#[test]
fn history_fts5_handles_polish_diacritics() {
    let dir = tempdir();
    let store = HistoryStore::open(&dir).unwrap();
    store.insert("Łódź jest pięknym miastem", None, Some("pl"), 1000).unwrap();
    store.insert("Kraków ma Smoka Wawelskiego", None, Some("pl"), 1500).unwrap();
    store.insert("Gdańsk leży nad morzem", None, Some("pl"), 2000).unwrap();

    // Szukaj bez diakrytyków - FTS5 unicode61 remove_diacritics powinien działać
    let lodz_results = store.list(Some("lodz"), 100).unwrap();
    assert!(!lodz_results.is_empty(), "FTS5 powinien matchować 'lodz' do 'Łódź'");

    let smoka_results = store.list(Some("smoka"), 100).unwrap();
    assert!(!smoka_results.is_empty(), "powinien znaleźć 'smoka' (stem of 'Smoka')");
    // Prefix search też działa
    let prefix_results = store.list(Some("smok*"), 100).unwrap();
    assert!(!prefix_results.is_empty(), "prefix 'smok*' powinien matchować");
}

#[test]
fn history_handles_quotes_in_query() {
    let dir = tempdir();
    let store = HistoryStore::open(&dir).unwrap();
    store.insert("Test \"quoted\" text", None, Some("pl"), 1000).unwrap();
    // Cudzysłowy powinny być sanityzowane
    let r = store.list(Some("\"quoted\""), 100).unwrap();
    // Może być pusty wynik (po sanityzacji) lub match - ważne że nie crashuje
    let _ = r;
}

// ======================================================================
// Settings
// ======================================================================

#[test]
fn settings_default_values() {
    let s = Settings::default();
    assert!(s.hotkey.is_none());
    assert_eq!(s.model_id, "parakeet-tdt-0.6b-v3");
    assert_eq!(s.language, "auto");
    assert_eq!(s.paste_method, "auto");
    assert_eq!(s.ui_language, "pl");
}

#[test]
fn settings_persistence_roundtrip() {
    let dir = tempdir();
    let store = SettingsStore::load_or_default(&dir).unwrap();
    let mut s = store.get();
    s.hotkey = Some("Alt+Space".to_string());
    s.ui_language = "en".to_string();
    s.paste_method = "clipboard".to_string();
    store.set(s.clone()).unwrap();

    // Nowy store z tego samego katalogu
    let store2 = SettingsStore::load_or_default(&dir).unwrap();
    let loaded = store2.get();
    assert_eq!(loaded.hotkey.as_deref(), Some("Alt+Space"));
    assert_eq!(loaded.ui_language, "en");
    assert_eq!(loaded.paste_method, "clipboard");
}

// ======================================================================
// STT Engines
// ======================================================================

#[test]
fn mock_engine_loads_and_transcribes() {
    let mut eng = MockEngine::default();
    assert!(!eng.is_loaded());
    let p = PathBuf::from("/tmp/fake_model.gguf");
    eng.load(&p).unwrap();
    assert!(eng.is_loaded());
    let text = eng.transcribe(&[0.0; 16000], 16000).unwrap();
    assert!(text.contains("mock"), "MockEngine output: {text}");
    assert!(text.contains("1.0"), "powinien pokazać 1.0s: {text}");
    eng.unload();
    assert!(!eng.is_loaded());
}

#[test]
fn mock_engine_empty_pcm_returns_empty() {
    let mut eng = MockEngine::default();
    eng.load(&PathBuf::from("/tmp/fake")).unwrap();
    // Pusty PCM powinien zwrócić placeholder ale bez crash
    let text = eng.transcribe(&[], 16000).unwrap();
    assert!(text.contains("0.0"));
}

// ======================================================================
// Model URL
// ======================================================================

#[test]
fn model_url_looks_valid() {
    assert!(PARAKEET_V3_URL.starts_with("https://"));
    assert!(PARAKEET_V3_URL.contains("huggingface.co"));
    assert!(PARAKEET_V3_URL.ends_with(".gguf"));
}

#[tokio::test]
async fn model_url_reachable() {
    // Sprawdź że HuggingFace URL odpowiada (HEAD, timeout 15s)
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .expect("reqwest builder");
    let resp = client
        .head(PARAKEET_V3_URL)
        .send()
        .await
        .expect("network error - HEAD na HuggingFace");
    assert!(
        resp.status().is_success() || resp.status().is_redirection(),
        "HuggingFace URL {} zwrócił {}",
        PARAKEET_V3_URL,
        resp.status()
    );
}

// ======================================================================
// Runtime: czy ggml dyliby są dostępne obok libgadaj_lib.dylib
// ======================================================================

#[test]
fn ggml_dylibs_present_in_target_debug() {
    // Ten test wykrywa najczęstszy problem runtime: brak ggml dylib w katalogu binarki.
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    // src-tauri/target/debug (cargo buduje tu bezpośrednio)
    let target_debug = manifest_dir.join("target/debug");
    if !target_debug.exists() {
        eprintln!("target/debug nie istnieje, pomijam runtime check");
        return;
    }
    let required = [
        "libggml.dylib",
        "libggml-base.dylib",
        "libggml-cpu.dylib",
        "libggml-blas.dylib",
    ];
    for name in required {
        let p = target_debug.join(name);
        if cfg!(target_os = "macos") {
            assert!(
                p.exists(),
                "Brak wymaganego dyliba: {} (kompilacja poszła, ale runtime się wywali)",
                p.display()
            );
        }
    }
}

#[test]
fn gadaj_lib_dylib_loaded() {
    // Sprawdź że nasza biblioteka się zbudowała
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let target_debug = manifest_dir.join("target/debug");
    let lib = target_debug.join("libgadaj_lib.dylib");
    if cfg!(target_os = "macos") {
        assert!(lib.exists(), "Brak libgadaj_lib.dylib w {}", target_debug.display());
        let bytes = std::fs::read(&lib).expect("nie mogę odczytać dyliba");
        let magic = &bytes[0..4];
        // MH_MAGIC_64 little endian (ARM64/x86_64): cf fa ed fe
        // MH_MAGIC big endian (32-bit):            fe ed fa ce
        assert!(
            magic == [0xcf, 0xfa, 0xed, 0xfe] || magic == [0xfe, 0xed, 0xfa, 0xce],
            "libgadaj_lib.dylib nie jest poprawnym Mach-O (magic: {magic:x?})"
        );
    }
}

#[test]
fn gadaj_exe_links_ggml_with_rpath() {
    // Sprawdź że binarka ma rpath ustawiony tak, żeby runtime znalazł ggml dyliby
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let target_debug = manifest_dir.join("target/debug");
    let exe = target_debug.join("gadaj");
    if !exe.exists() {
        eprintln!("gadaj exe nie istnieje, pomijam");
        return;
    }
    if cfg!(target_os = "macos") {
        let output = std::process::Command::new("otool")
            .arg("-l")
            .arg(&exe)
            .output()
            .expect("otool nie zadziałał");
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Szukamy LC_RPATH z @executable_path
        assert!(
            stdout.contains("LC_RPATH") && stdout.contains("@executable_path"),
            "Brak rpath w binarnce - runtime nie znajdzie ggml dylibów.\notool -l {}:\n{}",
            exe.display(),
            stdout
        );
    }
}
