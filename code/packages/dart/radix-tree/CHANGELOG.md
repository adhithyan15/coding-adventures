# Changelog

## 0.1.0 - 2026-11-21

### Added

- Add pure Dart DT14 radix tree (compressed trie) with Unicode-scalar edge
  compression, iterative traversal, edge re-compression, and payload-blind diagnostics.
- Provide full API surface: `RadixEntry`, `count`, `size`, `isEmpty`, `nodeCount`, `edgeCount`,
  `insert`, `put`, `search`, `get`, `containsKey`, `operator []`, `operator []=`, `delete`,
  `startsWith`, `wordsWithPrefix`, `keysWithPrefix`, `allWords`, `keys`, `entries`, `toMap`,
  `longestPrefixMatch`, `longestPrefixOf`, and `isValid`.
- Enforce the 5-Class Radix-Tree Security Resilience Showcase (deep-key stack safety,
  surrogate-tearing defense, merge-back re-compression, nullable value disambiguation,
  and payload-blind confidentiality).
- Meet all cross-platform build gates, empty authority profile, and >= 95% test coverage.
