# BUILD.md - budowanie Gadaj od zera

Instrukcja jak zbudować i uruchomić **Gadaj** lokalnie. Po przeczytaniu powinieneś
móc zbudować .app ze źródeł na świeżym komputerze.

## Wymagania (prerequisites)

| Narzędzie | Wersja | Po co |
|-----------|--------|-------|
| macOS | 11+ (Big Sur) | Platforma docelowa (Sequoia 15.x przetestowane) |
| Xcode Command Line Tools | latest | `clang`, `cc`, biblioteki systemowe, `iconutil` |
| Rust (rustup) | 1.80+ | Kompilacja backendu |
| Bun (lub Node 18+) | latest | Frontend, Vite, Tailwind, Tauri CLI |
| cmake | 3.14+ | Budowanie parakeet.cpp + ggml |
| Git | 2.30+ | Submoduły, praca z repo |
| Python 3 + Pillow | — | Generowanie 8-bit ikon (tylko jeśli regenerujesz ikony) |

Sprawdzenie:
```bash
xcode-select -p          # powinno zwrócić /Library/Developer/CommandLineTools
rustc --version          # >= 1.80
bun --version            # >= 1.0
cmake --version          # >= 3.14
git --version
python3 -c "from PIL import Image"  # jeśli planujesz regenerować ikony
```

## Krok 0: Instalacja prereq (jednorazowo)

```bash
# Xcode Command Line Tools
xcode-select --install

# Rust (rustup)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Bun
curl -fsSL https://bun.sh/install | bash

# cmake (przez Homebrew)
brew install cmake

# Python + Pillow (do generowania ikon)
pip3 install Pillow
```

## Krok 1: Klonowanie repo z submodułami

```bash
git clone https://github.com/jaromngmt-hub/gadaj.git
cd gadaj
git submodule update --init --recursive
```

**Ważne:** parakeet.cpp jest git submodulem. Bez `--init --recursive` katalog
`vendor/parakeet.cpp/` będzie pusty i build się wywali.

## Krok 2: Instalacja zależności JS

```bash
bun install
```

## Krok 3: Build parakeet.cpp + ggml

Build dzieje się **automatycznie** przy pierwszym `cargo build` (przez `build.rs`).
Ale jeśli chcesz zbudować ręcznie i obejrzeć output:

```bash
cd vendor/parakeet.cpp
cmake -B build -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=ON
cmake --build build --config Release -j$(sysctl -n hw.ncpu)
ls -lh build/libparakeet.a           # ~520MB static lib
ls build/third_party/ggml/src/*.dylib
```

GGML buduje się jako **shared library** (.dylib), parakeet jako **static** (.a).
Build trwa 3-5 min na Apple Silicon (M-seria).

## Krok 4: Dev mode (hot reload)

```bash
bun tauri dev
```

To odpala:
- Vite dev server na `http://localhost:1420`
- Kompilację Rust (incremental, 30s pierwszy raz, sekundy później)
- Okno Gadaj

Logi lecą na stdout + do `~/Library/Logs/pl.gadaj.app/gadaj/`.

## Krok 5: Build produkcyjny (.app + .dmg)

```bash
bun tauri build
```

To tworzy:
- `src-tauri/target/release/gadaj` — surowa binarka
- `src-tauri/target/release/bundle/macos/Gadaj.app` — spakowana apka
- `src-tauri/target/release/bundle/dmg/Gadaj_0.1.0_aarch64.dmg` — instalator

**Ale sam `tauri build` nie kopiuje ggml dylibów do bundle!**
Trzeba uruchomić skrypt pomocniczy:

```bash
bash scripts/copy_ggml_to_bundle.sh
```

Lub zrobić wszystko jednym poleceniem:

```bash
bun run build:full
```

To wykonuje: `bun run build` + `bun run tauri:build` + `bash scripts/copy_ggml_to_bundle.sh`.

## Krok 6: Pierwsze uruchomienie

Po buildzie masz `.app`. Pierwsze uruchomienie:

```bash
open src-tauri/target/release/bundle/macos/Gadaj.app
```

macOS Sequoia może pokazać Gatekeeper prompt "Gadaj cannot be opened". Rozwiązanie:

```bash
# Opcja 1: podpisanie ad-hoc (dla deweloperki)
codesign --force --deep --sign - src-tauri/target/release/bundle/macos/Gadaj.app
xattr -cr src-tauri/target/release/bundle/macos/Gadaj.app

# Opcja 2: kliknij prawym → Open (raz, potem już normalnie)
```

**Uwaga:** adhoc-signed apka **nie przejdzie notarizacji** Apple, więc `spctl --assess`
zwróci "rejected". Ale `open` ją uruchomi po pierwszym "Open Anyway" w Privacy & Security.
Dla prawdziwej dystrybucji potrzebujesz Apple Developer ID ($99/rok) + notarizacji.

## Krok 7: Pierwszy model .gguf

Aplikacja startuje w **onboardingu** — prowadzi usera przez:
1. Wybór hotkeya (np. `Alt+Space`)
2. Pobranie modelu STT (~675MB) z HuggingFace
3. Gotowe

Pobieranie modelu dzieje się w tle, pasek postępu w UI.

## Weryfikacja że wszystko działa

