---
category: Testing & coverage
---

# A test that asserts the pre-recording state inside the recorded tree blocks the recording from ever landing

VOS-028c3c lands as one PR in two commits. Commit A registers new experiments. A CI
recording then runs at commit A, and commit B commits the result. The results file binds
itself to the code that produced it by recording tree ids, and those ids covered whole crate
directories. Commit A added a test inside the probe crate asserting
`full_plan_missing(committed) == 600`. That held while the file was incomplete, and was
"to be deleted in commit B".

The recording (run 45417745745, 64 shards, join, `validate` exit 0) succeeded. It could
not land: commit B makes the assertion false. Deleting the test changes the probe's tree,
so the recording no longer matched the code. About 5 hours and dozens of runner-hours were
spent before anyone noticed.

The fix (VOS-D139, VOS-D140) narrowed the recorded ids to the inputs that determine recorded
values: each crate's `src/`, `Cargo.toml`, the lockfile and compile-time includes. Tests,
README and CHANGELOG are no longer covered. The test was removed before the re-record.

What to do differently:
- Before dispatching an expensive recording or snapshot that pins a tree, grep the pinned
  paths for anything that asserts the "before" state: counts of missing rows, "until commit
  B" comments, TODOs to delete later. Fix it before recording, not after.
- Pin the narrowest set of inputs that determine the recorded values. Pinning whole
  directories pins docs and tests too, and every later doc edit then invalidates the
  evidence.
