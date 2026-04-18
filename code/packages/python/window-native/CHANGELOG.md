# Changelog

All notable changes to `coding-adventures-window-native` will be documented in
this file.

## 0.1.0

- Added the first Python native window package built on `python-bridge`.
- Added a Rust extension module that creates and tracks native windows through
  `window-core` and `window-appkit`.
- Added a Python wrapper surface with size dataclasses, surface/render-target
  enums, a `Window` wrapper, and `create_window(...)`.
- Added tests for the macOS hidden-window smoke path and non-macOS unsupported
  behavior.
- Hardened the package BUILD scripts against symlinked artifact destinations and
  validated window sizes before crossing the Python/Rust boundary.
- Reworked the Unix `BUILD` staging steps to avoid heredocs inside command
  substitutions so Linux `/bin/sh` runners can execute the package checks
  portably.
- Reworked the Unix `BUILD` execution again so the staged import directory is
  created and reused without depending on shell variables surviving across
  separate repo runner invocations.
- Added platform-neutral wrapper tests so Linux CI still covers the Python
  window facade when the native AppKit smoke path is skipped.
