# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-04

### Added

- `analyzeMosaic(source: string): MosaicComponent` — full pipeline entry point
  (tokenize → parse → analyze in one call)
- `MosaicComponent`, `MosaicSlot`, `MosaicType`, `MosaicNode`, `MosaicProperty`,
  `MosaicValue`, `MosaicChild`, `MosaicImport` — complete typed IR
- Slot type resolution: `text`, `number`, `bool`, `image`, `node`, `color` primitives;
  `list<T>` parameterized lists; named component references
- Property value parsing: string literals, number+unit pairs (`16dp`, `50%`),
  `@slotRef` references, `#rrggbbaa` hex colors
- `when @flag { ... }` and `each @xs as item { ... }` child block analysis
- Import collection: automatically discovers non-primitive component references
  in the body and collects them into `MosaicComponent.imports`
- Grammar fix: `property_assignment` accepts `(NAME | KEYWORD)` so that Mosaic
  type keywords (`color`, `text`, `node`, `image`, `number`, `bool`) can appear
  as CSS-style property names (e.g., `color: #fff;`)
- Error throwing for undeclared slot references, unknown types, missing component
- 53 tests, 100% coverage
