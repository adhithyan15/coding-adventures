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
if [[ "$BACKEND" == "swiftui" ]]; then
  # SwiftUI is the one backend that LINKS the engine rather than dlopen-ing it,
  # so it needs the static archive and a header, not a dynamic library dropped
  # beside the binary.
  #
  # Engram's `MosaicHost.swift` opens with `import CEngram`, and the emitted
  # Package.swift declares only `CMosaicRuntime` -- the standard runtime shim.
  # Nothing tells the emitter that this package's host asset needs a different
  # module, so the emitted project fails immediately with:
  #
  #     error: no such module 'CEngram'
  #
  # This is the same root cause as UI47 (#13645): Engram routes through
  # `engram-capi` because the standard ABI cannot express its host intents yet.
  # Once #13728 moves the adapters onto the standard runtime, CEngram stops
  # being needed and this block should be deleted rather than generalised --
  # building emitter infrastructure for a configuration we intend to retire
  # would be the wrong investment.
  ( cd "$RUST" && cargo build -q -p engram-capi --release )
  STATIC="$RUST/target/release/libengram_capi.a"
  if [[ ! -f "$STATIC" ]]; then
    echo "error: expected the static archive at $STATIC" >&2
    exit 1
  fi
  mkdir -p "$APP/Sources/CEngram/include" "$APP/Sources/CEngram/lib"
  cp "$RUST/engram-capi/include/engram.h" "$APP/Sources/CEngram/include/engram.h"
  cp "$STATIC" "$APP/Sources/CEngram/lib/libengram_capi.a"
  cat > "$APP/Sources/CEngram/module.modulemap" <<'MODULEMAP'
module CEngram {
  header "include/engram.h"
  export *
}
MODULEMAP

  python3 - "$APP/Package.swift" <<'PYPKG'
import sys

path = sys.argv[1]
source = open(path).read()

if 'name: "CEngram"' in source:
    print("  CEngram already declared")
    sys.exit(0)

# Plain string surgery rather than regex: the emitted Package.swift is
# generated from a fixed template, so the anchors below are exact, and a
# regex here would only add escaping hazards for no extra robustness.
SYSTEM_LIBRARY = (
    "  targets: [\n"
    "    .systemLibrary(\n"
    '      name: "CEngram",\n'
    '      path: "Sources/CEngram"\n'
    "    ),\n"
)
if "  targets: [\n" not in source:
    sys.exit("Package.swift did not contain the expected targets list")
source = source.replace("  targets: [\n", SYSTEM_LIBRARY, 1)

APP_TARGET = (
    "    .executableTarget(\n"
    '      name: "App",\n'
    '      dependencies: ["CMosaicRuntime"],\n'
    '      path: "Sources/App"\n'
    "    ),\n"
)
APP_TARGET_LINKED = (
    "    .executableTarget(\n"
    '      name: "App",\n'
    '      dependencies: ["CMosaicRuntime", "CEngram"],\n'
    '      path: "Sources/App",\n'
    "      linkerSettings: [\n"
    '        .unsafeFlags(["-L", "Sources/CEngram/lib", "-lengram_capi"])\n'
    "      ]\n"
    "    ),\n"
)
if APP_TARGET not in source:
    sys.exit("Package.swift's App target did not match the expected shape")
source = source.replace(APP_TARGET, APP_TARGET_LINKED, 1)

open(path, "w").write(source)
print("  declared the CEngram system library in Package.swift")
PYPKG
  echo "  $APP/Sources/CEngram/{include/engram.h,lib/libengram_capi.a}"
else
  # This is the step whose absence makes an emitted native app inert. The
  # CMakeLists copies the library beside the binary post-build, but only if it
  # finds it here first.
  cp "$LIB_PATH" "$APP/$LIB_NAME"
  echo "  $APP/$LIB_NAME"
