# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Added

- `paint_line`, `paint_glyph_placement`/`paint_glyph_run`, `paint_group`,
  `paint_clip`, `paint_layer`, `transform2d`/`identity_transform`/
  `is_identity_transform` -- brings this package's instruction set up to the
  full `P2D02-paint-vm-ascii.md` contract (`rect`/`line`/`glyph_run`/
  `group`/`clip`/`layer`), needed by `code/packages/lua/paint_vm_ascii` and
  the new `code/programs/lua/cowsay` port.
- `paint_rect` gained two new trailing optional parameters, `stroke` and
  `stroke_width` (both default to "no stroke", matching the pre-existing
  behavior). This is purely additive: every existing call site
  (`barcode-2d`, `barcode_layout_1d`) calls `paint_rect` with five or fewer
  positional arguments and is unaffected.

No version bump: this package is still pre-1.0, the changes are entirely
additive (no existing field, function signature prefix, or return shape
changed), and no consumer's rockspec pins an exact revision that this would
invalidate.

## [0.1.0] - 2026-04-12

### Added

- Initial `PaintScene` and rectangle instruction primitives
