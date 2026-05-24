# VisiCalc — Flutter demo

Third cross-backend visual demo (Phase 2 / VC2-flutter), running on
the Flutter framework.

## What it shows

A `MaterialApp` shell containing:

- An auto-generated `FormulaBar extends StatelessWidget` (from
  `lib/generated/formula_bar.dart`, produced by
  `mosaic-compile --backend flutter` from
  `demo/visicalc/mosaic/FormulaBar.{mil,desktop.mll,dark.msl}`).
- A hand-written `Grid extends StatelessWidget` (in
  `lib/generated/grid.dart`), visually matching what the eventual
  Flutter Grid emitter should produce.

Tap a cell — the formula bar updates with its value and the selected
cell gets the excel-blue highlight (`#264F78` outline / `#007ACC`).
Type in the formula bar — it updates the local state.

5×5 sample spreadsheet hard-coded in `lib/main.dart`'s `_sampleRows`.
Same data as VC2-html and VC2-webcomp so all three demos look
visually identical.

## How to build the generated FormulaBar

```bash
bash scripts/build.sh
```

This invokes `mosaic-compile --backend flutter` against the canonical
Mosaic sources and writes `lib/generated/formula_bar.dart`.

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

## The Grid gap

The `mosaic-emit-flutter` pipeline emits a placeholder
`SizedBox.shrink()` for the `Grid` built-in primitive — only the
React emitter knows how to lower it into a real table widget. Until
the Flutter Grid emitter lands, `lib/generated/grid.dart` is
**hand-written** to mirror what the auto-generated widget should
produce.

The hand-written Grid uses native Flutter widgets (`Column` of
`Row`s with `Container` cells, `InkWell` for taps) styled to match
`Grid.dark.msl`'s palette — same look as VC2-html and VC2-webcomp.

When the Flutter Grid emitter lands, `build.sh` gains a second
`mosaic-compile --backend flutter` invocation that overwrites
`lib/generated/grid.dart` with the auto-generated output.

## Where this fits in the cross-backend demo plan

| Phase | Demo | Status |
|---|---|---|
| 2 | VC2-html | ✅ |
| 2 | VC2-webcomp | ✅ |
| 2 | VC2-flutter (this one) | ✅ |
| 2 | VC2-qt | TODO |
| 2 | VC2-swiftui | TODO |
| 2 | VC2-xaml | TODO |
| 3 | multi-component artifact-builder shells | TODO |
| 4 | demo/visicalc-all/ | TODO |
