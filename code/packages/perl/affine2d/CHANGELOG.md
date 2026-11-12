# Changelog

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

## [0.1.0] - 2026-04-02

### Added
- identity, translate, rotate, rotate_around, scale, scale_uniform, skew_x, skew_y
- compose, apply_to_point, apply_to_vector
- determinant, invert (undef if singular), is_identity, is_translation_only, to_array
