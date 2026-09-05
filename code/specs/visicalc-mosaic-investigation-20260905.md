# VisiCalc as a Mosaic reference application

Investigation date: 2026-09-05. Audited main commit:
`8e7352f08981dd42e7f502d151c2432688f4d765`.
Status: investigation and proposed implementation sequence; no migration performed.

## Recommendation

Make VisiCalc a reference application for UI38: one Rust application adapter,
one Mosaic application package, and generated platform hosts. Keep the existing
`spreadsheet-core` engine. Use real spreadsheet interactions to discover gaps
in Mosaic's packages, compiler, runtime, and generated hosts.

VisiCalc already uses Mosaic. The migration is from separately wired component
demos to the current standard application runtime, not an initial UI rewrite.

## Current evidence

- `code/programs/mosaic/visicalc/Grid.desktop.mll` references
  `mosaic-pkg-grid::Grid`. That package composes HostTable, nested For loops,
  Cell, conditional editing, and HostInput. The old audit of the legacy Grid
  primitive no longer describes this source.
- FormulaBar is also authored in Mosaic. The React build regenerates both
  components from the shared source directory.
- The shared VisiCalc source directory has no `mosaic-package.toml`, root
  application component, or BUILD file. It is currently a collection of
  component sources consumed by per-host scripts.
- React's `src/app/App.tsx` and `state.ts` own selection, editing, viewport,
  keyboard handling, and engine wiring. C# owns similar behavior independently.
- `UI38-mosaic-native-application-runtime.md` defines the intended replacement:
  `MosaicApp` start/dispatch/snapshot/restore, render props, effects, and
  announcements. `task-mosaic-app` is an existing concrete adapter to study.
- TaskApp is a useful native-host and acceptance example, but its README still
  describes a separate React adapter with a shared presentation fixture. Do not
  assume copying TaskApp automatically gives one adapter on every platform.

## Checks performed

1. Installed React dependencies with `npm install --package-lock=false` and ran
   `npm run build`: passed, including a fresh Mosaic compiler build, component
   generation, TypeScript checking, and Vite production bundling (32 modules).
   `npm ci` was unavailable because this demo has no committed lockfile.
2. Ran `cargo test -p mosaic-emit-xaml --test pkg_grid_compiles_to_xaml`:
   all five tests passed. These verify emission, not a WinUI build or interaction.
3. Transpiled and executed the existing React reducer and helpers without changing
   their behavior. `formulaChange("123")` followed by `commit` from initial state
   leaves `editRow == -1` and A1 uncommitted. App.tsx also gates engine commits on
   an active edit session. Direct formula-bar editing needs an explicit transition.
4. Executed `navigate(row: 30, col: 0)`: selection reaches row 31 while
   `viewportOffset == 0` and `viewportSize == 30`. The current component composition
   contains no scroll event hookup to advance this viewport.

The last two are confirmed host-state defects, not proven compiler defects.
No browser interaction, native launch, accessibility inspection, or full backend
matrix was performed in this investigation.

## First acceptance workflow

Use one versioned fixture with expected engine state and render slots after every
action. Replay it against the Rust adapter, then drive equivalent actions through
real generated React and WinUI controls:

- Open the common budget seed; E5 displays 169.
- Select E1; the formula bar displays its source rather than its computed value.
- Edit A1 to 20 through the formula bar; commit; E1 becomes 43 and E5 becomes 174.
- Edit a cell inline; Escape preserves the prior value; Enter commits once.
- Navigate past row 30; the selected cell remains visible and its coordinates
  round-trip correctly between the viewport and workbook.
- Scroll, resize, and switch theme; header/cell widths and selected/edit styles
  stay aligned. Measure realized rows rather than claiming virtualization from
  the presence of a repeater.
- Save, close, and reopen; workbook values and formulas survive.
- Verify accessible names, keyboard focus, and selection/edit announcements.

Use explicit empty/loading/error cases and canceled file dialogs in later cases.
Do not treat an Electron launch as native Windows acceptance.

## Proposed implementation sequence

1. **Establish the baseline and fixture.** Record existing failures, replace stale
   VisiCalc status claims, and add build plus interaction coverage for the workflow.
2. **Create the standard application boundary.** Add a proposed
   `visicalc-mosaic-app` crate implementing MosaicApp over spreadsheet-core.
   Own selection, edit buffer, viewport, and workbook commands there; implement
   versioned snapshots. Keep formula evaluation and file formats in the existing
   engine/IO crates. Route platform operations through standard host effects.
3. **Compose the application in Mosaic.** Add a package manifest and root
   `VisiCalc.mil/.mll/.msl` that composes FormulaBar, the shared Grid, and necessary
   scroll/file controls. Connect the web host to the same adapter through WASM.
   If the generic bridge lacks a needed capability, fix it in Mosaic.
4. **Prove generated WinUI.** Use the standard native ABI and strict capability
   profile. Build and launch generated output and complete the workflow. Reduce
   failures to small package/compiler/runtime regressions, then fix those upstream.
5. **Extend backend acceptance.** Repeat on Qt, then available Flutter, Compose,
   and SwiftUI runners. Retire old host behavior only after its replacement passes.
   Add multi-sheet, clipboard, undo/redo, and large-sheet scenarios incrementally.

For each failure: preserve a failing user workflow, identify whether it belongs to
the app adapter, component package, emitter, or generic host runtime, add the
smallest regression at that layer, fix it, and rerun the application workflow.
Do not add VisiCalc-specific branches to emitters or patch generated output.

## Existing uncommitted Windows work

The original `codex/visicalc-windows-validation` checkout remains untouched.
Its XAML changes contain useful hypotheses and regression cases: nested row/cell
projection, coordinate payloads, selected/editing styles, column widths, typography,
and flex sizing. Current main already has row-projection and flex-grid machinery,
so port failing cases first and retain only fixes still necessary.

The handwritten C# application behavior is a source of acceptance cases to migrate
into the Rust adapter. Qt linking and Deno/Electron packaging fixes are separate
platform maintenance work and should not block the Mosaic application migration.
The old patch does not apply cleanly to current XAML pipeline or Electron package
metadata; blindly transplanting it would obscure newer implementation changes.

## Completion bar

A backend is accepted only when generated controls build, launch, perform the
shared workflow, restore saved state, and pass accessibility checks with no silent
capability degradation. Source emission and screenshots alone are insufficient.
The desired result is a spreadsheet that continuously tests reusable Mosaic
capabilities, with application behavior maintained once.
