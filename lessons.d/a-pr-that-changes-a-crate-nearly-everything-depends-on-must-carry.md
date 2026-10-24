---
category: CI & GitHub Actions
---

# A PR that changes a crate nearly everything depends on must carry the ci-full-build label

**What went wrong.** Two PRs changed the generic `lexer` crate: #17822, which
applies F10 transitions to NEWLINE, and #17824, which adds F11 adjacency
predicates. Almost every package in the repo depends on that crate, so the
affected-package plan was about 5,300 of 5,337 packages, which is effectively
a full build. A pull request builds its affected set on one unsharded runner.

On both PRs `build (ubuntu-latest)` died about 38 minutes in, at package
~4,370/5,337, while it was building Rust lexer crates. The log stopped
mid-progress-bar, the "Build and test affected packages" step was still
`in_progress` when the job concluded `failure`, and no post-job step ran.
That is the disk-full signature recorded in
`a-runner-that-fills-its-disk-loses-its-own-log-so-measure.md`. Re-running
the job would not help: the same plan would fill the same disk again.

**The fix.** Add the `ci-full-build` label, then push. The label alone does
not start a run, and a re-run replays the old event's labels. With the label,
the PR runs main's forced, 15-shard Linux build
(`code/specs/build-plan-sharding.md`, "CI Shape"), which is built for exactly
this size.

**Next time.** Before pushing a change to a crate at the root of the
dependency graph (`lexer`, `parser`, `grammar-tools`, `interpreter-ir` and
similar), run `build-tool -diff-base origin/main -dry-run` and look at the
affected count. If it is a large fraction of the repo, label the PR
`ci-full-build` before its first push. The sharding spec's assumption that
pull-request builds "are already narrow" does not hold for these crates.
