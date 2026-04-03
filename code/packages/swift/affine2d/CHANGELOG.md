# Changelog — affine2d (Swift)

## [0.1.0] - 2026-04-02

### Added
- `Affine2D` struct with six-element homogeneous matrix representation
- Factory methods: `identity`, `translate`, `rotate`, `rotateAround`, `scale`, `scaleUniform`, `skewX`, `skewY`
- `then(_:)` for composing two transforms (self first, then other)
- `applyToPoint(_:)` and `applyToVector(_:)` for transforming geometry
- `determinant`, `inverted`, `isIdentity`, `isTranslationOnly`, `toArray()`
- 21 unit tests covering all operations
