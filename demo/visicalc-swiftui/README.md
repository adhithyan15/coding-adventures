# VisiCalc — SwiftUI demo

Fifth cross-backend visual demo (Phase 2 / VC2-swiftui), running on
the SwiftUI framework (macOS / iOS).

## What it shows

A SwiftUI `WindowGroup` containing:

- An auto-generated `FormulaBarView` (from
  `Sources/VisiCalc/Generated/FormulaBar.swift`, produced by
  `mosaic-compile --backend swiftui`).
- A hand-written `GridView` (`Sources/VisiCalc/Generated/Grid.swift`),
  visually matching what the eventual SwiftUI Grid emitter should
  produce.

Tap a cell — the formula bar updates with its value, the selected
cell gets the excel-blue highlight. Type in the formula bar — it
updates the local `@State`.

Same hard-coded 5×5 sample data as the other VC2-* demos.

## How to build the generated FormulaBar

```bash
bash scripts/build.sh
```

Runs `mosaic-compile --backend swiftui` against the Mosaic sources
and writes `Sources/VisiCalc/Generated/FormulaBar.swift`.

## How to run the app

### macOS

```bash
swift run                 # macOS terminal target
```

### iOS Simulator

```bash
xcodebuild -scheme VisiCalc \
  -destination 'generic/platform=iOS Simulator' \
  build
```

Or open `Package.swift` in Xcode and pick an iOS simulator from
the run-destination menu.  Both `VisiCalcApp.swift` and the
generated `FormulaBar.swift` are cross-platform — `AppKit` /
`.onExitCommand` are guarded by `#if os(macOS)` so the same
source ships to macOS, iOS, iPadOS, tvOS, and watchOS.

Requires Swift 5.9+ / Xcode 15+.  iOS Simulator runtime is
optional but recommended.  Install via App Store on macOS.

## Known emitter glitch

The current `mosaic-emit-swiftui` pipeline emits a slightly broken
`onSubmit` handler for the FormulaBar: `dispatch(.commit(value:
formula))` references `commit(value:)`, but the `FormulaBarEvent`
enum's `commit` case carries no associated value. The Swift
compiler will reject this.

Tracked as **UI31-swiftui-commit-arity** for the emitter team.
Workarounds until the emitter ships a fix:

1. Hand-patch the generated `FormulaBar.swift` after each run of
   `build.sh`: change `dispatch(.commit(value: formula))` to
   `dispatch(.commit)`.
2. Or wait for the emitter fix and re-run `build.sh`.

The fix belongs in
`code/packages/rust/mosaic-emit-swiftui/src/pipeline.rs` — likely a
one-line tweak to the `onSubmit` branch of `emit_host_input` to
match the `commit` event's parameter list.

## The Grid gap

The `mosaic-emit-swiftui` pipeline doesn't yet support the `Grid`
built-in primitive — only the React emitter does. Until the
SwiftUI Grid emitter lands, `Sources/VisiCalc/Generated/Grid.swift`
is hand-written.

## Where this fits in the cross-backend demo plan

| Phase | Demo | Status |
|---|---|---|
| 2 | VC2-html | ✅ |
| 2 | VC2-webcomp | ✅ |
| 2 | VC2-flutter | ✅ |
| 2 | VC2-qt | ✅ |
| 2 | VC2-swiftui (this one) | ✅ |
| 2 | VC2-xaml | TODO |
| 3 | multi-component artifact-builder shells | TODO |
| 4 | demo/visicalc-all/ | TODO |
