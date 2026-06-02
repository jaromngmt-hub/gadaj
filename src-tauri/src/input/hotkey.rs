//! Globalny skrót klawiszowy (push-to-talk) z wykorzystaniem `rdev`.
//!
//! rdev 0.5 nie udostępnia flag modyfikatorów w evencie, więc śledzimy je ręcznie.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use parking_lot::Mutex;

pub type KeyHandler = Arc<dyn Fn(bool) + Send + Sync + 'static>;

pub struct HotkeyListener {
    running: Arc<AtomicBool>,
    active: Arc<AtomicBool>,
    key: Mutex<Option<String>>,
    thread: Mutex<Option<thread::JoinHandle<()>>>,
}

impl HotkeyListener {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            active: Arc::new(AtomicBool::new(false)),
            key: Mutex::new(None),
            thread: Mutex::new(None),
        }
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    pub fn set_key(&self, key: String) {
        *self.key.lock() = Some(key);
    }

    pub fn start(&self, on_change: KeyHandler) -> Result<(), String> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        let key_str = self
            .key
            .lock()
            .clone()
            .ok_or("Nie ustawiono klawisza dyktowania")?;
        let running = self.running.clone();
        let active = self.active.clone();

        let handle = thread::spawn(move || {
            use rdev::{listen, EventType, Key};
            let parsed = parse_hotkey(&key_str);

            // Ręczne śledzenie modyfikatorów (rdev 0.5 nie daje flag w evencie)
            let meta_down = Arc::new(AtomicBool::new(false));
            let ctrl_down = Arc::new(AtomicBool::new(false));
            let alt_down = Arc::new(AtomicBool::new(false));
            let shift_down = Arc::new(AtomicBool::new(false));

            let meta_d = meta_down.clone();
            let ctrl_d = ctrl_down.clone();
            let alt_d = alt_down.clone();
            let shift_d = shift_down.clone();

            let callback = move |event: rdev::Event| {
                if !running.load(Ordering::Relaxed) {
                    return;
                }
                match event.event_type {
                    EventType::KeyPress(k) => {
                        // Aktualizuj stan modyfikatorów
                        match k {
                            Key::MetaLeft | Key::MetaRight => meta_d.store(true, Ordering::SeqCst),
                            Key::ControlLeft | Key::ControlRight => ctrl_d.store(true, Ordering::SeqCst),
                            Key::Alt | Key::AltGr => alt_d.store(true, Ordering::SeqCst),
                            Key::ShiftLeft | Key::ShiftRight => shift_d.store(true, Ordering::SeqCst),
                            _ => {}
                        }

                        if k == parsed.key {
                            // Sprawdź modyfikatory
                            let mods_match = if parsed.mods.is_empty() {
                                !meta_d.load(Ordering::SeqCst)
                                    && !ctrl_d.load(Ordering::SeqCst)
                                    && !alt_d.load(Ordering::SeqCst)
                                    && !shift_d.load(Ordering::SeqCst)
                            } else {
                                parsed.mods.iter().all(|m| match m {
                                    Mod::Cmd => meta_d.load(Ordering::SeqCst),
                                    Mod::Ctrl => ctrl_d.load(Ordering::SeqCst),
                                    Mod::Alt => alt_d.load(Ordering::SeqCst),
                                    Mod::Shift => shift_d.load(Ordering::SeqCst),
                                })
                            };

                            if mods_match && !active.swap(true, Ordering::SeqCst) {
                                on_change(true);
                            }
                        }
                    }
                    EventType::KeyRelease(k) => {
                        match k {
                            Key::MetaLeft | Key::MetaRight => meta_d.store(false, Ordering::SeqCst),
                            Key::ControlLeft | Key::ControlRight => ctrl_d.store(false, Ordering::SeqCst),
                            Key::Alt | Key::AltGr => alt_d.store(false, Ordering::SeqCst),
                            Key::ShiftLeft | Key::ShiftRight => shift_d.store(false, Ordering::SeqCst),
                            _ => {}
                        }

                        if k == parsed.key && active.swap(false, Ordering::SeqCst) {
                            on_change(false);
                        }
                    }
                    _ => {}
                }
            };

            if let Err(e) = listen(callback) {
                log::error!("rdev listen error: {e:?}");
            }
        });

        *self.thread.lock() = Some(handle);
        Ok(())
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(10));
    }
}

