# Changelog — @coding-adventures/layout-text-measure-estimated

## 0.1.0 — 2026-04-05

Initial release.

### Added

- `createEstimatedMeasurer(opts?)` — factory function returning a `TextMeasurer`
- `EstimatedMeasurerOptions` type with `avgCharWidthMultiplier` field (default 0.6)
- Single-line measurement: `width = length × font.size × multiplier`
- Multi-line measurement: `lineCount = ceil(length / charsPerLine)`
- Handles empty strings, zero-size fonts, very narrow maxWidth (clamped to 1 char/line)
