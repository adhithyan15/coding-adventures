# Changelog

## [0.2.0] - Unreleased

- **Breaking: `Surface`'s `children` slot is `items`.** `children` is an
  `Item` property in Qt Quick and a special prop in React, so a slot by that
  name collides with the host type the backends generate. `mosaic-std-controls`
  now depends on 0.2.0.

## [0.1.0] - Unreleased

- Added a schema-v1 package token palette with light/dark colors, type sizes,
  spacing, radii, surface, border, accent, and icon defaults.
- Added accessible `DisplayText`, `HeadingText`, `BodyText`, `CaptionText`, and
  `FoundationIcon` components composed only from Mosaic primitives.
- Added a themed `Surface` with a typed default authored-child mount. Included
  consumers expand it before emission and remain native-complete on all five
  native backends.
- Added package-expanded `native-complete` acceptance across all five native
  backends and consuming-package token-inheritance coverage.
