# Changelog

## Unreleased

### Added

- Add context-aware `%`, `em`, and `rem` computation, `auto` sizing/margins,
  min/max constraints, border-box sizing, per-side border values, text
  alignment, and white-space projection into reusable layout and paint seams.
- Add element style declarations, inherited custom properties and `var()`
  resolution, edge shorthands/longhands, attribute and structural selectors,
  viewport-aware media evaluation, and transport-neutral `@import` metadata.
- Add grammar-validated author stylesheets, selector specificity/source-order
  cascade, inherited computed styles, media-independent style contexts, and a
  reusable layout boundary for display, color, backgrounds, typography,
  decoration, dimensions, margin, and padding.

## [0.4.0] - 2026-08-27

### Added

- Add a narrow visited-link resolver seam that selects final link presentation
  without coupling HTML layout to browser history or storage.
- Add Mosaic-era purple visited links and inherited link underlines through
  reusable Layout IR text decoration.

## [0.3.0] - 2026-08-27

### Added

- Preformatted browser nodes now project `whiteSpace: pre` through the shared
  block extension contract for reusable inline formatting.

## [0.2.0] - 2026-07-29

### Changed

- Inline text now uses content-wrapping width hints so `layout-block` can place
  text, links, and trailing punctuation on the same line.
- The parser-to-positioned-layout acceptance test now verifies horizontal link
  geometry as well as retained navigation metadata.

## [0.1.0] - 2026-07-29

### Added

- Browser render-tree to shared Layout IR conversion.
- Mosaic-era default typography, colors, spacing, and page background.
- Retention of HTML roles, display classes, link targets, language, direction,
  and image metadata in the Layout IR extension bag.
- An executable parser-to-positioned-layout acceptance test.
