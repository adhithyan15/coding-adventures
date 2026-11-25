### Kiln: `kiln record-merge` joins the recording shards and writes the ledger

- New command `kiln record-merge` (KILN04 §7.1, stage 4 PR 3a). Every stage-4
  piece until now records and decides nothing by design — `kiln record collect`
  writes raw shard records — so no manifest and no pass could ever be produced.
  This is the job that joins them: B1 (`record.VerifyTree`), the three
  build-tool exports read once and hashed, `recfmt.DecodeShard` over every
  artifact, recomputed keys, `recmerge.Merge`, then the ledger.
- `internal/record/merge.go` — the job as a thin adapter over `recmerge.Merge`,
  which already implements binding B1–B6 and the cross-action rules M1–M15 and
  is fuzz-tested against a brute-force oracle. Writes manifests and then passes,
  and writes nothing at all when the verdict is void.
- `internal/record/mergekeys.go` — the production `recmerge.Keys` the merge
  engine's doc comment had been waiting for: one `*legacy.Traced` per distinct
  host-facts value, so there is one key algorithm in the repository and this
  only adapts it. A shard's own weak key is never trusted (B3).
- `record.ParsePartition` and `Partition.Paths` — the plan projected onto
  package paths, plus the name → path bridge the steps export needs for B6.
- `ledger.CheckAuthority` — lets a caller learn before it does expensive work
  whether its write would be refused; it answers exactly what the writes answer.
- Exit status is the report: 0 emitted, 1 job broken, 2 usage, 3 not permitted
  to write this ledger, 4 **run void** (a result, not a malfunction, so CI can
  tell "the recorder is not sound yet" from "the merge job is broken").
- The Kiln README's "`Ledger` and `Tools` are not built yet" was two-thirds
  wrong (the ledger is built and documented further down) and is corrected;
  `Tools` (M5) is the one interface genuinely absent.
- Not wired into CI yet: `kiln-record.yml` is stage-4 PR 3b. Nothing calls the
  command today, so it changes no build, no plan and no ledger.
- Tested: option refusals before any work, the unauthorized-write refusal, B1,
  shard-artifact refusals (symlink, missing file, duplicate index, one directory
  as two shards), the plan's name→path bridge, the key memo (one Traced per
  distinct facts), a refusal returned as an action verdict rather than an error,
  the CLI's exit codes, and the report's void output. An end-to-end EMIT test
  needs a real `prepare` shard, which only runs in a trusted linux/amd64 run; it
  belongs with PR 3b.
