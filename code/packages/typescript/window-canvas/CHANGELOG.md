# Changelog

All notable changes to `@coding-adventures/window-canvas` will be documented in
this file.

## 0.1.0

- Added the pure TypeScript browser window backend built on mounted canvas
  elements.
- Added DOM-agnostic environment adapters so the backend can be tested without
  jsdom.
- Added normalized resize, redraw, visibility, pointer, key, and text-input
  event translation.
- Added unit tests covering mount resolution, DPR synchronization, redraw
  scheduling, and input normalization.
- Expanded unit coverage for invalid DPR fallback, secondary and custom pointer
  buttons, and modified or non-printable key handling so the package clears the
  repository coverage threshold in CI.
