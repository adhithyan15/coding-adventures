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
build_engram_capi() {
  local rustflags="${RUSTFLAGS-}"
  case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*) rustflags="${rustflags:+$rustflags }-C target-feature=+crt-static" ;;
  esac
  (
    cd "$RUST"
    RUSTFLAGS="$rustflags" \
      cargo build -q -p engram-capi --release
  )
}

# A Windows cdylib is dropped directly beside the emitted XAML app. Static CRT
# linking is what makes that single file deployable without runner-local
# `vcruntime140.dll`; every other host keeps the caller's normal Rust flags.
build_engram_capi
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

EMIT_ARGS=(--backend "$BACKEND" --output "$OUTPUT" --emit-project)

# SwiftUI reaches the engine through the STANDARD runtime now, not through
# `engram-capi`.
#
# Its `MosaicHost.swift` override is retired: props, events, snapshot and
# restore go through `engram-mosaic-app` and the generated binding, and the
# file dialogs ride `Effect` through `[host_effects]`. So the app needs the
# standard app-ABI cdylib bundled, which is what `--runtime-library` does, and
# `native-complete` is the profile that makes the generated host REQUIRE it
# rather than fall back to a reflection bridge that will not be there.
#
# Without this the emitted app compiles and launches with no engine at all --
# the "runnable is not working" trap the rest of this script exists to catch.
if [[ "$BACKEND" == "swiftui" ]]; then
  ( cd "$RUST" && cargo build -q -p engram-mosaic-app --release )
  case "$(uname -s)" in
    Darwin) MOSAIC_LIB_NAME="libengram_mosaic_app.dylib" ;;
    Linux)  MOSAIC_LIB_NAME="libengram_mosaic_app.so" ;;
    *)      MOSAIC_LIB_NAME="engram_mosaic_app.dll" ;;
  esac
  MOSAIC_LIB="$RUST/target/release/$MOSAIC_LIB_NAME"
  if [[ ! -f "$MOSAIC_LIB" ]]; then
    echo "error: expected the standard runtime at $MOSAIC_LIB" >&2
    exit 1
  fi
  echo "  standard runtime at $MOSAIC_LIB"
  EMIT_ARGS+=(--profile native-complete --runtime-library "$MOSAIC_LIB")
fi

( cd "$RUST" && cargo run -q -p mosaic-compile -- pkg "$HERE" "${EMIT_ARGS[@]}" )

APP="$OUTPUT/$BACKEND"

