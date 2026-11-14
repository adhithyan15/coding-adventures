## [0.1.0] - UNRELEASED

### Fixed (first review of `kiln record collect`)

- A package with an EMPTY recipe (a comment-only BUILD: built, 0 of 0, no
  directory) is vacuously complete instead of an unknown write set; one such
  package used to void every run. Any other fact (a trace directory, a started
  step, another total) still leaves it unknown. The merge accepts it and does not
  emit it.
- `CollectResult.PostProblems`: why each failed post-build check failed (the check,
  its error, the first unexplained paths), sanitised and bounded; `kiln record
  collect` prints it as `post-build problem:` lines. It is not written to the shard.
- A new inventory path is no longer explained by a write BENEATH it; only a write
  at the path or above it explains it.
- A `TreeError` raised before the disk was compared (`TreeError.Partial`) counts
  one more unexplained tracked change.
- `traces/.collected`: `collect` creates it (`O_EXCL`) before it reads anything,
  and `kiln record step` then refuses (125, nothing run, no sequence number, no
  marker). `--results` may not be inside the checkout and `--out` may not exist
  (both refused before the tombstone).
- Every git call has a 10 minute deadline and keeps at most 1 GiB of stdout (more
  is an error) and 1 MiB of stderr (truncated with a marker).
- `DecodeClassified` and `DecodeStepSidecar` refuse duplicate and case-variant keys
  (`legacy.PolicyDecode`).
- `writeAtomic` names its temporary file randomly (not by pid), removes it on every
  path and fsyncs the parent directory after the link.
- `checkSidecars` looks at no more than `len(plan)+1` step indices, so a directory
  of orphan trace names cannot make a million reasons.
- The end-to-end test uses a deterministic metadata write for action C
  (`chmod u+w <repo>`, no `|| true`), adds an empty-recipe action, and logs
  collect's report when it fails.
