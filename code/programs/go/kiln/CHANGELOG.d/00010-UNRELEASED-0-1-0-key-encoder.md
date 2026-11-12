## [0.1.0] - UNRELEASED

### Added

- Program scaffold for Kiln (standard library only).
- `internal/key`: the canonical action-key encoder of KILN02 (key schema 1):
  `Preimage`, `Key`, `Hex`, `LegacyRecipe`, and strict validation that refuses
  (never repairs) malformed actions.
- Golden vectors at `code/specs/fixtures/kiln/key-vectors-v1.json`, with a
  loader that checks every valid vector and rejects every invalid one.
- CLI: `kiln version` and `kiln verify-vectors <path>`.
