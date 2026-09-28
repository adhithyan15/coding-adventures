### Added — End-to-end integration test against `mosaic-pkg-grid`

- New `tests/pkg_grid_compiles_to_xaml.rs` integration test.
- Resolves `mosaic-pkg-grid`'s source root relative to
  `CARGO_MANIFEST_DIR` (steps up four directory levels), then
  compiles each component (`Grid`, `Cell`, `Column`) through the
  three IR compilers and the XAML emitter.
- 5 tests cover: package source resolution; each component lowers
  through `from_pipeline` without error; Grid (the complex
  component using HostTable + For + Cell component reference)
  produces the expected XAML structure (UserControl root,
  ItemsRepeater for For, `<grid:Cell/>` reference, xmlns:grid
  declaration); Grid produces RowVm side-files.
- This is the spec §17 PR-6 capstone — the XAML emitter is
  "done" in the spec sense when `mosaic-pkg-grid` compiles cleanly
  end-to-end, which it now does.

