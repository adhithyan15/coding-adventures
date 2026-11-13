---
category: Security boundaries
---

# Parallel package gates that share a fixed-name scratch directory under CARGO_TARGET_TMPDIR delete each other's traces, and a missing trace must fail closed

PR #19314 (`rust/gf256` strict runtime jail) passed locally and failed only on
CI: `RCG009: negative control io-uring / bpf / env-leak NOT detected`, with
`full(enf=false obs=false) ... enforce-only=true`. The cause was not gf256.
Every Rust adopter passes `CARGO_TARGET_TMPDIR/capability-runtime` as
`evidence_dir`, which is one directory per workspace, and the gate kept its
scratch directory at a fixed name beneath it and `remove_dir_all`'d it at the
end. The build tool runs packages in parallel, so `ct-compare`'s gate finishing
deleted the directory `gf256`'s gate was still writing `strace -o` traces into.
Every observed run after that had no trace (enforce-only runs, which use no
strace, still passed), and the controls still to run at that moment failed.
Locally nothing else was running, so it passed.

Two things to do differently:

- Any directory a test harness creates and later deletes must be private to
  the process (`<name>-<pid>`), never a fixed name under a path that other
  packages' tests share. Reproduce this class of bug by starting two copies of
  the binary half a run apart.
- An observer that produced nothing must fail the gate. Treating an absent or
  empty trace as an empty string reads "saw nothing" as "no violations", which
  is fail-open; the harness now reports `RCG013` for any observed run without
  a trace, in strict and non-strict mode alike.