fi

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
  swiftui)
    ( cd "$APP" && swift build -c release )
    BIN="$APP/.build/release/App"
    if [[ ! -x "$BIN" ]]; then
      echo "error: swift build produced no executable at $BIN" >&2
      exit 1
    fi
    # The engine is LINKED here rather than loaded at runtime, so the check is
    # that its symbols actually made it into the binary -- the equivalent of
    # Qt's "is the library beside the executable", one layer earlier.
    #
    # DEFINED symbols specifically. The earlier form accepted an UNDEFINED
    # `_eg_` symbol too, which is exactly the binary that does not contain the
    # engine -- it expects to find it elsewhere at load time. Demonstrated with
    # a two-line C program: `nm -u` reports `_eg_snapshot`, and the old
    # condition passed it. A check that accepts the failure it exists to catch
    # is worse than no check, because the build says "verified".
    DEFINED="$(nm "$BIN" 2>/dev/null | grep -c ' T _eg_' || true)"
    UNDEFINED="$(nm -u "$BIN" 2>/dev/null | grep -c '_eg_' || true)"
    if [[ "$DEFINED" -eq 0 ]]; then
      echo "error: no DEFINED eg_* symbols in $BIN; the engine did not link" >&2
      echo "       ($UNDEFINED undefined eg_* symbols -- the engine is expected" >&2
      echo "        from somewhere else at load time, which will not be there)" >&2
      exit 1
    fi
    if [[ "$UNDEFINED" -gt 0 ]]; then
      echo "error: $UNDEFINED eg_* symbols in $BIN are undefined; the engine" >&2
      echo "       linked only partially and the app will fail at load" >&2
      exit 1
    fi
    # Static linking pulls only the objects actually referenced, so this is the
    # handful the SwiftUI host calls -- not the ~47 the cdylib exports. Counting
    # up to the full export list here would fail on a correct build.
    echo "  $DEFINED engine symbols linked into the binary"

    # Wrap it as a `.app`. `swift build` leaves a bare Mach-O executable, which
    # runs from a terminal but is not something a person can be handed: macOS
    # needs the bundle to give it a name, an icon slot, and a Dock identity.
    #
    # The version is only cosmetic here -- it shows in Finder's Get Info -- and
    # there is no version file in the repo to read, so the release lane passes
    # the tag it is publishing and a local build says so plainly.
    APP_NAME="Engram"
    APP_VERSION="${ENGRAM_VERSION:-0.0.0-dev}"
    BUNDLE="$APP/$APP_NAME.app"
    rm -rf "$BUNDLE"
    mkdir -p "$BUNDLE/Contents/MacOS" "$BUNDLE/Contents/Resources"
    cp "$BIN" "$BUNDLE/Contents/MacOS/$APP_NAME"
    printf 'APPL????' > "$BUNDLE/Contents/PkgInfo"
    cat > "$BUNDLE/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key><string>$APP_NAME</string>
  <key>CFBundleIdentifier</key><string>dev.mosaic.engram</string>
  <key>CFBundleName</key><string>$APP_NAME</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$APP_VERSION</string>
  <key>CFBundleVersion</key><string>$APP_VERSION</string>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

    # The bundled copy is the one that ships, and copying is a second chance to
    # lose the engine -- the Compose backend shipped a distribution with no
    # engine in it exactly this way. So the assertion is repeated against the
    # artifact rather than inherited from the binary it came from.
    SHIPPED="$(nm "$BUNDLE/Contents/MacOS/$APP_NAME" 2>/dev/null | grep -c ' T _eg_' || true)"
    if [[ "$SHIPPED" -ne "$DEFINED" ]]; then
      echo "error: the bundled executable has $SHIPPED engine symbols, the" >&2
      echo "       built one had $DEFINED" >&2
      exit 1
    fi
    echo "  $BUNDLE"
    echo ""
    echo "Built: $BUNDLE"
    ;;
  xaml)
    # WinUI 3, targeting net9.0-windows10.0.19041.0. The XAML markup compiler is
    # a Windows-native tool, so this step only runs on Windows -- `dotnet build`
    # elsewhere gets through restore and the C# project system and then stops:
    #
    #     error: XamlCompiler output file "…/output.json" was not created.
    #
    # Refusing here with that explanation is better than letting the build fail
    # a minute later inside a NuGet targets file.
    if [[ "$(uname -s)" != MINGW* && "$(uname -s)" != MSYS* && "$(uname -s)" != CYGWIN* ]]; then
      echo "error: the XAML backend builds on Windows only." >&2
      echo "       WinUI's markup compiler is a Windows-native tool; \`dotnet\`" >&2
      echo "       restores and type-checks elsewhere but cannot compile the XAML." >&2
      echo "       The project is emitted at $APP with the engine in place." >&2
      exit 3
    fi

    ( cd "$APP" && dotnet publish -c Release -r win-x64 --self-contained false -o "$APP/publish" )

    # .NET probes beside the executable, so the engine goes into the publish
    # output rather than the project directory.
    if [[ ! -f "$APP/publish/$LIB_NAME" ]]; then
      cp "$LIB_PATH" "$APP/publish/$LIB_NAME"
    fi
    if [[ ! -f "$APP/publish/$LIB_NAME" ]]; then
      echo "error: engine not placed beside the executable in $APP/publish" >&2
      exit 1
    fi
    echo "  engine placed at $APP/publish/$LIB_NAME"
    echo ""
    echo "Built: $APP/publish"
    ;;
  compose)
    # Compose Desktop, on the JVM, so one lane covers Linux, macOS and Windows
    # -- the cheapest breadth of the five backends.
    #
    # `createDistributable` rather than `packageDistributionForCurrentOS`: the
    # latter builds a .dmg/.msi/.deb, which needs platform packaging tools and
    # produces something that has to be installed before it can be checked. The
    # distributable is the same application tree, inspectable in place.
    if ! command -v gradle >/dev/null 2>&1; then
      echo "error: gradle is required to build the Compose backend." >&2
      echo "       The project is emitted at $APP with the engine in place;" >&2
      echo "       install Gradle (or use mise) and re-run." >&2
      exit 3
    fi

    ( cd "$APP" && gradle --quiet createDistributable )

    DIST="$APP/build/compose/binaries/main/app"
    if [[ ! -d "$DIST" ]]; then
      echo "error: gradle produced no distribution at $DIST" >&2
      exit 1
    fi

    # THE trap this backend has, and the reason compiling is not enough.
    #
    # `MosaicHost.loadCapi` resolves the engine at RUNTIME, trying the working
    # directory and then the directory holding its own jar. Compose Desktop
    # packages the jars and nothing else, so a distribution built without this
    # step launches into an app where every deck operation silently does
    # nothing -- and it compiles perfectly, which is exactly why CI's
    # acceptance lane cannot catch it.
    #
    # The jar directory is found rather than assumed: it is
    # `Contents/app` inside a macOS .app bundle and `lib/app` on Linux and
    # Windows, and hard-coding either would break the other two silently.
    HOST_JAR="$(find "$DIST" -name '*.jar' -exec sh -c '
      unzip -l "$1" 2>/dev/null | grep -q "MosaicHost.class" && echo "$1"
    ' _ {} \; | head -1)"
    if [[ -z "$HOST_JAR" ]]; then
      echo "error: no jar in $DIST contains MosaicHost; cannot place the engine" >&2
      exit 1
    fi
    JAR_DIR="$(dirname "$HOST_JAR")"
    cp "$LIB_PATH" "$JAR_DIR/$LIB_NAME"

    # Asserted, not assumed: the copy above could silently no-op if the
    # distribution were rebuilt afterwards, and the failure mode is an app that
    # starts and does nothing.
    if [[ ! -f "$JAR_DIR/$LIB_NAME" ]]; then
      echo "error: the engine is not beside the host jar at $JAR_DIR" >&2
      echo "       the app would launch with every deck operation unavailable" >&2
      exit 1
    fi

    # Present is not the same as usable. Re-check the SHIPPED copy exports the
    # engine's symbols, rather than trusting that the one verified in step [1]
    # arrived intact -- a truncated or wrong-architecture copy is a file that
    # exists and cannot be loaded, which is the same silent failure one step
    # further along.
    case "$(uname -s)" in
      Darwin) SHIPPED="$(nm -gU "$JAR_DIR/$LIB_NAME" 2>/dev/null | grep -c ' _eg_' || true)" ;;
      Linux)  SHIPPED="$(nm -D --defined-only "$JAR_DIR/$LIB_NAME" 2>/dev/null | grep -c ' eg_' || true)" ;;
      *)      SHIPPED="unknown" ;;
    esac
    if [[ "$SHIPPED" != "unknown" && "$SHIPPED" -lt 20 ]]; then
      echo "error: the engine beside the host jar exports only $SHIPPED eg_* symbols" >&2
      echo "       the app would launch and every deck operation would fail" >&2
      exit 1
    fi
    echo "  engine placed beside the host jar at $JAR_DIR/$LIB_NAME"
    echo "  shipped engine exports $SHIPPED eg_* symbols"
    echo ""
    echo "Built: $DIST"
    ;;
  *)
    echo "error: the $BACKEND compile step is not wired yet." >&2
    echo "       The project is emitted at $APP with the engine in place;" >&2
    echo "       what remains is invoking that backend's toolchain." >&2
    exit 3
    ;;
esac
