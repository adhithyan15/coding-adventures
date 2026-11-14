# `kiln record collect` (KILN04 stage 4, PR 2b-2b-ii): obligation checklist

Working file for the review. The coordinator removes it before the push.
Line numbers are those of `code/specs/KILN04-traced-read-sets.md` at the base of this
branch (`70d696cee5c`). "Test" names are in `collect_unix_test.go` unless another file is named.

| # | Spec item (section, line) | Implemented in | Tested by |
|---|---|---|---|
| 1 | Exclusive, non-blocking `flock` on the record directory; refuse when a step or an orphaned supervisor holds it (P12 obligations, l.1291) | `collect.go` `Collect` step 1 (`LockDir(dir, true)`), before anything of the directory is read | `TestCollectRefusesWhileAStepHoldsTheDirectory`, `TestCollectLockIsExclusive` |
| 2 | Re-verify `pre/` against `SEALED` and the binding id before using any file of it (P11, l.1144; P4, l.940) | `Collect` step 2 (`VerifyPre`), then root, kiln, git binding checks | `TestCollectRefusesATamperedPre`, `TestCollectRefusesAnotherRootOrBinary` |
| 3 | Use only the recorded git path and the recorded PATH, never look a program up on PATH (P11, l.1134) | `Collect` builds `source.NewGitAt(root, tools.Git, {HOME, PATH})`; the git file's SHA-256 must equal `binding.git_sha256` | `TestCollectUsesOnlyThePinnedGit` |
| 4 | Treat any plain git error from `VerifyTree` (not only `*TreeError`) as a void like `tracked-changed`; pass the commit `prepare` recorded (l.762) | `collect_post.go` `postVerifyTree` | `TestCollectPostBuildTreeErrors` (plain error, TreeError, moved HEAD) |
| 5 | An `Inventory` error after the build voids the shard and must not crash before the records are written (l.765) | `collect_post.go` `postInventory` | `TestCollectInventoryError` |
| 6 | Post-build `git fsck --connectivity-only` with the pinned git and the allowlist environment (l.776) | `collect_post.go` `postFsck` | `TestCollectFsckFailure`, `TestCollectFsckRunsPinned` |
| 7 | Keep `Result.Commit` from the pre-build `VerifyTree` (= `binding.revision`, P10) and pass it as `expectedCommit` to the post-build call (l.781) | `postVerifyTree` passes `binding.Revision`; the returned commit is compared | `TestCollectPostBuildTreeErrors/moved-head` |
| 8 | `TreeError.Paths`: the FULL list of problem paths (spec: header count `tracked_changed_unexplained`, l.2675) | `record.go` (`TreeError.Paths`), `verify.go` (`parseIndex`) | `TestTreeErrorPathsAreComplete` (`verify_test.go`) |
| 9 | Header counts `tracked_changed_unexplained` and `post_inventory_unexplained` (l.2675, merge M2b and M3) | `collect_post.go` `countUnexplained*` | `TestCollectHeaderCounts` |
| 10 | Shard writer derives `writes_complete` with `recfmt.CompleteOf` from the raw record and calls `CheckComplete` (`EncodeRecord` does); a record that ran nothing gets an explicit `false` (l.814) | `collect_actions.go` `finishRecord`, `collect.go` `encodeRecords` | `TestCollectWritesCompleteIsTheClassifiers`, `TestCollectRanNothingIsExplicitFalse` |
| 11 | `record.failed` overrides `record.json`; both present is a failure; a missing `record.json` for an action that ran a step is `unknown_writes` (P13, l.1390, l.1307) | `collect_actions.go` `readVerdict` | `TestCollectVerdictTable` |
| 12 | `record.json` that does not decode, or a stray `record.json.tmp-*`, is an unknown write set, never "no record" (l.1309) | `readVerdict`, `scanAction` | `TestCollectVerdictTable` |
| 13 | `record.json`'s `binding_id` and `sealed_sha256` equal the sealed directory's; `action` is the action (l.1307) | `readVerdict` | `TestCollectVerdictTable` |
| 14 | Read `record.json` bounded and no-follow; reuse the supervisor's reader (`readRecordBounded`) | `readVerdict` | `TestCollectVerdictTable/oversize` |
| 15 | `steps_run` of a `record.json` equals the number of started sidecars (l.1307) | `readVerdict` | `TestCollectVerdictTable` |
| 16 | A refused step (`sandbox-refused` in `parse_reasons`) is never read as "wrote nothing": any `parse_ok` false is an unknown write set (l.1310) | `readVerdict` | `TestCollectParseFailureIsUnknownWrites` |
| 17 | Every trace ends in a newline; re-hash it; digest and size equal the sidecar's `trace_sha256` and `trace_bytes` (l.1294) | `collect_actions.go` `checkTrace` | `TestCollectTraceChecks` |
| 18 | Compare the init's traced `--sha256` and the root's `--config-sha256` with `binding.sandbox_config_sha256` (l.1297). **Split by P13: the parse half is `step`'s** (`parseStep`, classify_step.go) and a mismatch is `parse_ok` false. `collect` classifies nothing and parses no trace, so it applies this through item 16 | (no new code in `collect`; see the open question in the report) | existing `classify_step_unix_test.go`; `TestCollectParseFailureIsUnknownWrites` |
| 19 | `.rerun` marker, a sidecar without a trace (or the reverse), `trace-write-failed`, `strace-pipes-held-open` are unknown write sets (l.1302) | `scanAction`, `checkSidecars` | `TestCollectMarkersAndOrphans` |
| 20 | `seq` values globally distinct, each below the final `traces/.seq`, consecutive in step order within each action (l.1305, l.1333) | `collect_actions.go` `checkSeq`, `collect.go` `checkGlobalSeq` | `TestCollectSeqChecks` |
| 21 | Any `.overlap` marker voids the shard (every record `unknown_writes`, `serial-violated`) (l.1305, l.1333) | `scanAction` (flag), `Collect` (marks every record) | `TestCollectOverlapVoidsEveryRecord` |
| 22 | `steps_run` equals the started sidecars; the traced step count equals `kiln-legacy-results` (l.1307; section 7 unknown write sets, l.2603) | `collect_actions.go` `crossCheckResults` | `TestCollectResultsCrossChecks` |
| 23 | `--results` (`-emit-results`) is consumed: schema, strict decode, status becomes the record's `status` (l.188, l.620, l.861) | `results.go`, `collect_actions.go` | `results_test.go`, `TestCollectResultsRefusals` |
| 24 | Untrusted step outputs: `record.failed`/`parse_reasons` text is sanitised and bounded before it reaches the shard; `<step>.strace-stderr` is opaque and never read or uploaded (l.1250) | `reasonText`; `scanAction` ignores `.strace-stderr` | `TestCollectNeverReadsStraceStderr`, `TestCollectUnknownTextIsBounded` |
| 25 | The shard is written atomically to `--out` (whole or absent, never replacing), decodes strictly with `recfmt` and is accepted by `recmerge` (l.2b-2b-ii in the shard writer item, l.814) | `collect.go` `writeShard` (self-check with `recfmt.DecodeShard`), `writeAtomic` | `TestCollectShardDecodesAndMerges`, `TestCollectRefusesOutPlacement` |
| 26 | CLI line `kiln record collect --root <abs> --dir <abs> --results <abs> --out <abs>` (l.861) | `recordcmd.go` `runRecordCollect` | `recordcmd_collect_test.go` |
| 27 | Full end to end (prepare, every step, collect, recmerge), five actions A to E | `sandbox_record_collect_linux_amd64_test.go` | `TestSandboxRecordCollect` (Linux/amd64, `kiln-sandbox` job; registered in `ci/assert_sandbox_tests.py`) |
| 28 | Spec and CHANGELOG updated | `KILN04-traced-read-sets.md`, `CHANGELOG.d/` | n/a |

## Not in this PR (2b-2c, tagged so in the spec)

Gates and toolchain probes (`Gate` stays empty in every record), the in-sandbox socket probe and the
`daemon-left` real-strace fixture. The header tokens `survivor` and `state-preloaded` are not produced
by `collect` (nothing in 2b-2b-ii detects them).

## Ambiguities and decisions (also in the report)

See the final report of the PR; each is marked "Decided" or "Open" in the spec edit.
