# Changelog

## Unreleased

- Composed `ReviewCard` from the reusable `mosaic-pkg-rating-controls`
  package instead of owning the answer grading button row directly.
- Switched multi-backend smoke coverage to the package artifact builder so
  nested component dependencies and their styles are verified together.

## 0.1.0

- Added the initial `ReviewCard` Mosaic component package.
- Added smoke tests that compile the component through mosmodel, moslayout,
  mosstyle, and the React, HTML, SwiftUI, XAML, Qt, Compose, and Flutter
  pipeline emitters.
