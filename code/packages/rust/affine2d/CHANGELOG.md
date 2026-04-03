# Changelog

## [0.1.0] - 2026-04-02

### Added
- `Affine2D` struct with 6-float [a,b,c,d,e,f] representation
- Factory functions: identity, translate, rotate, rotate_around, scale, scale_uniform, skew_x, skew_y
- Operations: multiply, apply_to_point, apply_to_vector, determinant, invert, is_identity, is_translation_only, to_array
- Full test suite covering composition, inversion, rotation, and degenerate cases
