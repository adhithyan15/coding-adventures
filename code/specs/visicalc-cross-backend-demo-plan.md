# VisiCalc cross-backend visual-demo plan

> **Status.** Survey + plan (no implementation in this PR).
>
> **Goal.** A *visual* VisiCalc demo (not feature-complete) running on
> every Mosaic backend — React, HTML, WebComponent, SwiftUI, Qt, XAML,
> and Flutter (new). "Visual" means the user sees the spreadsheet grid
> and formula bar painted correctly; "not feature-complete" means cell
> formulas, recalculation chains, multi-sheet support, etc. are
> explicitly out of scope. The same `.mil` / `.mll` / `.msl` sources
> drive every backend.

---

## 1. Current state

### What ships today

`code/programs/typescript/visicalc/` exists and works **on React only**:

- **Mosaic sources** (`code/programs/mosaic/visicalc/`):
  - `Grid.mil` — interface (column-headers, viewport-rows, column-
    widths, selection slots, onNavigate emit)
  - `Grid.desktop.mll` — one-line wrapper around the legacy `Grid`
    primitive (UI26 era)
  - `Grid.dark.msl` — dark-theme styles
  - `FormulaBar.mil` — interface (cell-address, formula, read-only;
    onFormulaChange/onCommit/onCancel)
  - `FormulaBar.desktop.mll` — Row[bar] containing a Text label and
    the legacy `Input` primitive
  - `FormulaBar.dark.msl` — dark-theme styles
- **Host application** (`code/programs/typescript/visicalc/src/app/`):
  - `state.ts` — reducer, AppState
  - `util.ts` — A1 cell-address helpers, viewport builder
  - `App.tsx` — useReducer + composes Grid + FormulaBar
  - `main.tsx` — React 18 root mount
- **Build glue**: `scripts/build.sh` runs `mosaic-compile --backend
  react` and writes `src/components/{Grid,FormulaBar}.tsx`.

### What's missing for cross-backend

1. **The sources use deprecated primitives.** `Grid` is the UI26-era
   built-in; UI29 moved it into the userland `mosaic-pkg-grid`
   package. `Input` is the UI25-era built-in; UI29's `HostInput` is
   the kernel-canonical replacement. Five of the six existing
   backends already lower both (Qt's QML emitter, the React emitter,
   etc.) but the long-term plan is to retire `Grid`/`Input` entirely
   under U29-X1. The VisiCalc sources should be migrated first so
   they stay valid past the retirement.
2. **No host shell for any non-React backend.** Compiling the Mosaic
   triple to e.g. Qt produces a `Grid.qml` and `FormulaBar.qml`, but
   there's no `main.qml` that mounts them, no `qmake` project, no
   QtQuick `Window`. Same gap on SwiftUI (no `App` scaffold), HTML
   (no `index.html` that wires the fragments), XAML (no `.csproj` —
   though `mosaic-emit-xaml --emit-project` does some of this for one
   component, not for a multi-component demo), and Flutter (the
   backend itself doesn't exist yet — that's the parallel PR).
3. **No mock-data shim.** The React App owns a real reducer that
   computes viewport rows from `state.cells`. To get a *visual*
   demo on a non-React backend without rebuilding the reducer in
   Swift/Dart/C#/QML/JS, every backend host needs a tiny "mock data"
   harness that hard-codes a sample dataset (5×5 grid with a few
   labels, "=A1+B1" string in one cell, etc.) and never updates it.

## 2. What "visual demo, not feature-complete" means

**In scope per backend:**

- The spreadsheet grid renders with column headers (`A`, `B`, `C`, …),
  row numbers (`1`, `2`, `3`, …), and visible cell contents from
  the mock dataset.
