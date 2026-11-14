## [0.1.0] - UNRELEASED

### Added

- KILN04 stage 4 PR 2b-2b-ii: `kiln record collect --root <abs> --dir <abs>
  --results <abs> --out <abs>` replaces the stub. It assembles the shard's
  `records.jsonl` (header, then one record per action of the plan) and refuses,
  writing nothing, when it cannot say what the build did.
  - Takes the exclusive, non-blocking `flock` on the record directory first, then
    re-verifies `pre/` (`VerifyPre`), the checkout, the running Kiln and the pinned
    git (by the digest in the binding); runs Git only by the recorded path with
    `HOME` and the recorded `PATH` (`source.NewGitAt`).
  - Per action (`collect_actions.go`): every entry of `traces/<sha256(action)>/` is one
    the supervisor makes; a `.rerun` or `.overlap` marker, a sidecar without a trace
    (or the reverse), `trace-write-failed`, `strace-pipes-held-open`, a stray
    `record.json.tmp-*`, a line that is not the planned one, a trace that does not
    end in a newline or does not re-hash to the sidecar's `trace_sha256` and
    `trace_bytes`, a gap in `seq`, a `record.json` that is missing, undecodable,
    of another action, binding or sealed directory, or that counts other started
    steps than the sidecars, `record.failed` (which overrides `record.json`), and
    any `parse_ok: false` (a refused step is never "wrote nothing") make the
    action's write set UNKNOWN (`unknown_writes`, an empty raw record,
    `writes_complete: false`). `<step>.strace-stderr` is never opened. Text from the
    trace is made printable and at most 190 bytes before it reaches a record.
  - Across the shard: `seq` values distinct, below the final `traces/.seq`, as many
    as the counter says, no `.overlap`, nothing unexpected in `traces/`; otherwise
    every record is unknown (`serial-violated`).
  - `--results` (`kiln-legacy-results/1`, `record.ReadResults`/`DecodeResults`): a
    non-following, bounded, strict read; its status becomes the record's `status`,
    and its step counts must equal the traced ones and the plan's.
  - After the build: `VerifyTree` with the commit `prepare` recorded, `git fsck
    --connectivity-only --no-dangling` (`record.Fsck`) with the pinned git and its
    allowlist environment, and `Inventory` against the pre-build inventory. A plain
    git error, an `fsck` failure or an inventory error is not a crash: the header
    gets the `tracked-changed` void and one unexplained change (or path). The header
    counts `tracked_changed_unexplained` (changed tracked paths no record's
    `tracked_writes` names or contains) and `post_inventory_unexplained` (new paths
    no record's `writes` overlap).
  - `writes_complete` is derived with `recfmt.CompleteOf` from the classifier's own
    answer; a record that ran nothing gets an explicit `false`; a record the format
    cannot encode becomes an unknown write set; a shard over the format's size caps
    becomes a `records-too-large` shard with every record unknown. The shard is
    decoded with `recfmt.DecodeShard` before it is written, atomically (never
    replacing a file), to `--out`.
  - `record.TreeError.Paths`: every path that has a problem, raw and sorted (the
    printed list stays at 16).
  - `TestSandboxRecordCollect` (the `kiln-sandbox` job; registered in
    `ci/assert_sandbox_tests.py`): prepare, every step, collect and the merge over five
    actions (a clean hit, a failed step, a write the classifier cannot place, a step
    run twice, a tracked file changed after the build).
