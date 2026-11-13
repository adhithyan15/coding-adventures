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