- The formula bar renders at the top with the address label (e.g.
  "`A1`") and the formula text input field (showing the selected
  cell's content).
- The selected cell has a visible highlight; the editing cell (if
  any) has a different highlight.
- Sticky header (column header stays visible when scrolling rows)
  works visually.

**Explicitly out of scope:**

- Live recalculation. The mock data is static — the formula bar
  shows the raw formula string, never the computed value.
- Edit commit. Pressing Enter does nothing (or just shows a
  no-op dispatch).
- Multi-sheet support. One sheet, fixed dimensions, no sheet tabs.
- Save/load. No file persistence.
- Undo/redo. No history.
- The full A1 formula parser. The mock data carries pre-formatted
  cell strings.
- True host-platform integration (menu bars, keyboard shortcuts
  beyond arrow-key navigation in the grid, OS-native file dialogs).

The bar: **a screenshot of each backend should look unmistakeably
like VisiCalc.** That's it.

## 3. Per-backend gap matrix

| Backend     | Mosaic source migration | Host shell needed                       | Estimated effort |
|---|---|---|---|
| React       | HostInput-ize FormulaBar | Already exists (`App.tsx`, mock-data path) | Trivial — just the source migration |
| HTML        | HostInput-ize            | New `index.html` that includes the compiled fragment + minimal CSS reset + hard-coded slot tokens substituted by simple Handlebars-style replace | Small |
| WebComponent| HostInput-ize            | New `index.html` that loads the compiled `.js` bundle and mounts `<visicalc-grid>` / `<visicalc-formula-bar>` with hard-coded attribute data | Small |
| SwiftUI     | HostInput-ize            | New `VisiCalcApp.swift` + `ContentView.swift` that wraps the generated Grid/FormulaBar SwiftUI views with mock `@State` data. SwiftPM `Package.swift` for the host shell. | Medium |
| Qt          | HostInput-ize            | New `main.qml` Window mounting `Grid {}` + `FormulaBar {}`, with mock model data via plain QML objects. `CMakeLists.txt` for the qt6 build, or an existing `qmake` setup | Medium |
| XAML        | HostInput-ize            | The `--emit-project` flag already produces a full WinUI 3 shell for *one* component. For VisiCalc we need a two-component shell that mounts Grid + FormulaBar in one window. Extend `mosaic-package-artifact-builder`'s XAML index to emit the same kind of shell for multi-component packages. | Medium |
| **Flutter** | HostInput-ize            | The Flutter backend itself doesn't exist yet (separate parallel PR — `mosaic-emit-flutter`). Once it lands, the host is a `main.dart` with a `MaterialApp` mounting the generated widgets. | New backend + host |

The "Mosaic source migration" column is the same one-time edit
across every backend — kill the legacy `Input` reference in
`FormulaBar.desktop.mll`, replace with `HostInput`. The legacy
`Grid` primitive is harder to retire (the rich grid behaviour —
sticky header, cell editing, viewport scrolling — currently lives
in the per-backend `emit_grid_*` functions). For the visual demo,
keep the legacy `Grid` for now; deferring the `mosaic-pkg-grid`
migration to a separate post-demo cleanup PR is the right call.

## 4. Recommended implementation roadmap

The plan below is **ordered for shortest path to first cross-backend
screenshot**, not by per-backend completeness.

### Phase 1 — source-level cleanup (1 PR)

1. **VC1.** Migrate `FormulaBar.desktop.mll` from `Input` to
   `HostInput`. Bonus: light-theme `.msl` files alongside the dark
   ones, so backends with different defaults (e.g. WinUI's light
   theme) render correctly. Verify the React demo still works.

### Phase 2 — minimal cross-backend hosts (1 PR per backend)

Each PR adds ONE backend's host scaffold + mock-data harness, runs
`mosaic-compile`, and produces a runnable visual demo:

- **VC2-html.** Static `code/programs/typescript/visicalc-html/index.html` that includes
  the compiled `Grid.html` + `FormulaBar.html` fragments with
  hard-coded slot substitutions. Open in browser, see VisiCalc.
- **VC2-webcomp.** `code/programs/typescript/visicalc-webcomp/index.html` that imports
  the `.js` bundle and mounts the custom elements with attributes.
  Open in browser, see VisiCalc.
- **VC2-qt.** `code/programs/cpp/visicalc-qt/` with `main.qml`, `CMakeLists.txt`,
  and a `qmldir` shim. `qmlscene main.qml` shows VisiCalc.
- **VC2-swiftui.** `code/programs/swift/visicalc-swiftui/` with `Package.swift`,
  `VisiCalcApp.swift`, `ContentView.swift`. `swift run` shows
  VisiCalc on macOS.
- **VC2-xaml.** `code/programs/csharp/visicalc-xaml/` with the multi-component
  WinUI 3 shell + mock VM. `dotnet build` + `dotnet run` shows
  VisiCalc on Windows. (May require the artifact-builder change in
  Phase 3 first.)
- **VC2-flutter.** Requires Flutter backend (separate parallel PR).
  `code/programs/dart/visicalc-flutter/main.dart` mounts the generated widgets.
  `flutter run` shows VisiCalc.

Each PR is independent — they can land in any order once Phase 1
is in.

### Phase 3 — artifact-builder multi-component shells (optional, 1 PR)

`mosaic-emit-xaml`'s `--emit-project` flag currently emits a shell
for ONE component (the layout root). For multi-component demos like
VisiCalc, extend `mosaic-package-artifact-builder`'s XAML index
emitter to also produce a `MainWindow.xaml` that mounts every
component in a vertically-stacked layout. Same idea for WebComponent
(generate an `index.html` that mounts every custom element) and
HTML (generate a multi-fragment page).

This phase is optional — the per-demo `VC2-*` PRs above can hand-
write their hosts. But pulling the multi-component shells into
artifact-builder removes the boilerplate burden from every future
multi-component demo (so e.g. the next VisiCalc-style demo only
needs to write its Mosaic sources and a tiny mock-data shim).

### Phase 4 — convergence (1 PR)

A `code/programs/typescript/visicalc-all/` directory (or a `Makefile` at the demo root)
that builds + runs every backend's host in one command, producing
seven screenshots side-by-side. Useful as a regression smoke test
for the Mosaic compiler and as a portfolio piece.

## 5. Open questions for the implementation PRs

1. **Mock-data shape.** Every backend host needs a tiny "5×5 sample
   spreadsheet" dataset. Should it be a shared JSON file the hosts
   each parse, or hard-coded in each host's native syntax? Lean
   toward shared JSON (single source of truth) but accept the
   per-host duplication if JSON parsing is awkward on a backend (Qt
   without QML JS plugins, e.g.).
2. **Theme handling.** VisiCalc currently ships dark-only. SwiftUI's
   default is light-mode-system-follows-OS. Should we ship light
   `.msl` variants of the existing dark styles, or force-dark each
   host shell? Lean toward shipping `Grid.light.msl` and
   `FormulaBar.light.msl` alongside the existing `.dark.msl` files,
   so each backend's `--style` flag can pick whichever theme matches
   the host OS default.
3. **Sticky header on non-React.** The `sticky-header: true` keyword
   in `Grid.desktop.mll` triggers special CSS-position-sticky wiring
   in the React emitter. Qt has scrollable item-view header pinning
   (`headerView` or a separately-anchored `RowLayout`); SwiftUI's
   `Section` headers can pin in a `LazyVStack`. Audit each backend
   to confirm the sticky behaviour actually fires for `Grid` and
   document any backend that degrades to non-sticky for v1.
4. **Cell-edit visualisation.** The selected/editing cell highlight
   currently uses the `state` blocks in `Grid.dark.msl`
   (`part sheet/cell:selected`, `part sheet/cell:editing`).
   Confirm every backend's emitter consumes these — XAML and Qt
   need spot checks.

## 6. Out of scope

- Live formula evaluation (deferred to a "feature-complete" demo).
- Multi-sheet support, save/load, undo/redo.
- The `mosaic-pkg-grid` migration that retires the legacy `Grid`
  primitive (separate post-demo cleanup PR).
- Touch / mobile-specific layouts (`.touch.mll` variants). The
  desktop layouts are the only target for the visual demo.

---

**Reviewer checklist:**

- [ ] Does Phase 1's source cleanup risk breaking the existing React
      demo? (Author: no — `HostInput` lowers to `<input>` exactly as
      the legacy `Input` did, modulo the multiline branch which
      VisiCalc doesn't use.)
- [ ] Is the per-backend effort estimate (column "Estimated effort"
      in §3) realistic, given each backend's tooling maturity?
- [ ] Is the mock-data approach acceptable, or should the host
      reducer be migrated to a backend-neutral shape first?
