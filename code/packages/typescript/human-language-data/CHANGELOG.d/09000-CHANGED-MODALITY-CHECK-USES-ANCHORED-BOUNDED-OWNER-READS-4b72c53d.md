### Changed — modality check uses anchored, bounded owner reads

- Bound Markdown source reads for whole-corpus modality tests and the read-only
  `--check` path without changing the 30-second and 60-second test budgets.
- Read each direct-owner track as an exact file set beneath a pinned parent
  handle: `NtCreateFile` relative opens on Windows and `dir_fd`/`O_NOFOLLOW`
  on Unix. At most 16 reads run at once, and all started reads finish before a
  failure returns. A swapped parent junction cannot redirect an owner read.
- Keep the byte-for-byte generated-output comparison, canonical formatting,
  independently sourced lesson/narration identities, and reconstructed public
  manifest check. The synchronous reader and write/recovery flow remain.
- Focused Windows checks passed: 50 Python shard-boundary tests, 18 direct-owner
  TypeScript tests (one file-symlink privilege skip), and all 44 modality
  manifest tests. Full-package and CI validation remain separate gates.
