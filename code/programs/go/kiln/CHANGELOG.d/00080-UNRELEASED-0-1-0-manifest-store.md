## [0.1.0] - UNRELEASED

### Added

- Manifest store (KILN04 stage 2), in `internal/ledger` (`manifest.go`,
  `manifest_store.go`): a store-only record kind in a `manifests/`
  subdirectory of the ledger directory. Canonical JSON lines
  (`kind`, `id`, `weak`, `target`, `os`, `arch`, `entries`, `revision`, `run`,
  `at`); unknown fields refused, each line must equal its canonical
  re-encoding, and `id` (SHA-256 of the entries list) is verified on read.
  Entries are sorted, unique and allowlisted to `pkg:`, `path:` and `list:`
  (KILN02 section 3.5 paths; `list:.` is the root).
- Caps: 8,192 entries, 512 bytes per name and 1 MiB per line per record
  (checked at emit, with `ErrManifestTooLarge`, and at read), 64 MiB per
  segment and 256 MiB per store. Reader policy: an over-cap record or segment
  is dropped and reported; structural corruption makes the whole store
  unusable.
- `Record` writes immutable segments atomically through the pass ledger's
  `Authority` gate (no restamping). Retention keeps the newest 4 distinct ids
  per weak key by `(run, at)` and drops records older than 30 days relative to
  an injected `now`. `MergeManifests` merges snapshot and delta
  deterministically (oldest first within a weak key, so re-merging is
  byte-stable). `ManifestSet.Manifests(weak)` is the lookup for stage 5.
- Tests prove the pass-ledger reader ignores `manifests/` and refuses a
  manifest segment placed among pass segments (and the reverse).
- KILN04 section 8.1 records the implementation decisions.
