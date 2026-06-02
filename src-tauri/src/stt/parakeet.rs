//! Wrapper na parakeet.cpp przez C API (`parakeet_capi.h`).
//!
//! Dla MVP używamy bezpośrednio FFI bez bindgen - ręczne deklaracje symboli
//! z `include/parakeet_capi.h`. Gdyby parakeet.cpp nie był zbudowany (brak
//! submoduła), ten moduł udostępnia fallback `MockEngine`, który zwraca
//! placeholder tekst.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;

use parking_lot::Mutex;

pub const PARAKEET_V3_URL: &str =
    "https://huggingface.co/mudler/parakeet-cpp-gguf/resolve/main/parakeet-tdt-0.6b-v3-q4_k.gguf";
pub const PARAKEET_V3_SIZE_BYTES: u64 = 180 * 1024 * 1024; // ~180MB
pub const PARAKEET_V3_FILENAME: &str = "parakeet-tdt-0.6b-v3-q4_k.gguf";

/// Trait abstrakcyjny nad silnikiem STT - ułatwia mockowanie i testy.
pub trait SttEngine: Send + Sync {
    fn load(&mut self, model_path: &Path) -> Result<(), String>;
    fn transcribe(&self, pcm: &[f32], sample_rate: u32) -> Result<String, String>;
    fn is_loaded(&self) -> bool;
    fn unload(&mut self);
    fn backend_name(&self) -> &'static str;
}

// ======================================================================
// Real engine: parakeet.cpp via FFI
// ======================================================================

#[repr(C)]
struct ParakeetCtx {
    _private: [u8; 0],
}

extern "C" {
    fn parakeet_capi_load(path: *const c_char) -> *mut ParakeetCtx;
    fn parakeet_capi_last_error(ctx: *mut ParakeetCtx) -> *const c_char;
    fn parakeet_capi_transcribe_pcm(
        ctx: *mut ParakeetCtx,
        samples: *const f32,
        n_samples: usize,
        sample_rate: u32,
        decoder: i32,
    ) -> *mut c_char;
    fn parakeet_capi_free_string(s: *mut c_char);
    fn parakeet_capi_free(ctx: *mut ParakeetCtx);
}

pub struct ParakeetEngine {
    ctx: *mut ParakeetCtx,
    loaded: bool,
}

// Bezpieczeństwo: ctx jest wewnętrznie zarządzany przez parakeet.cpp
unsafe impl Send for ParakeetEngine {}
unsafe impl Sync for ParakeetEngine {}

impl Default for ParakeetEngine {
    fn default() -> Self {
        Self { ctx: std::ptr::null_mut(), loaded: false }
    }
}

impl Drop for ParakeetEngine {
    fn drop(&mut self) {
        self.unload();
    }
}

impl SttEngine for ParakeetEngine {
    fn load(&mut self, model_path: &Path) -> Result<(), String> {
        if self.loaded {
            return Ok(());
        }
        let path_str = model_path.to_str().ok_or("Nieprawidłowa ścieżka modelu")?;
        let cpath = CString::new(path_str).map_err(|_| "Ścieżka zawiera null byte")?;
        let ctx = unsafe { parakeet_capi_load(cpath.as_ptr()) };
        if ctx.is_null() {
            return Err(format!(
                "Nie udało się załadować modelu parakeet z {}",
                path_str
            ));
        }
        self.ctx = ctx;
        self.loaded = true;
        log::info!("Załadowano model parakeet: {}", path_str);
        Ok(())
    }

    fn transcribe(&self, pcm: &[f32], sample_rate: u32) -> Result<String, String> {
        if !self.loaded || self.ctx.is_null() {
            return Err("Model nie jest załadowany".into());
        }
        if pcm.is_empty() {
            return Ok(String::new());
        }
        let result_ptr = unsafe {
            parakeet_capi_transcribe_pcm(
                self.ctx,
                pcm.as_ptr(),
                pcm.len(),
                sample_rate,
                0, // 0 = default decoder
            )
        };
        if result_ptr.is_null() {
            return Err("parakeet_capi_transcribe_pcm zwrócił null".into());
        }
        let cstr = unsafe { CStr::from_ptr(result_ptr) };
        let s = cstr.to_string_lossy().into_owned();
        unsafe { parakeet_capi_free_string(result_ptr) };
        Ok(s.trim().to_string())
    }

    fn is_loaded(&self) -> bool {
        self.loaded
    }

    fn unload(&mut self) {
        if !self.ctx.is_null() {
            unsafe { parakeet_capi_free(self.ctx) };
            self.ctx = std::ptr::null_mut();
        }
        self.loaded = false;
    }

    fn backend_name(&self) -> &'static str {
        "parakeet.cpp"
    }
}

// ======================================================================
// Fallback engine: gdy parakeet.cpp nie jest zbudowany (np. brak submoduła)
// ======================================================================

pub struct MockEngine {
    loaded: bool,
    path: Option<String>,
}

impl Default for MockEngine {
    fn default() -> Self {
        Self { loaded: false, path: None }
    }
}

impl SttEngine for MockEngine {
    fn load(&mut self, model_path: &Path) -> Result<(), String> {
        self.loaded = true;
        self.path = Some(model_path.to_string_lossy().into_owned());
        log::warn!(
            "MockEngine: model '{}' nie zostanie faktycznie załadowany. \
             Zbuduj vendor/parakeet.cpp, aby włączyć prawdziwy STT.",
            model_path.display()
        );
        Ok(())
    }

    fn transcribe(&self, pcm: &[f32], _sample_rate: u32) -> Result<String, String> {
        let secs = pcm.len() as f32 / 16000.0;
        Ok(format!(
            "[mock STT: {:.1}s nagrania, model={:?}, brak parakeet.cpp]",
            secs,
            self.path
        ))
    }

    fn is_loaded(&self) -> bool {
        self.loaded
    }

    fn unload(&mut self) {
        self.loaded = false;
        self.path = None;
    }

    fn backend_name(&self) -> &'static str {
        "mock"
    }
}

/// Wybiera dostępny silnik. Zwraca Mock jeśli parakeet.cpp nie jest zbudowany.
pub fn make_engine() -> Box<dyn SttEngine> {
    #[cfg(parakeet_built)]
    {
        Box::new(ParakeetEngine::default())
    }
    #[cfg(not(parakeet_built))]
    {
        log::warn!(
            "parakeet.cpp nie jest zbudowany - używam MockEngine. \
             Aby włączyć prawdziwy STT, uruchom: git submodule update --init --recursive"
        );
        Box::new(MockEngine::default())
    }
}

/// Globalny, leniwy silnik STT współdzielony między pipeline a komendami.
pub fn shared_engine() -> &'static Mutex<Box<dyn SttEngine>> {
    use once_cell::sync::Lazy;
    static ENGINE: Lazy<Mutex<Box<dyn SttEngine>>> =
        Lazy::new(|| Mutex::new(make_engine()));
    &ENGINE
}