echo "[3/4] Placing the engine where the emitted project expects it..."
if [[ "$BACKEND" == "swiftui" ]]; then
  # Nothing to place. The standard runtime was bundled by `--runtime-library`
  # at emit time, into `Sources/App/Runtime/`, where SwiftPM carries it as a
  # resource and `Bundle.module` finds it.
  #
  # This is where the `CEngram` system-library block used to be: it built
  # `engram-capi` as a static archive, wrote a module map, and patched the
  # emitted `Package.swift` to link it, because Engram's `MosaicHost.swift`
  # opened with `import CEngram`. Its own comment said to delete it rather than
  # generalise it once the adapters moved to the standard runtime, "building
  # emitter infrastructure for a configuration we intend to retire would be the
  # wrong investment". That move has now happened for SwiftUI, so this is that
  # deletion.
  echo "  the standard runtime was bundled at emit time"
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

    # On macOS, turn the bare executable into a relocatable `.app`.
    #
    # This is the step without which a Qt payload is worthless. `qt_add_executable`
    # links the frameworks by ABSOLUTE path -- `/opt/homebrew/opt/qtbase/lib/
    # QtCore.framework/...` on a Homebrew machine -- so the binary runs perfectly
    # for whoever built it and fails to launch for everyone else. Nothing about
    # the build says so; it is the same "runnable is not working" trap as an app
    # shipped without the engine, one layer down.
    #
    # `macdeployqt` copies the frameworks in and rewrites those paths to
    # `@executable_path`. It needs a bundle to work on, which is why the bundle
    # is built here rather than left to the packaging step.
    if [[ "$(uname -s)" == "Darwin" ]]; then
      APP_NAME="Engram"
      APP_VERSION="${ENGRAM_VERSION:-0.0.0-dev}"
      BUNDLE="$APP/$APP_NAME.app"
      rm -rf "$BUNDLE"
      mkdir -p "$BUNDLE/Contents/MacOS" "$BUNDLE/Contents/Resources"
      cp "$BIN_DIR/EngramApp" "$BUNDLE/Contents/MacOS/$APP_NAME"
      # The host does `QDir(appDir).filePath("libengram_capi.dylib")`, and for a
      # bundled app `appDir` is `Contents/MacOS` -- so the engine goes beside the
      # executable, not into `Frameworks`.
      cp "$BIN_DIR/$LIB_NAME" "$BUNDLE/Contents/MacOS/$LIB_NAME"
      printf 'APPL????' > "$BUNDLE/Contents/PkgInfo"
      cat > "$BUNDLE/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key><string>$APP_NAME</string>
  <key>CFBundleIdentifier</key><string>dev.mosaic.engram.qt</string>
  <key>CFBundleName</key><string>$APP_NAME</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$APP_VERSION</string>
  <key>CFBundleVersion</key><string>$APP_VERSION</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

      MACDEPLOYQT="$(command -v macdeployqt || true)"
      if [[ -z "$MACDEPLOYQT" ]]; then
        QT_PREFIX="$(brew --prefix qt 2>/dev/null || true)"
        [[ -n "$QT_PREFIX" && -x "$QT_PREFIX/bin/macdeployqt" ]] && \
          MACDEPLOYQT="$QT_PREFIX/bin/macdeployqt"
      fi
      if [[ -z "$MACDEPLOYQT" ]]; then
        echo "error: macdeployqt not found; the bundle would link Qt by" >&2
        echo "       absolute path and fail to launch on any other machine." >&2
        exit 1
      fi

      # `-qmldir` matters: the app is QML, and without it macdeployqt bundles
      # the frameworks but not the QML modules, producing a bundle that passes
      # a dependency check and then shows an empty window.
      "$MACDEPLOYQT" "$BUNDLE" -qmldir="$APP" || {
        echo "error: macdeployqt failed" >&2
        exit 1
      }

      # Re-sign, ad hoc. Rewriting install names invalidates every signature
      # macdeployqt touched, and on arm64 the loader REFUSES a dylib whose
      # signature does not verify -- so the app exits immediately, silently,
      # with no output at all.
      #
      # This is worth stating plainly because the dependency check below passes
      # on the broken bundle: every path is relocatable, and the app still does
      # not start. "No absolute paths" and "actually launches" are two
      # different claims, and only the second one is what a user gets.
      codesign --force --deep --sign - "$BUNDLE" 2>/dev/null || {
        echo "error: could not ad-hoc sign $BUNDLE; on arm64 macOS the app" >&2
        echo "       would exit immediately with no output." >&2
        exit 1
      }
      if ! codesign --verify --deep "$BUNDLE" 2>/dev/null; then
        echo "error: $BUNDLE does not pass signature verification after" >&2
        echo "       signing; it would not launch." >&2
        exit 1
      fi
      echo "  signed ad hoc and verified"

      echo "  macdeployqt bundled Qt into $BUNDLE"
      echo ""
      echo "Built: $BUNDLE"
    else
      echo ""
      echo "Built: $BIN_DIR"
    fi
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
    # The engine is BUNDLED here, not linked.
    #
    # This check used to count DEFINED `_eg_` symbols in the binary, because
    # Engram's `MosaicHost.swift` linked `engram-capi` statically through a
    # `CEngram` system library. That override is retired: SwiftUI reaches the
    # engine through the standard runtime, which is `dlopen`ed from the app's
    # resource bundle. So there are now zero `_eg_` symbols in a CORRECT build,
    # and the old check failed the very configuration it was meant to protect.
    #
    # What replaces it asks the same question one layer out: did the runtime
    # actually land in the bundle, and is it the library we just built? The
    # failure it guards against is unchanged -- an app that builds, launches,
    # and has no engine.
    # `--show-bin-path`, not `.build/release`. That name is a SYMLINK to the
    # architecture-specific directory SwiftPM really builds into
    # (`.build/arm64-apple-macosx/release`), and `find` does not traverse
    # symlinks without `-L` -- so searching it reports nothing on a build that
    # is entirely correct. Asking SwiftPM where it put things is the same thing
    # the TaskApp CI lane does.
    # The EXACT path, not a `find` for it at any depth.
    #
    # A depth-agnostic glob cannot tell a correct layout from a broken one, and
    # that is not hypothetical here: the first version of this check used one,
    # and it passed a bundle placed where `Bundle.module` never looks. An
    # assertion that accepts the failure it exists to catch is worse than none,
    # because the build prints "verified" -- which is the argument this script
    # makes elsewhere and did not follow here.
    #
    # SwiftPM's layout under `--show-bin-path` is fixed, so the literal path is
    # available and the indirection bought nothing.
    BIN_ROOT="$( cd "$APP" && swift build -c release --show-bin-path )"
    RUNTIME_IN_BUNDLE="$BIN_ROOT/App_App.bundle/Runtime/libmosaic_app.dylib"
    if [[ ! -f "$RUNTIME_IN_BUNDLE" ]]; then
      echo "error: no App_App.bundle/Runtime/libmosaic_app.dylib under $BIN_ROOT" >&2
      echo "       the app would launch with every deck operation unavailable" >&2
      exit 1
    fi
    # Byte-identical, not merely present: a stale copy from an earlier build
    # would satisfy an existence check and ship the wrong engine.
    if ! cmp -s "$MOSAIC_LIB" "$RUNTIME_IN_BUNDLE"; then
      echo "error: the bundled runtime differs from $MOSAIC_LIB" >&2
      exit 1
    fi
    # And it must be a real engine rather than an empty library, which is the
    # same contract the `engram-capi` export count checks at step [1/4].
    # By NAME, not by count -- and not filtered on `mosaic_app_`.
    #
    # A count over `mosaic_app_*` was wrong in both directions.
    # `mosaic_buffer_free` has no `_app` infix, so that filter could not see a
    # symbol `CMosaicRuntime.c` resolves through its FAILING branch: a runtime
    # without it is rejected by `resolve_symbols`, `is_ready()` returns false,
    # and every deck operation reports UNAVAILABLE. And a floor of six made
    # `mosaic_app_complete_effect` mandatory, which the loader deliberately
    # tolerates missing. The check demanded the optional symbol and ignored a
    # required one.
    #
    # `complete_effect` is listed anyway, because Engram needs it even where the
    # loader does not: every `[host_effects]` handler answers through it, and an
    # unanswered `Await` takes the session's persistence with it.
    MOSAIC_SYMBOLS="$(nm -gU "$RUNTIME_IN_BUNDLE" 2>/dev/null || true)"
    MISSING=""
    for symbol in mosaic_app_create mosaic_app_dispatch mosaic_app_snapshot \
                  mosaic_app_restore mosaic_buffer_free mosaic_app_destroy \
                  mosaic_app_complete_effect; do
      if ! grep -q " _$symbol\$" <<<"$MOSAIC_SYMBOLS"; then
        MISSING="${MISSING:+$MISSING }$symbol"
      fi
    done
    if [[ -n "$MISSING" ]]; then
      echo "error: the bundled runtime does not define: $MISSING" >&2
      echo "       the app would launch with every deck operation unavailable" >&2
      exit 1
    fi
    echo "  runtime bundled at $RUNTIME_IN_BUNDLE (full app ABI present)"

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

    # The resource bundle carries the engine, so it has to come too -- and it
    # goes at the `.app` ROOT, not in `Contents/Resources`.
    #
    # That is counter-intuitive and was wrong here first time round. SwiftPM's
    # generated accessor does not use `Bundle.main.resourceURL`; it builds
    # `Bundle.main.bundleURL.appendingPathComponent("App_App.bundle")` and falls
    # back to an ABSOLUTE build-machine path baked in at compile time. For a
    # packaged app `bundleURL` is `Engram.app` itself, so a bundle under
    # `Contents/Resources` is never consulted.
    #
    # The failure that causes is total and silent on the build machine: the
    # baked-in `.build` path still resolves there, so the app runs locally and
    # fatal-errors ("could not load resource bundle", SIGTRAP) on every other
    # machine. Placing it here is also what removes that absolute path from the
    # startup search, rather than leaving a shipped binary that `dlopen`s from a
    # directory that happened to exist on a CI runner.
    RESOURCE_BUNDLE="$(dirname "$(dirname "$RUNTIME_IN_BUNDLE")")"
    cp -R "$RESOURCE_BUNDLE" "$BUNDLE/"
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
    # Again the exact path, for the same reason: this is the assertion that let
    # the wrong placement through, and a glob here cannot distinguish the layout
    # that runs from the one that fatal-errors on launch.
    SHIPPED_RUNTIME="$BUNDLE/App_App.bundle/Runtime/libmosaic_app.dylib"
    if [[ ! -f "$SHIPPED_RUNTIME" ]]; then
      echo "error: the .app has no App_App.bundle/Runtime/libmosaic_app.dylib" >&2
      echo "       at its root, which is where Bundle.module looks; it would" >&2
      echo "       fatal-error on launch on any machine but this one" >&2
      exit 1
    fi
    if ! cmp -s "$MOSAIC_LIB" "$SHIPPED_RUNTIME"; then
      echo "error: the runtime inside the .app differs from $MOSAIC_LIB" >&2
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

    # `-p:Platform=x64` is required, and `-r` does not supply it: the emitted
    # build.ps1 says so in as many words -- "WindowsAppSDK's self-contained
    # mode rejects AnyCPU; the csproj's <Platforms> only declares the SET, the
    # ACTIVE one comes from this arg". Without it MSBuild may resolve AnyCPU
    # and put the output where the flattening targets do not look.
    ( cd "$APP" && dotnet publish -c Release -r win-x64 -p:Platform=x64 \
        --self-contained false -o "$APP/publish" )

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
