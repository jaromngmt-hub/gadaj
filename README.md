# Gadaj

> Polska, open-source'owa apka do dyktowania offline. Naciskasz klawisz → mówisz → puszczasz → tekst wkleja się tam, gdzie masz kursor.

**Status: MVP w budowie** — kompiluje się, działa z mock STT do developmentu. Prawdziwy STT (Parakeet V3) włącza się automatycznie po `git submodule update --init --recursive` + buildzie parakeet.cpp.

## Funkcje

- **Polski first-class** — UI po polsku, model obsługuje polski + 25 innych języków
- **Offline** — żadne audio nie opuszcza komputera
- **Push-to-talk** — przytrzymaj klawisz, mów, puść
- **Auto-paste** — tekst wkleja się przez `Cmd+V` / `Ctrl+V` tam, gdzie masz focus
- **Historia z FTS5** — pełnotekstowe przeszukiwanie wszystkich transkrypcji (SQLite)
- **Cross-platform** — macOS + Windows (Linux w roadmapie)

## Stack technologiczny

- **Shell:** [Tauri 2](https://tauri.app) (Rust + WebView)
- **Frontend:** React 18 + TypeScript + Vite + Tailwind CSS
- **i18n:** react-i18next (PL domyślnie, EN dostępny)
- **STT:** [parakeet.cpp](https://github.com/mudler/parakeet.cpp) (C++17/ggml) — port inferencji NVIDIA Parakeet
- **Audio:** cpal + Silero-style RMS VAD
- **DB:** rusqlite (bundled) z FTS5
- **Globalne skróty:** rdev
- **Pasting:** enigo + arboard

## Wymagania developerskie

- **Rust** ≥ 1.77
- **Bun** (lub Node.js ≥ 18)
- **CMake** ≥ 3.20
- **C++ compiler** (Xcode CLT na macOS, MSVC na Windows)
- Na **macOS**: wystarczy `xcode-select --install`
- Na **Windows**: Visual Studio 2022 z C++ toolchain + Vulkan SDK (opcjonalnie)

## Quick start

```bash
# 1. Sklonuj repozytorium (z submodułami)
git clone --recursive https://github.com/jaromngmt-hub/gadaj.git
cd gadaj

# 2. Zainstaluj zależności JS
bun install

# 3. Tryb developerski (parakeet.cpp buduje się przy pierwszym uruchomieniu)
bun tauri dev
```

> **Pierwszy build trwa 5-15 minut** — kompilowane są zależności Tauri 2 + cały parakeet.cpp + ggml. Kolejne buildy są szybkie (incremental).

## Build produkcyjny

```bash
bun tauri build
```

Generuje natywne instalatory:
- macOS: `.app` w `target/release/bundle/macos/` i `.dmg` w `target/release/bundle/dmg/`
- Windows: `.msi` w `target/release/bundle/msi/`

## Pierwsze uruchomienie

Po instalacji:

1. Przejdź onboardingiem:
   - Wybierz klawisz dyktowania (np. `Right Option` na Macu)
   - Poczekaj na pobranie modelu Parakeet V3 (~180MB)
2. Przytrzymaj klawisz dyktowania, mów, puść
3. Tekst pojawi się w aktywnej aplikacji

## Architektura

```
gadaj/
├── src/                  # React/TypeScript frontend
│   ├── components/       # Onboarding, Settings, History, Main
│   ├── i18n/             # tłumaczenia PL/EN
│   └── store.ts          # Zustand store
├── src-tauri/            # Rust core
│   ├── src/
│   │   ├── pipeline.rs   # orkiestrator stanów
│   │   ├── audio/        # capture (cpal) + VAD
│   │   ├── stt/          # parakeet FFI + resampling
│   │   ├── input/        # hotkey (rdev) + paste (enigo)
│   │   ├── history.rs    # SQLite + FTS5
│   │   ├── models.rs     # menedżer modeli
│   │   ├── settings.rs   # persystencja ustawień
│   │   └── commands.rs   # Tauri commands dla frontendu
│   └── build.rs          # buduje parakeet.cpp przez cmake
├── vendor/parakeet.cpp/  # git submodule
└── ...
```

**Pipeline:**
```
[Idle] → naciśnij klawisz → [Recording] → puść klawisz →
  VAD trim → resample 16kHz → parakeet.cpp → schowek → Cmd/Ctrl+V → [Idle]
```

## Licencja

MIT — zobacz [LICENSE](LICENSE).

Ten projekt używa modeli NVIDIA Parakeet, które podlegają oryginalnym licencjom NVIDIA. Zobacz [LICENSE](LICENSE) po szczegóły.

## Inspiracja

Projekt mocno inspirowany [Handy](https://github.com/cjpais/Handy) (open-source Whisper Flow) — ale napisany od zera, w innym stylu, z polskim jako domyślnym językiem i Parakeet zamiast Whisper jako domyślnym silnikiem.

## Roadmap

- [ ] v1.0: Push-to-talk, Parakeet V3, historia z FTS5
- [ ] v1.1: Tryb toggle, post-process LLM (Ollama/OpenAI)
- [ ] v1.2: Streaming EOU (sub-sekundowy response)
- [ ] v2.0: Linux support, voice commands
