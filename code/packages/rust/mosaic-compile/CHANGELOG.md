# Changelog — mosaic-compile

## Unreleased

### Added - warn on style parts that match no layout part (`--strict-style`)

After compiling the `.msl`, mosaic-compile now checks it against the resolved
layout's part map via `mosstyle_compiler::unmatched_parts` and prints a warning
for every style `part` whose name matches no exported part — with a
`did you mean` suggestion when a sub-path tail (`sheet/cell` → `cell`) is itself
an exported part. The mosstyle `validate` step is deliberately lenient about
sub-path names (it only checks the top-level segment), so a stale `sheet/cell`
that the emitter silently ignores used to compile clean and render unstyled —
that is how the VisiCalc light-theme grid lost its gridlines. The new `--strict-style`
flag escalates the warning into a hard error (exit 1) for CI that wants to fail
on stale stylesheets. Default behavior is unchanged (warning only, exit 0).

### Changed - shared package-reference resolver

Pipeline mode now uses `mosaic-package-resolver::LayoutPackageResolver` for
`pkg::P::C` layout inlining instead of carrying a private resolver copy inside
the CLI crate.

### Added — pipeline mode for HTML / WebComponent / SwiftUI / Qt / Flutter (VC2-html bonus)

Pre-existing: pipeline mode (`--interface --layout --style`) only
supported `--backend react` and `--backend xaml`. The other five
backends required the legacy single-file `.mosaic` mode.

This release wires every emit-crate's `pipeline::from_pipeline()`
into the CLI dispatch, so `mosaic-compile --backend html|
webcomponent|swiftui|qt|flutter` now works in pipeline mode:

```
mosaic-compile --backend html \
  --interface code/programs/mosaic/visicalc/FormulaBar.mil \
  --layout    code/programs/mosaic/visicalc/FormulaBar.desktop.mll \
  --style     code/programs/mosaic/visicalc/FormulaBar.dark.msl \
  -o FormulaBar.html
```

The `paint` backend stays single-file only — it's a raster pipeline
that bypasses the three-IR compile chain.

A new shared `emit_single_file()` helper unifies the
"write-and-log" code for every single-file backend (HTML / Webcomp /
SwiftUI / Qt / Flutter all produce one string per compile). XAML
still has its own arm because it emits a multi-file triple
(.xaml + .xaml.cs + .Event.cs) plus per-For RowVm side files.

This unblocks the VisiCalc Phase 2 visual-demo cycle — VC2-html
needs `mosaic-compile --backend html` to land before it can wire
its build.sh; VC2-flutter / VC2-qt / VC2-swiftui / VC2-webcomp
will exercise the same path.

### Added — `--variant` flag for multi-layout pipelines (UI30 / ML1)

New `--variant <name>` flag plus directory-mode resolution on
`--layout` implements the UI30 multi-layout spec:

- **File-path mode (unchanged):** `--layout path/to/Grid.desktop.mll`
  uses the file verbatim. Every existing build script keeps working
  byte-for-byte. Passing `--variant` in this mode logs a warning
  (the flag is ignored) but doesn't fail.
- **Directory mode (new):** `--layout path/to/src/ --variant touch`
  reads the `.mil`'s component declaration to learn the component
  name C, then resolves `path/to/src/C.touch.mll`. If that file
  doesn't exist, falls back to bare `path/to/src/C.mll` (the
  default variant). If neither exists, prints a clear "looked
  for: <list>" error and exits 1.
- **Default-variant semantics:** omitting `--variant` is equivalent
  to `--variant default`, which after the variant-file probe falls
  through directly to bare `<C>.mll`. The string `default` is
  reserved and cannot appear in a filename.

The implementation is a single new `resolve_layout_path()` helper
called once at the top of `run_pipeline()`. The rest of the
pipeline is unaware of variants — every downstream stage just sees
the resolved file path. This keeps the diff small and the
backwards-compatibility surface narrow.

Smoke verified end-to-end against the VisiCalc sources:
- `--layout code/programs/mosaic/visicalc/FormulaBar.desktop.mll` (file
  mode) and `--layout code/programs/mosaic/visicalc/ --variant desktop`
  (directory mode) produce **byte-identical** `.tsx` output.
