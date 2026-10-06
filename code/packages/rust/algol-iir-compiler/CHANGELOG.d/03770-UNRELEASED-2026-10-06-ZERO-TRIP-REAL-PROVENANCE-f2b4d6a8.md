## [0.377.0] - UNRELEASED

### Added
- Preserve runtime-real formatter provenance for unrelated caller-frame locals
  across statically proven zero-trip `step` and `while` loop elements. The
  controlled variable and storage promoted through capture or call-by-name
  remain conservative.
