# Changelog

## 0.1.0 - 2026-08-26

### Added

- Add the generic DT13 trie with exact lookup, explicit nullable-value
  presence, ordered prefix enumeration, longest-prefix matching, deletion
  pruning, and invariant validation.
- Define Unicode-scalar traversal and scalar-numeric ordering without implicit
  normalization or locale collation.
- Keep every operation stack-safe for long keys and redact keys and values from
  the structural string representation.
- Add strict cross-platform build gates, 90% minimum coverage, and an empty
  authority profile.
