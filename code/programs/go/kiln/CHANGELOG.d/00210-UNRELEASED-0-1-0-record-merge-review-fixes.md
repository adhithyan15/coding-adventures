## [0.1.0] - UNRELEASED

### Changed

Fixes from the D13 adversarial review of the stage 4 merge engine
(`internal/recmerge`, `internal/recfmt`, KILN04 section 7.1). The strict
verdict, the only one that produces manifests and passes, is now stricter.

- Truncated executions are an unknown write set and void the run (M-1): a
  status other than `built` (even with every step run), `steps_run` below
  `steps_total`, and `daemon-left`. `daemon` and `daemon-left` are both daemon
  kinds for the peer rule.
- The shard header carries `tracked_changed_unexplained` (a required key); a
  non-zero count voids the run (M-2).
- An action that depended on another action's recorded writes (any reason from
  rules M5 to M14) voids the run (M-3, second-order order dependence).
- `Merge` also computes a report-only relaxed verdict
  (`Outcome.RelaxedEmittable`, `Result.RelaxedRunVoid`, `Outcome.Truncated`,
  `Outcome.OrderDependent`) in which the M-1 and M-3 classes cost only their
  action. It never produces a manifest, a pass or a ledger record.
- A recipe mismatch is an unknown write set that `writes_complete` never
  excuses (L-1); a step with `clean: false` is unknown, and `len(steps)` must
  equal `steps_run` (L-2).
- A global work budget (ancestor probes weighted by prefix length) and a
  4 GiB retained-text cap give run-void `size` (L-3); paths are limited to
  4,096 bytes and 128 segments; a record that fails validation stops the
  analysis.
- Cause text no longer depends on map iteration order (L-4).
- Rule M7 and the tainted check for `list:` entries use overlap, ancestors
  included (L-6). The wording of the daemon rule and its spec text no longer
  claim to cover port 0 or abstract binds (L-5).
- Decoding refuses invalid UTF-8, NUL and U+FFFD in paths, a BOM, padding or CR
  around a line, `writes_complete: null`, entries outside the manifest
  allowlist and statuses that are not lowercase words (L-7).
- `FuzzMerge` now varies steps, status, gate, refusal, the recipe digest,
  `daemon-left` and the new header count, and states the soundness invariants of
  the strict verdict separately from the rule oracle.
- Spec: section 7.1 gains M-1, M-2, M-3, the relaxed verdict and the resource
  bounds; section 14 names second-order order dependence, shard
  under-reporting and the report-only binary digests.

### Round 2 (D13 re-review, four LOW findings)

- The work meter now charges the owner lists of every visited node, the
  `HasPrefix` cost of descendant walks and every reason added, so many actions
  sharing many writes end as run-void `size` or finish quickly.
- The M-3 own-record rule is an allow-list (`intraActionKind`): `failed`,
  `precondition`, `network`, `host-read`, `state-write`, `writes-tracked`,
  `vcs-read`, `trace-visible`, `unscrubbed`, `trace-incomplete`, `daemon-left`
  and `manifest-*`. Everything else, including a daemon client and any unknown
  kind, is order dependence. The dead `unstable-path` check is gone.
- `null` is refused for every field of every record type, not only lists.
- The path part of every entry and the action name get the same checks as other
  paths (length, depth, NUL, U+FFFD).

### Security review (2 MEDIUM, 3 LOW, INFO)

- The reason-set trim is amortised (only when the set has doubled; each trim is
  charged `n log n`), and a record may carry at most 64 distinct reason kinds.
- Size caps hold before allocation: `DefaultLimits` is 64 MiB per line and
  1 GiB per file, a token pre-pass refuses over-long lists, strings and deep
  nesting before any struct is built, the binding, host facts and inventory have
  byte caps, `DecodeShard` stops at a retained-text cap, and `Merge` counts the
  host facts in its own cap.
- The void-cause sets are bounded: exact distinct count, smallest 64 as text.
- `null` and empty elements in string lists are refused; a shard whose artifact
  index is outside the plan stops the analysis, so its host facts never reach
  `Keys.Weak`.
- The meter saturates; host facts refuse NUL, control characters, U+FFFD,
  `..` in the paths and empty executor args; U+FFFD is refused in all free text;
  `Outcome.Action` documents that it must be sanitised before printing.
- New `FuzzDecodeRecord` target (decoded records are valid and a fixed point of
  encode and decode).
