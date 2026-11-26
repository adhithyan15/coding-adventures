## [0.1.0] - UNRELEASED

### Added (the record workflow: record on main pushes, merge, keep the ledger in the Actions cache; disarmed)

- `.github/workflows/kiln-record.yml` (stage 4 PR 3b): on `push` to `main` and `workflow_dispatch`,
  one matrix leg per shard on `ubuntu-26.04` (prepare with `--exec-head`, the traced build, collect,
  the census and `census_not_clean.py --strict-exec-head`), then one merge job running
  `kiln record-merge --scope shared` over the shard artifacts and saving the ledger with
  `actions/cache/save` under `kiln-ledger-v1-<run id>-<attempt>`, restoring the newest
  `kiln-ledger-v1-` entry first.
- Trust: the cache is saved only by the merge job, only when the run is a push or dispatch on the
  default branch of this repository and `record-merge` exited 0 and added entries. A shard that is not
  clean uploads nothing, a run with a missing shard is void and merges nothing, a void verdict (exit 4)
  saves nothing. Runs are serialised (`cancel-in-progress: false`). `contents: read` only.
- The shard files (which may carry host paths) are artifacts with `retention-days: 1`; the job summary
  holds counts only.
- `ci/record_ledger_stats.py`: counts the entries and bytes of a ledger directory (`records=N bytes=B`),
  for the summary and for dropping a restored ledger that is over 64 MiB or malformed (`record-merge`
  does not prune). Tests: `code/scripts/tests/test_kiln_record_ledger_stats.py`.
- DISARMED: both jobs start with a literal `false &&` because the language selector (Go only) does not
  exist yet; see KILN04 7.2. The selector PR removes it.
- KILN04 gains section 7.2 recording the owner's decisions: the stored record is the existing ledger
  only (names and hashes), shard artifacts live one day, the Actions cache is the ledger's home for the
  shadow phase, and a rollback-proof store plus explicit approval come before any reuse.
- `kiln:recorder` is unchanged.
