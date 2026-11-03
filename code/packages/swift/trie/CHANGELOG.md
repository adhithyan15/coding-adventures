# Changelog — Trie (Swift)

## [Unreleased]

### Fixed

- `BUILD_windows`: `where swift` now discards its output with cmd's `>nul
  2>nul` instead of `>/dev/null`. Under `cmd /C` the `/dev/null` redirect
  always failed, so the line always fell through to its "Swift not available"
  echo and the Swift step never ran on Windows, even on runners with Swift
  installed.

## 0.1.0 — 2026-07-11

### Added

- Initial release: pure-Swift port of the `trie` reference package.
- `Trie<Value>` with `insert`, `search`, `containsKey`, `delete`, `startsWith`,
  `wordsWithPrefix`, `allWords`, `keys`, `longestPrefixMatch`, plus `count`,
  `isEmpty`, `isValid()`, and a `CustomStringConvertible` description.
- Keys on `Unicode.Scalar` (matching the reference's `char`), so combining and
  precomposed forms stay distinct; enumerations are scalar-sorted.
- Value-type semantics (assigning a trie makes an independent copy).
- 11 XCTest cases: exact search vs prefix, size-stable overwrite, lexicographic
  prefix enumeration, leaf/shared-prefix deletion, delete-nonexistent, longest-
  prefix-match, Unicode-scalar + empty-string keys, sorted keys/allWords,
  description, and a value-semantics copy check.
