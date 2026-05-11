# Changelog — mosaic-compile

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
