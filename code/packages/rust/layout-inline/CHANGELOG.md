# Changelog

## Unreleased

- Replaced whitespace/code-point fragmentation with host-neutral Unicode
  line-break and grapheme analysis, including inherited HTML direction.

## [0.1.0] — initial release

### Added
- Producer-neutral inline formatting for `LayoutNode` runs.
- Greedy word fragmentation, explicit line breaks, and `break-all` support.
- Baseline, top, middle, and bottom vertical alignment.
- Per-line semantic wrapper fragments for precise hit-testing and decoration.
- Atomic replaced-content callback so block layout remains the sizing owner.
