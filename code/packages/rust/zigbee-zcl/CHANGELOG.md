# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Added

- `ZclAttributeReportSummary` plus `attribute_report_summary()` for compact
  parsed-report shape and D23 state-delta coverage diagnostics.

## [0.1.0] - 2026-05-06

### Added

- ZCL frame-control and frame parser/encoder primitives.
- Foundation read-attributes and on/off cluster command builders.
- Level and color-temperature command frame builders for light actuation.
- Typed attribute report parsing for common scalar/string data types.
- D23 capability and state-delta mapping for common smart-home clusters.
- Temperature and illuminance measurement cluster projections into normalized
  D23 sensor state deltas.
- Relative Humidity Measurement cluster projection into normalized D23 sensor
  state deltas.
