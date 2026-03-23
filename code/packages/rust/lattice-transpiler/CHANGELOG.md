# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- Initial implementation of the end-to-end Lattice-to-CSS transpiler
- `transpile_lattice(source: &str) -> Result<String, LatticeError>` — pretty-printed CSS (2-space indent)
- `transpile_lattice_minified(source: &str) -> Result<String, LatticeError>` — minified CSS
- `transpile_lattice_with_indent(source, indent, minified)` — full control over formatting
- Thin wiring layer: delegates to `coding-adventures-lattice-ast-to-css::transform_lattice_with_options`
- 20 integration tests covering the full pipeline from Lattice source to CSS text
