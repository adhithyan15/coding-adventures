# Changelog

All notable changes to `mosaic-dev` will be documented in this file.

## [Unreleased]

### Fixed

- Handle `one-of` slots in preview dummy props, using the first declared
  member so generated hosts receive a value from the closed set. Restores
  compilation against the new `SlotType::OneOf` variant.

## [0.1.0] — 2026-05-20

### Added

- Initial release of `mosaic-dev` — a Storybook-style single-component
  preview runner for Mosaic packages.
- CLI surface: `mosaic-dev <PACKAGE_ROOT> --backend <name> --component
  <Name> [--port N] [--no-open]`.
- Six backend strategies:
  - `react` — drives `mosaic_package_artifact_builder::build_package`
    with `Backend::React`, auto-generates `index.html` + `main.tsx`,
    spawns `npx vite` on the configured port, opens the browser.
  - `html` — drives `build_package` (stubbed where the kernel pipeline
    isn't wired yet), serves the generated `index.html` from an
    embedded `tiny_http` server.
  - `webcomponent` — same shape as `html` but auto-registers the
    component as a custom element and instantiates it via attributes
    on a `<tag>`.
  - `swiftui` — drives `build_package` with `Backend::SwiftUI`,
    auto-generates a SwiftPM host project, spawns `swift run`.
  - `qt` — drives `build_package` with `Backend::Qt`,
    auto-generates a `main.qml` host, spawns `qmlscene`.
  - `xaml` — returns a clear "not yet supported" error pending
    Windows-host integration.
- File-system watching via the `notify` crate, scoped to
  `<PACKAGE_ROOT>/src/` and the three Mosaic source extensions
  (`.mil` / `.mll` / `.msl`).
- 100ms debounce on file events so a single editor save doesn't trigger
  multiple rebuilds.
- Auto-generated host wrappers that pull dummy slot values from the
  component's `.mil` interface, honouring inline defaults.
- 14 unit tests covering dummy-prop generation, every backend's wrapper
  output, and CLI parsing.

### Known limitations (deferred to future PRs)

- SwiftUI and Qt use full-process restart on change (no HMR).
- HTML / WebComponent require manual browser refresh (SSE auto-reload
  is a follow-up).
- One component per `mosaic-dev` invocation.
- No `.mosaic-dev.toml` for custom dummy-prop overrides yet.
- XAML backend requires Windows; not yet implemented.
