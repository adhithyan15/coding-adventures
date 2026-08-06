# Changelog

All notable changes to `mosaic-pkg-project-nav` are documented in this
file. The format follows [Keep a Changelog](https://keepachangelog.com/)
and the package follows semantic versioning.

## 0.1.0 — 2026-08-06 — initial release

### Added

- `ProjectNav` component: the nested-project tree + add/add-subproject
  composer, extracted verbatim from `TaskApp`'s own rail block (same part
  names, same styling in both themes, same layout structure — a
  refactor, not a redesign). Built entirely from kernel primitives.
- Verified live, behavior-identical to the pre-extraction inline version:
  create a top-level project, create a nested sub-project (indent glyph
  renders), switch selection between projects (the "on" raised-card
  styling follows the active project), in both themes. Zero console
  errors.

See [task-app-project-nav-v1.md](../../../specs/task-app-project-nav-v1.md)
for the full scope, including what deliberately stayed in `TaskApp` (the
brand row, the view-switcher).