- Fallback path: a tempdir with only `FormulaBar.mll` (no
  `.touch.mll`) compiles cleanly with `--variant touch`,
  matching the bare-default output exactly.
- Error path: a directory with neither `FormulaBar.touch.mll`
  nor `FormulaBar.mll` exits 1 with `looked for:
  FormulaBar.touch.mll, FormulaBar.mll`.

### Added — `pkg` subcommand (UI29 §4.3 / U29-R3)

`mosaic-compile pkg <PACKAGE_ROOT> --backend <name> --output <DIR>` compiles
every component in a Mosaic package to backend-specific source files via
the three-language pipeline (manifest → moslayout + mosmodel + mosstyle →
emitter). The implementation lives in the new
`mosaic-package-artifact-builder` crate; this CLI is a thin shell that
maps argv to a `BuildOptions` and prints the resulting artifact paths.

Wired package backends now match the package artifact builder: React (`.tsx`),
SwiftUI (`.swift`), Qt (`.qml`), XAML (`.xaml` + code-behind), WebComponent
(`.js`), HTML (`.html`), and Flutter (`.dart`). The package-scoped
`--emit-project` flag forwards to the builder so app packages can emit runnable
project shells, including XAML/WinUI output, from the normal CLI path.

The root-level `--backend` flag is now `required: false` at the spec
level so the new subcommand can declare its own scoped `--backend`. The
root-mode runtime check still enforces presence, so existing
`mosaic-compile --backend X SOURCE.mosaic` invocations are unchanged.

### Added — three-file pipeline mode (UI23 / UI24)

The CLI now supports two mutually exclusive modes:

- **Legacy single-file mode** (unchanged): `mosaic-compile --backend X SOURCE.mosaic`
- **Pipeline mode** (new): `mosaic-compile --backend react --interface I.mil --layout L.mll --style S.msl`

In pipeline mode the CLI runs `mosmodel-compiler`, `moslayout-compiler`,
and `mosstyle-compiler` in sequence (passing the descriptor JSON and part
map JSON between them for cross-IR validation), then invokes
`mosaic_emit_react::pipeline::from_pipeline` to produce a `.tsx` file with
the UI24 Flux dispatch shape.

Currently only `--backend react` is wired in pipeline mode. The other
backends (`webcomponent`, `html`, `paint`) continue to work in legacy
mode and will gain pipeline support in follow-up PRs.

New flags:
- `--interface PATH` — `.mil` mosmodel interface file
- `--layout PATH` — `.mll` moslayout file
- `--style PATH` — `.msl` mosstyle file

The positional `SOURCE` argument is now optional (legacy mode); the CLI
rejects invocations that mix it with the pipeline flags.

Added a dependency on `mosmodel-compiler`, `moslayout-compiler`, and
`mosstyle-compiler`.

## 0.1.0 — 2026-05-11

Initial release.

- Implement `mosaic-compile` CLI binary.
- Reads `.mosaic` source files via `mosaic-analyzer`.
- Drives compilation via `MosaicVM`.
- `--backend webcomponent` → emits a Custom Element JS file via
  `mosaic-emit-webcomponent`.
- `--backend html` → emits a static HTML snapshot via `mosaic-emit-html`.
- `--backend react` → emits a React JSX functional component via
  `mosaic-emit-react`.
- `--backend paint` → emits a raster PNG image via `mosaic-emit-paint`.
  This backend bypasses `MosaicVM` and calls `mosaic-emit-paint::render_png_with_defaults`
  directly; it produces binary output written with `write_bytes_or_die`.
- `--fixtures <path>` — load a JSON fixture file for slot value resolution
  (used by the `html` backend).
- `--css <path>` — load a CSS file to inline in the HTML output
  (used by the `html` backend).
- `--output <path>` / `-o` — override the default output file path.
  Default: `<ComponentName>.js`, `.html`, `.jsx`, or `.png` depending on backend.
- `--help` / `-h` — print usage information.
- `--version` / `-V` — print the version string.
- All file I/O errors are reported to stderr with a clear message.
