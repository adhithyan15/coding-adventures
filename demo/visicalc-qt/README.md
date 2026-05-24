# VisiCalc — Qt demo

Fourth cross-backend visual demo (Phase 2 / VC2-qt), running on the
Qt/QML stack.

## What it shows

A `Window` containing:

- An auto-generated `FormulaBar.qml` (from `build/FormulaBar.qml`,
  produced by `mosaic-compile --backend qt`). Exposes properties
  (`cellAddress`, `formula`, `readOnly`) and signals (`formulaChange`,
  `commit`, `cancel`) — the host wires them in `main.qml`.
- A hand-written 5×5 spreadsheet grid below, styled to match
  `Grid.dark.msl`. Tap a cell to select it and pull its value into
  the formula bar.

Hard-coded sample data matches VC2-html / VC2-webcomp / VC2-flutter
so all four demos look visually identical across backends.

## How to build the generated FormulaBar

```bash
bash scripts/build.sh
```

Runs `mosaic-compile --backend qt` against the Mosaic sources and
writes `build/FormulaBar.qml`.

## How to run the demo

Two options. **Either requires Qt 6 SDK installed** (https://www.qt.io/download).

### 1. Quickest: `qml` runner (one command)

```bash
qml main.qml
```

The `qml` binary ships with Qt 6 in `<qt-install>/bin/qml`. It loads
a single QML file into a `QQmlApplicationEngine` with no
compilation step — ideal for development.

### 2. CMake build (real binary)

```bash
cmake -B build-cmake
cmake --build build-cmake
./build-cmake/visicalc_qt_app
```

Builds a standalone executable using the C++ wrapper in
`src/main.cpp`. This is what you'd ship.

## The Grid gap

The `mosaic-emit-qt` pipeline doesn't yet support the `Grid`
built-in primitive — only the React emitter knows how to lower it
into a real table. Until the Qt Grid emitter lands,
`main.qml` inlines a hand-written QtQuick block (nested
`RowLayout`/`Repeater` over the `sampleRows` property) that
visually mirrors what the eventual auto-generated `Grid.qml`
should produce.

## Where this fits in the cross-backend demo plan

| Phase | Demo | Status |
|---|---|---|
| 2 | VC2-html | ✅ |
| 2 | VC2-webcomp | ✅ |
| 2 | VC2-flutter | ✅ |
| 2 | VC2-qt (this one) | ✅ |
| 2 | VC2-swiftui | TODO |
| 2 | VC2-xaml | TODO |
| 3 | multi-component artifact-builder shells | TODO |
| 4 | demo/visicalc-all/ | TODO |
