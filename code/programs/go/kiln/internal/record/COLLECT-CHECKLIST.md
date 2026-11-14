# `kiln record collect` (KILN04 stage 4, PR 2b-2b-ii): obligation checklist

Working file for the review. The coordinator removes it before the push.
Spec line numbers are those of `code/specs/KILN04-traced-read-sets.md` at the base of this
branch (`70d696cee5c`); the spec has since been edited (section "As implemented by `collect`").
All test names are in `internal/record` unless a file is named. `collect_unix_test.go`,
`collect_verdict_unix_test.go`, `collect_post_unix_test.go`, `collect_helpers_unix_test.go` and
`collect_fixture_unix_test.go` are the collect tests.

| # | Spec item (section, line) | Implemented in | Tested by | Status |
|---|---|---|---|---|
| 1 | Exclusive, non-blocking `flock` on the record directory; refuse if a step or orphaned supervisor holds it (P12 obligations, l.1291) | `collect.go` `Collect` step 1, taken before anything of the directory is read | `TestCollectRefusesWhileAStepHoldsTheDirectory`, `TestCollectLockIsExclusive`, `TestCollectTakesTheLockBeforeReadingAnything` | done |
| 2 | Re-verify `pre/` against `SEALED` and the binding id before using any file (P11 l.1144, P4 l.940) | `Collect` step 2 (`VerifyPre`), then root, kiln, git | `TestCollectRefusesATamperedPre`, `TestCollectRefusesAnotherRootOrBinary` | done |
| 3 | Only the recorded git path and recorded PATH, never looked up (P11 l.1134) | `Collect`: `source.NewGitAt(root, tools.Git, {HOME, PATH})`; git's SHA-256 must be `binding.git_sha256` | `TestCollectUsesOnlyThePinnedGit`, `TestCollectRefusesAToolsFileWithoutHome`, `TestCollectFsckRunsPinned` | done |
| 4 | Any plain git error from `VerifyTree` voids the shard like `tracked-changed`; pass the commit `prepare` recorded (l.762) | `collect_post.go` `postBuild` | `TestCollectPostBuildSeams`, `TestCollectPostBuildTreeErrors/moved_head` | done (+ stricter count, divergence 1) |
| 5 | An `Inventory` error after the build voids the shard; no crash before the records are written (l.765) | `postBuild` | `TestCollectPostBuildSeams/an_inventory_that_cannot_be_taken`, `TestCollectATrackedFileReplacedByADirectory` | done (+ stricter count, divergence 1) |
| 6 | Post-build `git fsck --connectivity-only`, pinned git, allowlist env (l.776) | `collect.go` `Fsck` | `TestCollectFsckFailure` (real git, a blob removed), `TestCollectFsckRunsPinned` (arguments and environment) | done |
| 7 | Keep `Result.Commit` from the pre-build `VerifyTree` (= `binding.revision`), pass it as `expectedCommit` (l.781) | `postBuild` | `TestCollectPostBuildSeams/the_checks_are_handed_the_pinned_commit`, `.../a_VerifyTree_that_vouches_for_another_commit` | done |
| 8 | `TreeError.Paths`: the full list of problem paths, for the header count | `record.go`, `verify.go` (`parseIndex`) | `treeerror_paths_test.go` | done |
| 9 | Header counts `tracked_changed_unexplained`, `post_inventory_unexplained` (l.2675-2679; M2b, M3) | `collect_post.go` | `TestCollectPostBuildTreeErrors`, `TestCollectHeaderCounts`, `TestUnexplainedPathsAndCoverage` | done |
| 10 | Shard writer derives `writes_complete` with `recfmt.CompleteOf`, `EncodeRecord` calls `CheckComplete`; ran-nothing records get an explicit `false` (l.814) | `collect_actions.go` `assembleAction`, `collect.go` `encodeShard` | `TestCollectWritesCompleteIsTheClassifiers`, `TestCollectRanNothingIsExplicitFalse`, `TestCollectAnUnencodableRecordIsUnknown` | done |
| 11 | `record.failed` overrides `record.json`; both present is a failure; missing for an action that ran is `unknown_writes` (l.1312, l.1390) | `readVerdict` | `TestCollectVerdictTable` | done |
| 12 | Undecodable `record.json`, or a stray `record.json.tmp-*`, is unknown, never "no record" | `readVerdict`, `scanAction` | `TestCollectVerdictTable` | done |
| 13 | `record.json`'s `binding_id`, `sealed_sha256` equal the sealed directory's; `action` is the action | `readVerdict` | `TestCollectVerdictTable` | done |
| 14 | `record.json` read bounded and no-follow with the supervisor's reader | `readVerdict` uses `readRecordBounded` | `TestCollectVerdictTable` (over the bound, symlink, directory) | done |
| 15 | `steps_run` of a `record.json` equals the number of started sidecars | `readVerdict` | `TestCollectVerdictTable`, `TestCollectAVerdictWithNoStepVoidsTheShard` | done |
| 16 | `sandbox-refused` in `parse_reasons` is never "wrote nothing" (l.1315) | `readVerdict` (any `parse_ok` false) | `TestCollectVerdictTable/sandbox-refused_is_not_an_empty_write_set` | done |
| 17 | Every trace ends in a newline; re-hash; digest and size equal the sidecar's (l.1301) | `checkTrace` | `TestCollectTraceChecks` | done |
| 18 | Compare the init's traced `--sha256` and the root's `--config-sha256` with `binding.sandbox_config_sha256` (l.1304) | **`step`** (`parseStep`, P13: the parse half); `collect` relies on `parse_ok` | existing `classify_step_unix_test.go`, `TestCollectVerdictTable/a_trace_that_did_not_parse` | done by `step`; divergence 2 (open question) |
| 19 | `.rerun`; a sidecar without a trace or the reverse; `trace-write-failed`; `strace-pipes-held-open` are unknown (l.1309) | `scanAction`, `checkSidecars` | `TestCollectMarkersAndOrphans`, `TestCollectSidecarChecks`, `TestCollectTraceChecks` | done |
| 20 | `seq` globally distinct, below the final `.seq`, consecutive within an action (l.1312, l.1333) | `checkSidecars`, `globalSeqCheck` | `TestCollectSeqChecks` | done (+ count equals counter, divergence 3) |
| 21 | Any `.overlap` marker voids the shard (l.1312, l.1333) | `scanAction`, `globalSeqCheck` | `TestCollectOverlapVoidsEveryRecord`, `TestCollectSeqChecks/an_overlap_marker` | done |
| 22 | `steps_run` equals the started sidecars; traced step count equals `kiln-legacy-results` (l.1312; section 7 l.2603) | `assembleAction` | `TestCollectResultsCrossChecks` | done |
| 23 | `--results` consumed: strict decode, status becomes the record's status (l.188, 620, 861) | `results.go` | `results_test.go`, `TestCollectRefusesAnUnusableResults` | done |
| 24 | Untrusted step outputs: record.failed text sanitised and bounded; `.strace-stderr` never read (l.1250) | `reasonText`; `scanAction` | `TestCollectUnknownTextIsBounded`, `TestCollectMarkersAndOrphans/strace-stderr_is_never_read`, `TestReasonTextIsPrintableAndBounded` | done |
| 25 | Shard written atomically to `--out`; decodes strictly with `recfmt`; accepted by `recmerge` | `collect.go` `encodeShard`, `selfCheck`, `writeAtomic` | `TestCollectShardDecodesAndMerges`, `TestCollectRefusesOutPlacement`, `TestCollectAShardOverTheCapsIsReplacedByAVoidOne`, `TestCollectSelfCheck*` | done |
| 26 | CLI `kiln record collect --root --dir --results --out` (l.861) | `recordcmd.go` `runRecordCollect` | `recordcmd_collect_test.go` | done |
| 27 | Full end to end, five actions A to E | `sandbox_record_collect_linux_amd64_test.go` | `TestSandboxRecordCollect` (registered in `ci/assert_sandbox_tests.py`; the contract test `code/scripts/tests/test_assert_kiln_sandbox_tests.py` passes) | written, NOT RUN (Linux only) |
| 28 | Spec and CHANGELOG | `KILN04-traced-read-sets.md`, `CHANGELOG.d/00340-...-record-collect.md` | n/a | done |

## Not in this PR (2b-2c)

Gates and toolchain probes (`gate` stays empty in every record), the in-sandbox socket probe, the
`daemon-left` real-strace fixture. The header tokens `survivor` and `state-preloaded` are not
produced by `collect` (nothing in 2b-2b-ii detects them).

## Ambiguities and decisions

See the report. In short: (a) `--out` is `records.jsonl` (the spec never says), (b) void shard
exits 0, (c) the stricter counts on git, fsck and inventory failures, (d) the G7 digests stay with
`step`, (e) an action `built` with zero steps is unknown, (f) the `traces/` entry and sidecar-count
checks are stricter than the text.
