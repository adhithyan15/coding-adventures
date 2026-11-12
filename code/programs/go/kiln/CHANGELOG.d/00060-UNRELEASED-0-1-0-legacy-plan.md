## [0.1.0] - UNRELEASED

### Added

- `kiln legacy-plan` and `kiln explain` (KILN03, loop item 7c-1): `internal/legacy`
  loads a strict `kiln-legacy-graph/1` export, computes each package's closure
  (deps, dev_deps seeded at P, nested and enclosing packages for every member),
  own digests, reads (repo_inputs, globs, the boundary file, ancestor
  configuration) and a semantics-version-0 key, classifies packages
  (`cacheable`, `unpinned`, `uncacheable`) and writes a deterministic plan with
  a histogram. Tools, env and escape scans are not done yet: every package
  carries `m1-incomplete:tools-env` and `m1-incomplete:scans`.
- `source.Describer` (link, target text and executable bit from the Git index)
  and `source.Cleaner` (untracked, non-ignored files) on the Git and Folder
  adapters; the Git source refuses a dirty tree.
- `key.FilesDigest` and `key.ConsumedDigest`: section digests built by the key
  encoder itself.
- `code/kiln/legacy/pinned.json` and `never.json` (both empty), and
  `code/specs/fixtures/kiln/glob-cases-v1.json` with a test that the Source
  matcher satisfies every case.
