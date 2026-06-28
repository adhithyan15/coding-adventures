# Changelog

## Unreleased

- Added the reusable `mosaic-pkg-session-progress` dependency and mounted its
  `SessionProgress` component in the app shell.
- Added a multi-backend artifact-builder smoke test proving `EngramApp` emits
  through HTML, React, SwiftUI, Qt, XAML, and Flutter while consuming
  `mosaic-pkg-review-card` and `mosaic-pkg-session-progress`.
- Asserted that nested package styles from `SessionProgress`, `ReviewCard`, and
  `RatingControls` reach the generated Engram HTML artifact.

## 0.1.0

- Added the initial Engram Mosaic app package.
- Added an `EngramApp` root component that consumes `ReviewCard` from
  `mosaic-pkg-review-card`.
- Added smoke tests for manifest boundaries, source compilation, and component
  dependency resolution.
