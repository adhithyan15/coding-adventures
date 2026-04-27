# Changelog

All notable changes to `text-interfaces`.

## [0.1.0] — initial release

### Added
- `FontQuery`, `FontWeight`, `FontStyle`, `FontStretch` — the abstract font request types.
- `FontResolver` trait with associated `Handle` type; `FontResolutionError` enum (EmptyQuery, NoFamilyFound, InvalidWeight, LoadFailed).
- `FontMetrics` trait — font-global getters (units_per_em, ascent, descent, line_gap, x_height, cap_height, family_name) with associated `Handle` type. Infallible on valid handles.
- `TextShaper` trait — shape codepoints into positioned glyph runs. `ShapeOptions` for script / language / direction / features. `ShapedRun` / `Glyph` output types. `ShapingError` enum (UnsupportedScript, UnsupportedDirection, ShapingFailed).
- `measure()` — thin convenience function that wraps a shaper + metrics pair and returns a `MeasureResult` for a single line.
- Comprehensive unit tests covering all constructors, error display, and a synthetic in-memory shaper that exercises the generic `measure()` function.

### Design
- All traits are generic over the backend's handle type via an associated type. The Rust type system enforces the TXT00 font-binding invariant at compile time.
- Zero external dependencies — the crate is pure types and traits.
