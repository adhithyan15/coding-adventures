#!/usr/bin/env bash
#
# Build Engram as a **real native app** from the Mosaic package.
#
# This is the point of Mosaic: one declarative package, native apps on every
# platform. Not Electron — Electron is a web app in a wrapper, and shipping it
# as "native" concedes exactly the thing Mosaic exists to prove.
#
# ## What makes these native
#
# Every native host binds `engram-capi`'s `eg_*` symbols through a real Rust
# cdylib — 47 exported functions — rather than loading wasm. Qt is C++/QML,
# SwiftUI is Swift, Compose is Kotlin, XAML is C#, Flutter is Dart. The UI is
# genuinely the platform's own.
#
# ## The trap this script exists to close
#
# The hosts resolve the engine **at runtime**, from the application directory:
#
#     library_.setFileName(QDir(appDir).filePath("libengram_capi.dylib"));
#
# and the emitted CMakeLists copies that library beside the binary **only if it
# already sits in the project directory**. Emission does not put it there. So an
# emitted project builds cleanly, links nothing, launches, and then does
# nothing at all — every deck operation silently unavailable.
#
# CI's Qt lane compiles the emitted app, which is a real gate on emission but
# says nothing about this: compiling is exactly the step that still succeeds.
#
# So the sequence below is: build the engine for the host platform, PLACE IT in
# the emitted project, build, and then verify it actually landed beside the
# binary.

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPO="$(cd "$HERE/../../../.." && pwd)"
RUST="$REPO/code/packages/rust"

BACKEND="qt"
OUTPUT=""
RUN_BUILD=0

usage() {
  cat <<'USAGE'
build-native.sh — build Engram as a native app from the Mosaic package

  --backend NAME  qt (default) | swiftui | compose | xaml | flutter
  --output DIR    Where to emit (default: <engram-app>/dist-native-<backend>)
  --build         Also compile the emitted project
  -h, --help      Show this message

Backends other than qt emit and place the engine, but their compile step is
not wired yet — each needs its own toolchain invocation. See the tracking
issues.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --backend) BACKEND="$2"; shift 2 ;;
    --output)  OUTPUT="$2"; shift 2 ;;
    --build)   RUN_BUILD=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

case "$BACKEND" in
  qt|swiftui|compose|xaml|flutter) ;;
  *) echo "unsupported backend: $BACKEND" >&2; usage >&2; exit 2 ;;
esac

if [[ -z "$OUTPUT" ]]; then
  OUTPUT="$HERE/dist-native-$BACKEND"
fi

# The cdylib's filename is platform-specific, and the emitted CMakeLists looks
# for these exact names.
case "$(uname -s)" in
  Darwin)  LIB_NAME="libengram_capi.dylib" ;;
  Linux)   LIB_NAME="libengram_capi.so" ;;
  MINGW*|MSYS*|CYGWIN*) LIB_NAME="engram_capi.dll" ;;
  *) echo "unsupported host platform: $(uname -s)" >&2; exit 2 ;;
esac

echo "[1/4] Building the Engram engine as a native library..."
( cd "$RUST" && cargo build -q -p engram-capi --release )
LIB_PATH="$RUST/target/release/$LIB_NAME"
if [[ ! -f "$LIB_PATH" ]]; then
  echo "error: expected $LIB_PATH after building engram-capi" >&2
  ls -la "$RUST/target/release" | grep -i engram >&2 || true
  exit 1
fi

# The host resolves ~40 symbols by name at runtime. A library that exists but
# exports nothing produces the same silent, feature-free app as no library at
# all, so this checks the contract rather than the file.
case "$(uname -s)" in
  Darwin) EXPORTS="$(nm -gU "$LIB_PATH" | grep -c ' _eg_' || true)" ;;
  Linux)  EXPORTS="$(nm -D --defined-only "$LIB_PATH" | grep -c ' eg_' || true)" ;;
  *)      EXPORTS="unknown" ;;
esac
if [[ "$EXPORTS" != "unknown" && "$EXPORTS" -lt 20 ]]; then
  echo "error: $LIB_NAME exports only $EXPORTS eg_* symbols; the host resolves ~40" >&2
  exit 1
fi
echo "  $LIB_NAME exports $EXPORTS eg_* symbols"

echo "[2/4] Emitting the Mosaic app for the $BACKEND backend..."
rm -rf "$OUTPUT"
( cd "$RUST" && cargo run -q -p mosaic-compile -- pkg "$HERE" \
    --backend "$BACKEND" --output "$OUTPUT" --emit-project )

