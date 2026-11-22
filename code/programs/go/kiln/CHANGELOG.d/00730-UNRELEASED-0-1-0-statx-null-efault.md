## [0.1.0] - UNRELEASED

### Added (KILN04 D45, `kiln:recorder` 12 to 13)

- A completed `statx` call whose `pathname` argument is literal `NULL`, whose
  `dirfd` passes `dirfdPath`, whose `flags` decode without `AT_EMPTY_PATH`
  (including numeric `0x1000` / `4096`), and whose result is `-1 EFAULT` now
  drops with no event instead of refusing with `path-arg`. This unblocks Rust
  `std`'s `statx` availability probe (`statx(AT_FDCWD<...>, NULL, ...)`).
- Kept refused: `statx(..., NULL, AT_EMPTY_PATH, ...)` (`path-arg`), malformed
  `flags` (`flags`), non-`EFAULT` outcomes such as `= 0` or `= -1 ENOENT`
  (`path-arg`), non-`NULL` non-string path pointers such as `0x1` (`path-arg`),
  bad/unannotated `dirfd`, and dying `statx` calls (`unresumed-call`).

### Changed

- `RecorderVersion` bumped 12 to 13 in `internal/legacy/traced.go` (the accept
  set grows, so every weak key changes).
- Added Decision D45 to `code/specs/KILN04-traced-read-sets.md`.
