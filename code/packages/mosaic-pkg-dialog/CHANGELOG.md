# Changelog

All notable changes to `mosaic-pkg-dialog` are documented in this file.
The format follows [Keep a Changelog](https://keepachangelog.com/) and
the package follows semantic versioning.

## 0.1.0 — 2026-05-20

Initial release.  Ships a single Dialog component used as the
cross-backend smoke test for the UI29 kernel.

### Added

- `mosaic-package.toml` manifest declaring
  `[components].exports = ["Dialog"]` and targeting UI29 kernel
  version `"1"`.
- `Dialog.mil`: interface with three text slots (`title`, `message`,
  `close-label`) and one zero-payload emit (`onClose`).
- `Dialog.mll`: layout composed of `Box` (outer frame, `dialog-root`),
  `Column` (vertical stack, `dialog-stack`), and three inner `Box`
  parts (`dialog-title`, `dialog-message`, `dialog-actions`) wrapping
  two `Text` leaves and one `HostButton`.
- `Dialog.dark.msl`: dark-theme stylesheet covering all four named
  parts.
- `tests/package_compiles.rs`: integration smoke test that round-trips
  every source file through `mosmodel-compiler`, `moslayout-compiler`,
  and `mosstyle-compiler`, asserts the structural shape of the layout
  tree, and drives `mosaic-package-artifact-builder::build_package` for
  every backend the builder currently supports (React, SwiftUI, Qt) —
  asserting a non-empty `Dialog.<ext>` lands on disk for each.

### Deliberate scope decisions

- **Only Box, Column, Text, HostButton.**  The "lowest-common-denominator"
  primitive set — every Mosaic backend on `main` today (React, SwiftUI,
  Qt, WebComponent, HTML, XAML) lowers all four.  No `If`, no `For`, no
  `HostTable`, because those primitives are still landing in parallel
  PRs and are not present in every backend yet.
- **Visibility is host-controlled.**  v0.1.0 has no `visible: bool`
  slot; the host decides whether to mount Dialog by including or
  excluding it from its parent's layout tree.  This works on every
  backend today and avoids gating v0.1.0 on cross-backend `If` parity.
- **WebComponent and HTML artifact-builder skip.**  The
  `mosaic-package-artifact-builder` crate currently returns
  `UnsupportedBackend` for `Backend::WebComponent` and `Backend::Html`
  (their `from_pipeline` entry points are landing in parallel PRs).
  The smoke test asserts that current behaviour, so the day those PRs
  land the test fails loudly and a contributor knows to promote the
  variants into `SUPPORTED_BACKENDS`.
- **XAML is not exercised.**  XAML is not yet a variant of the artifact
  builder's `Backend` enum (its emitter is still landing under the
  `feat(mosaic-emit-xaml)` PR series), so this package's smoke test
  cannot drive it.  The README's primitive-availability table is the
  source of truth for what *parses*; the smoke test is the source of
  truth for what *the builder lowers today*.

### v0.2.0 plans

- Add `slot visible : bool` plus an `If ( when: slot: visible )` gate
  once `If` lands across React/SwiftUI/Qt (currently only HTML,
  WebComponent, and XAML have it).
- Add an `onOpen` companion emit (optional symmetry with `onClose`).
- Light-theme `Dialog.light.msl` once the multi-theme cascade lands in
  mosstyle.
