# Changelog — mosaic-driver

## [0.1.0] — 2026-05-11

### Added

- Initial implementation of the `mosaic` CLI binary.
- Full three-stage pipeline: `.mil` → mosmodel → `.mll` → moslayout → `.msl` → mosstyle.
- `--interface <path.mil>` flag — run only the model stage.
- `--layout <path.mll>` flag — run only the layout stage.
- `--style <path.msl>` flag — run only the style stage.
- Default mode: `mosaic <ComponentName>` — loads `<Name>.mil`, `<Name>.mll`, `<Name>.msl`
  from the current directory, runs all three stages, and prints a JSON summary.
- Passes `interface_json` from mosmodel to moslayout for slot-ref validation.
- Passes `part_map_json` from moslayout to mosstyle for part-name validation.
- JSON summary output embeds structured `interface` and `parts` objects (not
  double-encoded strings) alongside the raw `css` string.
