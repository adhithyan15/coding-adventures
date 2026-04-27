# Changelog

All notable changes to `@coding-adventures/window-core` will be documented in
this file.

## 0.1.0

- Added the TypeScript mirror of the shared window-core contract.
- Added builder validation for logical sizes, mount targets, and min/max bounds.
- Added normalized event, render-target, and backend interfaces for browser and
  native adapters.
- Added unit tests covering validation, size conversion, and backend handoff.
- Expanded unit coverage for shared modifier defaults, pointer-button helpers,
  scale-factor validation, dimension overflow guards, and builder flag toggles
  so the package clears the repository coverage threshold in CI.
