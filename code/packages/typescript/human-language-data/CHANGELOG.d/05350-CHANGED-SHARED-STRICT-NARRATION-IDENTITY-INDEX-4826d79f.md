### Changed — shared strict narration identity index

- Expose one generated-hash API that proves exact narration language ownership
  before opening chapter bytes and returns stable per-language lesson identities.
- Reject unsafe or reserved identities, duplicate and case-fold-colliding lessons
  across chapters or languages, aggregates, nesting, symlinks, and non-regular
  owners at the shared boundary.
- Replace the modality-private narration projection so modality and future
  generated-owner gates enforce the same completeness and security contract.
