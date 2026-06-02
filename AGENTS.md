# AGENTS.md - instrukcje dla AI agents (i nowych developerów)

## Stack
- Tauri 2 (Rust + React 18 + TypeScript)
- parakeet.cpp (vendored jako git submodule) dla STT
- cpal + enigo + rdev + arboard + rusqlite
- React z Zustand, i18next

## Ważne reguły projektu

1. **NIE commituj plików budowanych** — Cargo.lock jest w .gitignore (to jest binarny projekt)
2. **Vendor parakeet.cpp jako submodule** — nie rób `git clone` w vendor/, używaj `git submodule add`
3. **Build.rs buduje parakeet.cpp przez cmake** — nie linkuj zewnętrznych .so/.dll
4. **MockEngine fallback** — gdy parakeet.cpp nie jest zbudowany, kod używa MockEngine (zwraca placeholder tekst)
5. **cfg flag `parakeet_built`** — ustawiane w build.rs gdy vendor/parakeet.cpp istnieje

## Komendy developerskie

```bash
bun install              # zainstaluj zależności JS
bun run build            # build frontendu (Vite)
bun tauri dev            # dev mode z hot reload
bun tauri build          # produkcyjny build
cargo check --manifest-path src-tauri/Cargo.toml    # tylko sprawdź Rust
```

## Struktura modułów

### Backend (Rust)
- `pipeline.rs` - orkiestrator stanów, koordynacja audio→STT→paste
- `audio/capture.rs` - cpal stream w dedykowanym wątku (Send+Sync workaround)
- `audio/vad.rs` - RMS-based VAD
- `stt/parakeet.rs` - FFI do parakeet_capi + MockEngine fallback
- `stt/resample.rs` - linear + sinc resampling do 16kHz
- `input/hotkey.rs` - rdev global hotkey z ręcznym śledzeniem modyfikatorów
- `input/paste.rs` - clipboard + enigo Cmd/Ctrl+V
- `history.rs` - SQLite + FTS5
- `models.rs` - download/manage modeli
- `settings.rs` - persystencja JSON
- `commands.rs` - Tauri commands dla frontendu
- `state.rs` - globalny AppState

### Frontend (React)
- `App.tsx` - główny router (onboarding/main/settings/history)
- `store.ts` - Zustand store + tauri invoke wrapper
- `components/Main.tsx` - ekran główny z dużym przyciskiem dyktowania
- `components/Onboarding.tsx` - setup wizard
- `components/Settings.tsx` - ustawienia
- `components/History.tsx` - lista transkrypcji z search

## Kiedy coś nie działa

1. **parakeet.cpp nie buduje się** — sprawdź `vendor/parakeet.cpp/third_party/ggml` istnieje (git submodule)
2. **Tauri command "no method found"** — brakuje `use tauri::{Manager, Emitter}` w pliku
3. **Send/Sync error na cpal::Stream** — stream musi żyć w wątku w którym został stworzony, użyj LocalStream wrapper
4. **rdev key detection nie działa** — rdev 0.5 nie ma flag modyfikatorów w evencie, trzeba je śledzić ręcznie
5. **enigo API errors** — enigo 0.3 ma `enigo.key(Key, Direction)` zamiast `key_down/key_up`
6. **FTS5 not available** — sprawdź czy rusqlite ma feature `bundled` (tak jest w Cargo.toml)

## Build artifacts

- `src-tauri/target/` — build Rust (NIE w git)
- `dist/` — build Vite (NIE w git)
- `vendor/parakeet.cpp/build/` — cmake build parakeet (NIE w git)
- `node_modules/` — bun/npm deps (NIE w git)