APP="$OUTPUT/$BACKEND"

echo "[3/4] Placing the engine where the emitted project expects it..."
# This is the step whose absence makes an emitted native app inert. The
# CMakeLists copies the library beside the binary post-build, but only if it
# finds it here first.
cp "$LIB_PATH" "$APP/$LIB_NAME"
echo "  $APP/$LIB_NAME"

if [[ "$RUN_BUILD" -eq 0 ]]; then
  echo ""
  echo "Emitted: $APP  (re-run with --build to compile)"
  exit 0
fi

echo "[4/4] Building the native app..."
case "$BACKEND" in
  qt)
    cmake -S "$APP" -B "$APP/build" -DCMAKE_BUILD_TYPE=Release
    cmake --build "$APP/build" --config Release --parallel

    # Verify the engine reached the binary's directory. CMake's copy is
    # conditional, so a rename or a moved output directory silently produces an
    # app that launches and does nothing -- and compiling cannot catch it,
    # because compiling is the part that still works.
    BIN_DIR="$APP/build"
    if [[ ! -f "$BIN_DIR/$LIB_NAME" ]]; then
      # Multi-config generators put the binary in a subdirectory.
      FOUND="$(find "$APP/build" -name "$LIB_NAME" -print -quit || true)"
      if [[ -z "$FOUND" ]]; then
        echo "error: $LIB_NAME is not beside the built binary" >&2
        echo "       the app would launch with every deck operation unavailable" >&2
        exit 1
      fi
      BIN_DIR="$(dirname "$FOUND")"
    fi
    echo "  engine verified beside the binary in $BIN_DIR"
    echo ""
    echo "Built: $BIN_DIR"
    ;;
  flutter)
    # `flutter create` adds the platform runner directories the emitted project
    # does not carry -- macos/, linux/, windows/ -- without touching the Dart
    # sources or pubspec already there.
    ( cd "$APP" && flutter create --platforms=macos,linux,windows . >/dev/null )
    ( cd "$APP" && flutter pub get >/dev/null )

    case "$(uname -s)" in
      Darwin) FLUTTER_TARGET="macos" ;;
      Linux)  FLUTTER_TARGET="linux" ;;
      *)      FLUTTER_TARGET="windows" ;;
    esac
    ( cd "$APP" && flutter build "$FLUTTER_TARGET" --release )

    # Flutter's bundle layout differs per platform -- Frameworks/ on macOS,
    # lib/ on Linux, beside the exe on Windows -- so the engine's destination is
    # three problems rather than one. Locate the built bundle and place it where
    # that platform's loader looks.
    case "$FLUTTER_TARGET" in
      macos)
        BUNDLE="$(find "$APP/build/macos" -maxdepth 6 -name "*.app" -print -quit)"
        [[ -n "$BUNDLE" ]] || { echo "error: no .app in $APP/build/macos" >&2; exit 1; }
        mkdir -p "$BUNDLE/Contents/Frameworks"
        cp "$LIB_PATH" "$BUNDLE/Contents/Frameworks/$LIB_NAME"
        PLACED="$BUNDLE/Contents/Frameworks/$LIB_NAME"
        ;;
      linux)
        BUNDLE="$APP/build/linux/x64/release/bundle"
        [[ -d "$BUNDLE" ]] || { echo "error: no bundle at $BUNDLE" >&2; exit 1; }
        mkdir -p "$BUNDLE/lib"
        cp "$LIB_PATH" "$BUNDLE/lib/$LIB_NAME"
        PLACED="$BUNDLE/lib/$LIB_NAME"
        ;;
      *)
        BUNDLE="$APP/build/windows/x64/runner/Release"
        [[ -d "$BUNDLE" ]] || { echo "error: no bundle at $BUNDLE" >&2; exit 1; }
        cp "$LIB_PATH" "$BUNDLE/$LIB_NAME"
        PLACED="$BUNDLE/$LIB_NAME"
        ;;
    esac
    [[ -f "$PLACED" ]] || { echo "error: engine not placed at $PLACED" >&2; exit 1; }
    echo "  engine placed at $PLACED"
    echo ""
    echo "Built: $BUNDLE"
    ;;
  *)
    echo "error: the $BACKEND compile step is not wired yet." >&2
    echo "       The project is emitted at $APP with the engine in place;" >&2
    echo "       what remains is invoking that backend's toolchain." >&2
    exit 3
    ;;
esac
