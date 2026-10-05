# XAML nested text-style inheritance

Related: #14274 and epic #14267.

Generated VisiCalc cells use an alignment-only implicit TextBlock style. WinUI resolves the nearest implicit style, so this replaces the ancestor style carrying the light foreground on a dark background.

Emit a uniquely keyed TextBlock style for every container text-style resource. Derive it from the enclosing generated style with BasedOn, and expose an implicit TextBlock style based on that key. Track the active resource lexically during emission, restoring it after each node and structural table row. Keys are compiler-generated, never authored strings. Preserve local setter precedence, template inheritance and sibling isolation; do not change generated output by hand or add application-specific branches.

Validate nested color/alignment and local overrides, siblings, repeated/conditional content and structural table rows. Run the emitter suite, generate and build VisiCalc with its standard Rust adapter, then inspect the actual Windows app and commit a formula in an isolated state fixture. This fixes text-style composition only; overall native design, accessibility, persistence and release acceptance remain open.

## Validation recorded 2026-10-05

- `cargo test --manifest-path code/packages/rust/Cargo.toml -p mosaic-emit-xaml --quiet`: 352 unit tests and 13 integration tests passed; one ignored doctest.
- Built the standard `visicalc-mosaic-app` Rust DLL, then generated the application with `mosaic-compile pkg ... --backend xaml --emit-project --profile permissive --runtime-library ...`.
- Fresh generated root: `dotnet build -p:Platform=x64 -v:quiet` passed with zero errors and eight existing WMC1506 binding warnings.
- Actual WinUI window inspected through computer-use at 1908 x 1018. The body values now render light on the dark green background; headers remain aligned and visually distinct. The generated repeater/conditional cell templates successfully resolve the ancestor style chain at runtime.
- Launched with an isolated `MOSAIC_APP_STATE_PATH` fixture. Formula-field Value changed from `=8+9` to `=19+23`; Return produced accessibility text `A1, 42, formula =19+23`, status `Mosaic runtime handled onCommit`, visible A1 `42`, and dependent totals E1 `65`, A5 `66`, E5 `196`.
- Remaining observed defects: selection is not clearly visible, formula input is too narrow, and numeric text does not occupy the full cell width for right alignment. Keep #14274 and full native acceptance open. This is not a packaged-release or complete accessibility/persistence acceptance result.
- Security subagent review passed with no vulnerabilities found.

## CI repair for #16719

The repository metadata gate references a deleted PR-head commit for the merged Elixir/Lua barcode adoption. Bind registry and owning backlog-item implementation/validation revisions to verified squash merge 9a99df33af2dc2dfe89e0748422e4ba387f97e22. Keep package-tree hashes unchanged, mark that item merged, and preserve unrelated parity-loop scheduling state for its owner. Validate the neutral fixture contract and JSON state. No gate weakening or package implementation changes.

The main refresh also brings merged PR #16717. Apply the same durable-evidence rule to its Java/Kotlin/Dart targets after verifying all three tree hashes against squash merge 103296279680459f634c213a1cf3828367e71568. Preserve main's complete Elixir/Lua merge record when resolving the overlapping state edit.
