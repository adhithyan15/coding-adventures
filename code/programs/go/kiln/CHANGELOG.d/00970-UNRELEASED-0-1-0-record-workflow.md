## [0.1.0] - UNRELEASED

### Added (the record workflow plan, and a ledger counter for it)

- KILN04 section 7.2 records the project owner's four decisions (the stored record is the existing
  ledger only, names and hashes; shard artifacts live one day; the Actions cache is the ledger's home
  for the shadow phase, saved only by the trusted merge job; a rollback-proof store and explicit
  approval come before any reuse), the cache design, and the order of work: build-tool
  language-filtered plan, Kiln selector, the workflow dispatch-only, then the push trigger.
- Findings recorded there: no Go-only selector exists; `record-merge` does not apply the high-water
  mark (needed, with pruning and a durable store, before real reuse); the 64 MiB drop is shadow-only.
- `ci/record_ledger_stats.py`: counts the entries and bytes of a ledger directory (`records=N bytes=B`),
  naming no path or content, refusing symlinks and misshapen directories. Tests:
  `code/scripts/tests/test_kiln_record_ledger_stats.py`.
- No workflow lands in this change. `kiln:recorder` is unchanged.
