---
category: Rust
---

# Two worktrees must not share one CARGO_TARGET_DIR: a path dependency's fingerprint ignores which checkout it came from

To save disk and build time, I built a PR worktree
(`.claude/worktrees/pr`, on fresh `origin/main`) with
`CARGO_TARGET_DIR` pointed at the main checkout's `target/`. Back in the
main checkout, which was on a branch that had *added* a function to
`chief-of-staff-daemon-secret-file`, a crate that called the new function
failed:

```text
error[E0425]: cannot find function `read_owner_only_secret_from`
  ...
  = note: similarly named function `read_owner_only_secret` defined here
```

The function was right there in the source. Cargo had reused the
worktree's build of the crate. A workspace path dependency's package id and
fingerprint come from its workspace-relative path, which is identical in
both checkouts. The worktree's files were newer, so the artifact looked
fresh and was linked against the older API. It could just as easily have
compiled and *run* the wrong code silently, with tests passing against
stale dependencies.

**Do instead:**
- Give each worktree its own target directory: leave `CARGO_TARGET_DIR`
  unset there, or point it somewhere per worktree.
- If you have already shared one, `touch` the sources of every crate that
  differs between the checkouts (`git diff --name-only A...B`), or
  `cargo clean -p <crate>` for each, before trusting a build in the other
  checkout.

**It happened again (2026-10-19).** The session disk filled at 97%, so I
deleted the per-worktree target directories and pointed three worktrees at
one `target-baseline/`. Clippy in the PR 2 worktree built `aarch64-encoder`
without `FRINTZ`. Clippy in the PR 3 checkout then reused that build and
failed with `no method named frintz`, although the source had it. Earlier, a
full `lang_matrix` run in that setup failed one test that passed on every
rerun, probably the same reuse. Under disk pressure, sharing is sometimes the
only option. In that case `touch` every crate the two checkouts differ in
before *each* build you rely on, not once, and treat a failure that
disappears on rerun as this until you have proved otherwise.
