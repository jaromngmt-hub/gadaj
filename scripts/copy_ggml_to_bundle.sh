#!/bin/bash
# Kopiuje ggml dyliby do Contents/Frameworks/ w bundled .app
# Tauri domyślnie nie pakuje niestandardowych dylibów, więc robimy to ręcznie.
set -euo pipefail

BUNDLE="${1:-src-tauri/target/release/bundle/macos/Gadaj.app}"
FRAMEWORKS_DIR="$BUNDLE/Contents/Frameworks"

if [ ! -d "$BUNDLE" ]; then
    echo "Bundle nie istnieje: $BUNDLE"
    echo "Najpierw uruchom: bun tauri build"
    exit 1
fi

mkdir -p "$FRAMEWORKS_DIR"

GGML_SRC="vendor/parakeet.cpp/build/third_party/ggml"

# Główne dyliby z src/
for dylib in "$GGML_SRC/src"/libggml*.dylib; do
    cp -f "$dylib" "$FRAMEWORKS_DIR/"
done

# Dyliby z subfolderów (ggml-blas/, ggml-metal/)
for sub in ggml-blas ggml-cpu ggml-metal; do
    if [ -d "$GGML_SRC/src/$sub" ]; then
        for dylib in "$GGML_SRC/src/$sub"/lib*.dylib; do
            [ -f "$dylib" ] && cp -f "$dylib" "$FRAMEWORKS_DIR/"
        done
    fi
done

# Dyliby z innych katalogów (ggml/src/ dla base)
echo "Skopiowane do $FRAMEWORKS_DIR:"
ls -1 "$FRAMEWORKS_DIR" | sed 's/^/  /'

# Weryfikacja - binarka powinna widzieć te dyliby
BIN="$BUNDLE/Contents/MacOS/gadaj"
if [ -f "$BIN" ]; then
    echo ""
    echo "Weryfikacja linków @rpath:"
    otool -L "$BIN" | grep rpath | sed 's/^/  /'
fi

# Wyczyść extended attributes (quarantine) - inaczej Gatekeeper będzie blokował
xattr -cr "$BUNDLE" 2>/dev/null || true

echo ""
echo "Gotowe. Aby uruchomić:"
echo "  open '$BUNDLE'"
