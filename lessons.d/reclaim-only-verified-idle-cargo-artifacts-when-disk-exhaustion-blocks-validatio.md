---
category: Repo policy / workflow reminders
---

# Reclaim only verified idle Cargo artifacts when disk exhaustion blocks validation

The first native18 producer failed in rustc-LLVM with no space on device before
any test summary. Preserve compiler101/wrapper1 and report no semantic red.
Generic recursive deletion was automatically blocked by policy and did not run.
Use the safer official cargo clean command only for explicitly named, resolved,
non-reparse generated caches with consumed producers. Keep source worktrees,
logs and receipts outside those caches; leave the frozen Miri run/inputs/target
untouched. Cleanup removed the failed105MiB cache and an idle historical19GiB
cache, restoring space without weakening tests or requiring a new worktree.
