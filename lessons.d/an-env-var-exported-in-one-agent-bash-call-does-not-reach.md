---
category: Rust
---

# An env var exported in one agent Bash call does not reach the next cargo run, so set build-size knobs on every cargo command

**What went wrong.** LANG83 PR4 ran with about 1.5 GB of free disk, shared
with two other agents. The instructions were to build with
`CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0`. The first Bash call ran
`export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 && cargo test …`. The
agent's shell state does not carry over between tool calls, so every later
`cargo` run built with incremental compilation and full debug info. By the
time anyone checked, `target/debug/incremental` was 653 MB and the target dir
was 1.5 GB. That is most of the disk budget, spent on a cache nobody asked for.
Changing the env vars later also changes the profile fingerprint, which forces
one full rebuild.

**Fix.** Delete `target/debug/incremental` and the test executables in
`target/debug/deps` (the rlibs are enough to relink). Then put the variables on
**each** command line:
`CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 cargo test --offline -p …`.

**Next time.** In an agent session, treat every Bash call as a fresh shell.
Prefix every `cargo` line with any env var that matters for size or
correctness, or put it in a config file. Check `du -sh target` after the first
build, not after the fifth.
