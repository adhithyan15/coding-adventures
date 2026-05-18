# Changelog — mosaic-compile

## Unreleased

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
