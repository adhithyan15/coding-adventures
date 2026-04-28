# Changelog — coding_adventures_barcode_2d (Elixir)

All notable changes to this package are documented here.

---

## [0.1.0] — 2026-04-24

### Added

- `CodingAdventures.Barcode2D.ModuleGrid` struct — the universal intermediate
  representation for all 2D barcode encoders. Holds `rows`, `cols`, a
  list-of-lists boolean `modules` grid, and a `module_shape` (`:square` or
  `:hex`).

- `CodingAdventures.Barcode2D.Barcode2DLayoutConfig` struct — pixel-level
  layout options with defaults matching the TypeScript implementation:
  `module_size_px: 10.0`, `quiet_zone_modules: 4`, `foreground: "#000000"`,
  `background: "#ffffff"`, `show_annotations: false`, `module_shape: :square`.

- `make_module_grid/3` — creates an all-light `ModuleGrid` of the given
  dimensions and module shape. Starting point for every encoder.

- `set_module/4` — pure immutable single-module update. Returns
  `{:ok, new_grid}` on success or `{:error, reason}` for out-of-bounds access.
  Uses `List.replace_at/3` to avoid mutating the original grid.

- `layout/2` — converts a `ModuleGrid` to a `PaintScene` (from the
  `paint_instructions` package). Dispatches to `layout_square` or `layout_hex`
  based on `module_shape`. Returns `{:ok, scene}` or `{:error, reason}`.

- Square rendering: each dark module → one `paint_rect`. Total canvas size
  includes quiet zone on all four sides.

- Hex rendering (MaxiCode): each dark module → one `paint_path` with seven
  commands (`move_to`, five `line_to`, `close`) tracing a flat-top regular
  hexagon. Odd rows offset right by `hex_width / 2` for standard hexagonal
  tiling.

- Full ExUnit test suite with 100% coverage (61 tests, 0 failures), covering
  all happy paths, edge cases, and all validation error branches.
