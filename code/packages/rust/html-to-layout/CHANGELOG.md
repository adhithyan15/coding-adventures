# Changelog

## Unreleased

- Retain label association metadata in Layout IR for shared hit-region
  resolution.

- Project form controls into the reusable `layout-controls` contract with
  deterministic intrinsic sizes, CSS appearance, constraints, and state.

## [0.7.0]

- Project multiple image and linear/radial gradient layers, per-layer
  position/size/repeat/origin/clip, and elliptical corner radii into the
  reusable background contract.

## [0.6.0]

- Project computed opacity, affine transforms and origins, filters, one
  box/text shadow, blend modes, isolation, and uniform border radii into the
  reusable visual-effects and paint contracts.

## [0.5.0]

- Project decoded image dimensions, CSS `aspect-ratio`, and `object-fit` into
  the reusable replaced sizing contract.

- Project computed `box-decoration-break` plus inline margin, padding, and
  border values into the reusable fragmented inline-box contract.

- Project computed `float` and `clear` values into the reusable
  `layout-float` extension contract.
- Project CSS table layout, border spacing/collapse, caption side, vertical
  alignment, section kinds, and HTML row/column spans into `ext["table"]`.

## Unreleased

- Expand one-to-four-value border width/color/style shorthands and retain
  per-side `none`, `hidden`, `solid`, `dashed`, `dotted`, and `double` styles
  in shared paint metadata.

- Project computed position, inset, z-index, and overflow values into the
  reusable `layout-positioned` extension contract.

## Unreleased

### Added

- Compute CSS grid container and item values into a typed `ext["grid"]`
  boundary, including track lists, named areas, implicit tracks, placement
  shorthands, spans, gaps, ordering, and two-axis alignment.
- Compute CSS flex container/item longhands and shorthands into the typed,
  layout-engine-independent `flex` extension contract.
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
