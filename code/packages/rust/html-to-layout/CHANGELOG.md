# Changelog

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