impl Default for HotkeyListener {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mod {
    Cmd,
    Ctrl,
    Alt,
    Shift,
}

#[derive(Debug, Clone)]
struct ParsedHotkey {
    key: rdev::Key,
    mods: Vec<Mod>,
}

fn parse_hotkey(s: &str) -> ParsedHotkey {
    use rdev::Key;
    let parts: Vec<String> = s.split('+').map(|p| p.trim().to_string()).collect();
    let mut mods = Vec::new();
    let mut key = Key::Alt; // fallback

    for p in parts.iter() {
        match p.as_str() {
            "Cmd" | "Meta" | "Super" | "Win" => mods.push(Mod::Cmd),
            "Ctrl" | "Control" => mods.push(Mod::Ctrl),
            "Alt" | "Option" => mods.push(Mod::Alt),
            "Shift" => mods.push(Mod::Shift),
            "Cmd/Ctrl" => {
                #[cfg(target_os = "macos")]
                mods.push(Mod::Cmd);
                #[cfg(not(target_os = "macos"))]
                mods.push(Mod::Ctrl);
            }
            "Space" => key = Key::Space,
            "Tab" => key = Key::Tab,
            "Enter" | "Return" => key = Key::Return,
            "Escape" | "Esc" => key = Key::Escape,
            "BackSpace" | "Backspace" => key = Key::Backspace,
            "Delete" => key = Key::Delete,
            "Home" => key = Key::Home,
            "End" => key = Key::End,
            "PageUp" => key = Key::PageUp,
            "PageDown" => key = Key::PageDown,
            "Up" => key = Key::UpArrow,
            "Down" => key = Key::DownArrow,
            "Left" => key = Key::LeftArrow,
            "Right" => key = Key::RightArrow,
            k if k.len() == 1 => {
                let c = k.chars().next().unwrap().to_ascii_lowercase();
                key = match c {
                    'a' => Key::KeyA,
                    'b' => Key::KeyB,
                    'c' => Key::KeyC,
                    'd' => Key::KeyD,
                    'e' => Key::KeyE,
                    'f' => Key::KeyF,
                    'g' => Key::KeyG,
                    'h' => Key::KeyH,
                    'i' => Key::KeyI,
                    'j' => Key::KeyJ,
                    'k' => Key::KeyK,
                    'l' => Key::KeyL,
                    'm' => Key::KeyM,
                    'n' => Key::KeyN,
                    'o' => Key::KeyO,
                    'p' => Key::KeyP,
                    'q' => Key::KeyQ,
                    'r' => Key::KeyR,
                    's' => Key::KeyS,
                    't' => Key::KeyT,
                    'u' => Key::KeyU,
                    'v' => Key::KeyV,
                    'w' => Key::KeyW,
                    'x' => Key::KeyX,
                    'y' => Key::KeyY,
                    'z' => Key::KeyZ,
                    '0' => Key::Num0,
                    '1' => Key::Num1,
                    '2' => Key::Num2,
                    '3' => Key::Num3,
                    '4' => Key::Num4,
                    '5' => Key::Num5,
                    '6' => Key::Num6,
                    '7' => Key::Num7,
                    '8' => Key::Num8,
                    '9' => Key::Num9,
                    '`' => Key::BackQuote,
                    _ => Key::Alt,
                };
            }
            k if k.starts_with('F') => {
                if let Ok(n) = k[1..].parse::<u8>() {
                    key = match n {
                        1 => Key::F1,
                        2 => Key::F2,
                        3 => Key::F3,
                        4 => Key::F4,
                        5 => Key::F5,
                        6 => Key::F6,
                        7 => Key::F7,
                        8 => Key::F8,
                        9 => Key::F9,
                        10 => Key::F10,
                        11 => Key::F11,
                        12 => Key::F12,
                        _ => Key::Alt, // rdev 0.5 nie ma F13+
                    };
                }
            }
            _ => {}
        }
    }

    ParsedHotkey { key, mods }
}

fn parse_hotkey_owned(s: &str) -> (rdev::Key, Vec<Mod>) {
    let p = parse_hotkey(s);
    (p.key, p.mods)
}
