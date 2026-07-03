# Changelog

## Unreleased

- Added a light-theme stylesheet (`NoteEditor.light.msl`) mirroring the dark theme's structure with a light palette. Selected at build time via `mosaic-compile pkg --theme light` (the style analogue of the layout `--variant`).


## 0.1.0

- Add `NoteEditor`, a reusable focused-field note editor component with field
  selection, selected field editing, tag editing, and save/delete/cancel emits.
- Add portable deck and note-type option lists so generated shells can create
  notes without a host-specific modal.
