## [0.1.0] - UNRELEASED

### Added (KILN04 decision D48; spec only, no production change)

- `code/specs/KILN04-traced-read-sets.md`: decision D48, "exec heads recorded at
  exec time". It fixes how a binary the step built under the fresh tmpfs and
  deleted (Go's `pkg.test`, today `trace-incomplete:exec-unreadable:under-tmp`) gets
  its head read: a second seccomp filter with a user-notification listener on
  `execve`/`execveat`, the listener handed over a dedicated unix socketpair to a
  supervisor in `kiln record step`, a pin-continue-confirm protocol per
  notification, the `<trace>.exechead` JSONL sidecar and its digest in the step
  sidecar, classifier consumption by per-tid pairing plus an exact path match,
  `exechead-*` census sub-codes, the fail-closed cases, the security preconditions
  with the probe evidence, the known v1 limit (`gone`), the cost, the
  `RecorderVersion` bump (16 to 17 when consumption lands) and the follow-up PRs.
  Classifier consumption is gated on PR #19902's valid-map `PR_SET_MM_MAP` probe.
- `code/specs/KILN04-exec-head-spike.md`: Status says the design is adopted as D48,
  and the markers the two CI runs (45342132127, 45346779343) answered are promoted
  from INFERRED to VERIFIED; the open items stay open (section 7).