```bash
# 1. Testy Rust (18 integration + 8 unit + normalize)
cargo test --manifest-path src-tauri/Cargo.toml

# 2. Sprawdź rpath
otool -l src-tauri/target/release/bundle/macos/Gadaj.app/Contents/MacOS/gadaj | grep -A 1 LC_RPATH
# Powinno pokazać: @executable_path/../Frameworks

# 3. Sprawdź ggml dyliby w bundle
ls src-tauri/target/release/bundle/macos/Gadaj.app/Contents/Frameworks/
# Powinno być 15 plików libggml*.dylib

# 4. Sprawdź podpis
codesign --verify --strict src-tauri/target/release/bundle/macos/Gadaj.app
# Powinno wyjść bez błędu

# 5. Uruchom
./src-tauri/target/release/bundle/macos/Gadaj.app/Contents/MacOS/gadaj &
sleep 3
pgrep gadaj    # powinien pokazać PID
kill %1
```

## Struktura katalogów builda

```
target/
├── debug/
│   ├── gadaj                          # binary (dev mode)
│   ├── libgadaj_lib.dylib             # dynamic lib (cdylib)
│   ├── libggml*.dylib                 # ggml dyliby (kopiowane przez build.rs)
│   └── deps/                          # zależności Rust
└── release/
    ├── gadaj                          # binary (release mode)
    ├── libgadaj_lib.dylib
    ├── libggml*.dylib
    ├── bundle/
    │   ├── macos/
    │   │   └── Gadaj.app/
    │   │       ├── Contents/
    │   │       │   ├── Info.plist
    │   │       │   ├── MacOS/gadaj
    │   │       │   ├── Resources/    # ikony itp
    │   │       │   └── Frameworks/   # ggml dyliby (kopiowane przez skrypt)
    │   │       └── _CodeSignature/
    │   └── dmg/
    │       └── Gadaj_0.1.0_aarch64.dmg
    └── ...
```

## Troubleshooting

### "Library not loaded: @rpath/libggml.0.dylib"

- Brakuje ggml dylib w `Contents/Frameworks/`
- Uruchom `bash scripts/copy_ggml_to_bundle.sh`
- Sprawdź `ls Gadaj.app/Contents/Frameworks/`

### "Failed to setup app: invalid icon"

- Ikony są 16-bit, Tauri wymaga 8-bit RGBA
- Konwertuj: `python3 -c "from PIL import Image; [Image.open(f).convert('RGBA').save(f, 'PNG') for f in ['32x.png','128x.png','128x@2x.png','256x.png','512x.png','icon-1024.png']]"`
- Regeneruj `icon.icns`: `rm icon.icns && iconutil -c icns icon.iconset -o icon.icns`
- Wymuś rebuild: `touch src-tauri/tauri.conf.json && cargo build --release`

### "PluginInitialization failed"

- Plugin potrzebuje konfiguracji której nie ma w `tauri.conf.json`
- Albo usuń plugin z `lib.rs`, albo dodaj pustą sekcję `plugins.<name>`

### "Błąd podczas uruchamiania aplikacji Gadaj: attempt to set a logger..."

- Dwa loggery walczą o inicjalizację
- Rozwiązanie: nie inicjuj `env_logger`, zostaw tylko `tauri-plugin-log`

### "cannot find native static library `parakeet`"

- `vendor/parakeet.cpp` nie zbudowany
- Sprawdź czy `libparakeet.a` istnieje: `ls vendor/parakeet.cpp/build/libparakeet.a`
- Jeśli nie: `cd vendor/parakeet.cpp && cmake -B build && cmake --build build -j8`

### Rdev nie wykrywa klawisza

- Pierwsze uruchomienie wymaga uprawnień Accessibility
- System Settings → Privacy & Security → Accessibility → dodaj Gadaj
- Zrestartuj aplikację

### Paste nie działa

- Na macOS wymaga uprawnień Accessibility (jak wyżej)
- Alternatywnie w Settings przełącz `paste_method` na "clipboard" (bez auto-paste)

## Cross-platform

| Platforma | Status | Wymagania |
|-----------|--------|-----------|
| macOS Apple Silicon | ✅ Production | Jak wyżej |
| macOS Intel | ✅ Powinno działać (nie testowane) | Zmień build target: `cargo build --target x86_64-apple-darwin` |
| Windows | 🟡 Setup (code jest, build nie testowany) | Visual Studio Build Tools, cmake, NSIS dla .msi |
| Linux | 🟡 Setup | gcc, cmake, webkit2gtk-4.1, librsvg2 |

Windows build:
```cmd
# Zainstaluj Visual Studio Build Tools (Desktop development with C++)
# Zainstaluj Rust + cmake
git clone ... && cd gadaj
git submodule update --init --recursive
bun install
cargo install tauri-cli --version "^2.0"
bun tauri build
```

## CI (GitHub Actions) - TODO

Wkrótce: `.github/workflows/release.yml` który buduje dla macOS + Windows.

---

**Czas typowego builda:**
- `bun install`: 30s
- Pierwszy `cargo build` (parakeet + ggml): 4-5 min
- Kolejne incremental: 5-30s
- `bun tauri build` (release): 2-3 min

**Czas pobrania modelu:** ~675MB, 30-120s zależnie od łącza.
