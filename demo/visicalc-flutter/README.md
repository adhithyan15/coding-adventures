# VisiCalc — Flutter demo (live, on the Rust engine)

The Flutter VisiCalc demo, now **computing on the shared Rust `spreadsheet-core`
engine** through its C ABI (`spreadsheet-capi`), reached via `dart:ffi` — the
same engine the SwiftUI and Qt demos link natively and the web demos run as
WebAssembly. This is the third native backend wired to the engine.

## What it shows

A `MaterialApp` shell containing the auto-generated `FormulaBar` and `Grid`
widgets (`lib/generated/`, produced by `mosaic-compile --backend flutter` from
the shared `demo/visicalc/mosaic/*` sources).

- The grid renders **engine-computed** values: the classic cross-footing budget
  where column E totals each row, row 5 totals each column, and E5 is the grand
  total (169) — all formulas evaluated by the Rust engine, not hard-coded.
- Editing the formula bar writes through to the engine via
  `SpreadsheetModel.setCell`, which recomputes every dependent cell.

## How it's wired to the engine

```
Flutter widgets (generated)  ──  SpreadsheetModel / SpreadsheetSession (lib/engine.dart)
   Grid(viewportRows: model…)     │  sc_set_cell / sc_get_value … (dart:ffi, String↔char*)
                                  ▼
   native/libspreadsheet_capi.dylib  ←  spreadsheet-capi (Rust C ABI)  ←  spreadsheet-core
```

`lib/engine.dart` loads the engine dynamic library with `DynamicLibrary.open`
and marshals strings by hand (UTF-8 via `dart:convert`, libc `malloc`/`free`
bound through `DynamicLibrary.process()`) — **no extra pub dependency**, not even
`package:ffi`. It maps the engine's JSON value shape (the same contract the
TS/WASM/Swift/Qt engines emit) into display text.

## Build, test, run

```bash
bash scripts/build.sh   # regenerate widgets + build & vendor the engine (cdylib)
flutter test            # HEADLESS: proves the grid is engine-computed + recomputes
flutter pub get && flutter run -d macos   # launch the desktop app
```

`test/engine_test.dart` loads the vendored engine through `dart:ffi` and asserts
the grid is engine-computed (E1 = 38, A5 = 39, E5 = 169), that editing A1
15 → 115 recomputes the totals (E5 → 269), and that a formula entry computes with
binary-op error propagation (`=1/0` → `#DIV/0!`, and `=A1+1` over it → `#DIV/0!`).

> Known gap (tracked separately): the generated Flutter `FormulaBar` places its
> `TextField` directly in a `Row` without a `Flexible` wrapper, so it throws an
> unbounded-width layout assertion when the full app is pumped. That's a
> `mosaic-emit-flutter` emitter bug, independent of the engine wiring (the grid
> renders fine). The headless `engine_test.dart` is the canonical proof here,
> matching how the SwiftUI and Qt demos verify.

## How to run the app

```bash
flutter pub get
flutter run            # picks default target
flutter run -d chrome  # web target
flutter run -d macos   # desktop target
flutter run -d ios     # iOS sim
flutter run -d android # Android emulator
```

(Requires Flutter SDK 3.0+. Install via https://flutter.dev/docs/get-started/install.)

## How to build deployable artefacts (CI-friendly)

```bash
flutter build apk --debug            # Android — debug-signed APK
flutter build apk --release          # Android — release APK (needs signing config)
flutter build ios --no-codesign      # iOS — .app bundle (sideload-ready)
flutter build web --release          # Web — static dist/
flutter build macos --release        # Desktop — .app
```

Verified locally on macOS arm64:

| Target | Command | Output | Result |
| --- | --- | --- | --- |
| Android | `flutter build apk --debug` | `build/app/outputs/flutter-apk/app-debug.apk` | ✅ |
| iOS | `flutter build ios --no-codesign` | `build/ios/iphoneos/Runner.app` (15.2 MB) | ✅ |
| Web | `flutter build web --release` | `build/web/` | ✅ |

The `android/` and `ios/` platform-runner scaffolds are generated
by `flutter create --platforms=ios,android .` and checked in.
Volatile per-platform caches (`android/.gradle/`, `ios/Pods/`,
`xcuserdata/`, etc.) are gitignored — `flutter build` regenerates
them on first run.

## Infinite virtualized sheet

`SpreadsheetSession` (`lib/engine.dart`) also binds the engine's **viewport
primitive** over dart:ffi — `window(r0,c0,r1,c1)` (a dense `List<List<String>>`
rectangle), `usedRange()`, `columnLetters()`, `currentRevision()`, and
`changedSince()` — so a windowed Flutter grid can render only the visible
rectangle of an unbounded sheet (the Flutter sibling of the web/SwiftUI/Qt
infinite views).

Headless proof: `test/window_test.dart` seeds far-flung sparse cells and asserts
the window is engine-computed + dense (A1=15, E1=38, E5=169), a formula 1000
rows down (`Z1000` = 39) is reachable, the gaps are empty (sparse), column
letters run AA/BA, and editing `A1` dirties the far dependent `Z1000` via
`changedSince`. Run with `flutter test test/window_test.dart`.

## Where this fits in the cross-backend demo plan

| Backend | Engine | Status |
|---|---|---|
| HTML (web) | WASM | ✅ live |
| WebComponent (web) | WASM | ✅ live |
| SwiftUI (macOS / iOS) | C ABI | ✅ live |
| Qt / C++ | C ABI | ✅ live |
| Flutter (this one) | C ABI (dart:ffi) | ✅ live (grid; formula-bar emitter gap tracked) |
| Compose / Android (Kotlin) | C ABI (FFM / JNI) | in progress |
| XAML (.NET, Windows) | C ABI (P/Invoke) | in progress |
