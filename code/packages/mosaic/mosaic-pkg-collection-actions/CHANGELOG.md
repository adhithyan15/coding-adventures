# Changelog

## Unreleased


- Added `delete-note-disabled` / `delete-note-type-disabled`, wired to the two
  destructive buttons. A delete with nothing selected used to be a silent
  no-op (#13933); the control is now withdrawn rather than offering a click
  that does nothing.

- Added a light-theme stylesheet (`CollectionActions.light.msl`) mirroring the dark theme's structure with a light palette. Selected at build time via `mosaic-compile pkg --theme light` (the style analogue of the layout `--variant`).


## 0.1.0

- Add `CollectionActions`, a target-neutral collection status and workflow
  action surface for Mosaic-generated study apps.
