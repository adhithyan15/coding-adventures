## [0.1.0] - UNRELEASED

### Added

- `internal/recfmt` (KILN04 stage 4, PR 1): the strict record formats shared by
  the recording shards and the merge job. `Binding`, `HostFacts`, `Inventory`,
  the shard header and the record line (the classifier's `RawRecord` plus the
  shard's per-action fields), with `Decode*` and canonical `Encode*`.
  Decoding refuses unknown, duplicate and case-variant keys, trailing data,
  missing keys, null lists, unsorted or repeated path lists and any path
  `key.ValidatePath` refuses; `ReadLines` caps a line at 256 MiB and a file at
  2 GiB without buffering past the cap. `StepsDigest` and `PartitionDigest`.
- `internal/recmerge` (KILN04 stage 4, PR 1): `Merge(Input) Result`, the pure
  merge engine. No I/O and no clock. Binding checks B1-B6 and rules M1-M15 of
  KILN04 section 7.1, three levels of "no" (action, shard-void, run-void), the
  per-(shard, action) record model, weak-key recomputation through a `Keys`
  interface, and emission of `ledger.Manifest` and `ledger.Record` values for
  the actions that survive. A `go test -fuzz FuzzMerge` target compares the
  emitted set with an independent O(n^2) oracle.
- KILN04 section 7.1 "Merge decisions (stage 4a)": gaps G1 (an unknown write
  set voids the run, `writes_complete` modelled as a switch), G3 (records are
  keyed by shard and action), G4 (weak keys are recomputed from host facts) and
  G11 (the daemon rule uses the reason's kind); section 7 rewritten to match.

### Changed

- Three one-line exports, no behaviour change: `legacy.PolicyDecode`,
  `traced.Sanitize` and `traced.CapReasons`.

### Notes

- Until the classifier reports `writes_complete` (stage 4, PR 4) every
  `trace-incomplete` reason is an unknown write set, so most real runs will
  emit nothing. That is by design; the report still says why.
- A record whose `steps_run` is below `steps_total` is an unknown write set
  whatever its status, which the plan's text only required for `built` actions.
