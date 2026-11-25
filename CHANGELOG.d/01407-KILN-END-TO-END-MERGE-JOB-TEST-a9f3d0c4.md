### Kiln: prove the merge job end to end against a real recorded shard

- `TestSandboxRecordCollect` (the required `kiln-sandbox` job) now runs
  `record.Merge` — the code behind `kiln record-merge` — over the shard it just
  recorded, instead of only feeding `recmerge.Merge` a fake `Keys` and a
  hand-built `Expected`. The job derives both itself: B1 from the fixture's
  commit, the graph/steps/plan digests from the exports `prepare` sealed, the
  keys from the real `legacy.Traced`.
- It asserts the two properties that make the job safe: B1 refuses a revision
  the tree is not, **before** any shard is read or keyed; and a void run writes
  nothing at all — zero manifests, zero passes, and an empty ledger directory
  (KILN04 §7, rule 3: all or nothing).
- This was the last unproven link between "shards record" and "the ledger has
  manifests". It is Linux-only, like the rest of the recorder's end-to-end
  tests, and runs in the `kiln-sandbox` CI job.
