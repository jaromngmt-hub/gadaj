//! Wklejanie tekstu do aktywnego okna: schowek + symulacja Cmd/Ctrl+V.

use arboard::Clipboard;
use enigo::{
    Direction::{Press, Release},
    Enigo, Key, Keyboard, Settings as EnigoSettings,
};

pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    let mut cb = Clipboard::new().map_err(|e| format!("Clipboard: {e}"))?;
    cb.set_text(text.to_string()).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_clipboard() -> Result<String, String> {
    let mut cb = Clipboard::new().map_err(|e| format!("Clipboard: {e}"))?;
    cb.get_text().map_err(|e| e.to_string())
}

/// Kopiuje tekst do schowka i symuluje Cmd/Ctrl+V w aktywnym oknie.
pub fn paste_text(text: &str) -> Result<(), String> {
    copy_to_clipboard(text)?;
    std::thread::sleep(std::time::Duration::from_millis(60));

    let mut enigo = Enigo::new(&EnigoSettings::default())
        .map_err(|e| format!("Enigo init: {e:?}"))?;

    #[cfg(target_os = "macos")]
    let modifier = Key::Meta;
    #[cfg(not(target_os = "macos"))]
    let modifier = Key::Control;

    enigo.key(modifier, Press).map_err(|e| format!("key press: {e:?}"))?;
    enigo.key(Key::Unicode('v'), Press).map_err(|e| format!("key v: {e:?}"))?;
    enigo.key(Key::Unicode('v'), Release).map_err(|e| format!("key v release: {e:?}"))?;
    enigo.key(modifier, Release).map_err(|e| format!("key release: {e:?}"))?;

    Ok(())
}
