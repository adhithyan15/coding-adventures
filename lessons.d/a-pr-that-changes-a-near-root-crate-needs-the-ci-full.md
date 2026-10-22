---
category: CI & GitHub Actions
---

# A PR that changes a near-root crate needs the ci-full-build label, because the unsharded PR build runs out of disk

PR #17803 changed only the Rust `lexer` crate and its JavaScript wrapper.
Almost every language frontend depends on `lexer`, so the PR's build plan
marked 5,337 packages as affected. Pull-request builds are not sharded: one
`ubuntu-latest` runner built all of them. That build died twice at the same
point, around package 4,380, and no test or compile step had failed:

- The first attempt was reported only as "the hosted runner lost communication
  with the server."
- The rerun's annotation showed the real cause: `No space left on device` while
  the runner wrote its own diagnostic log.

The macOS leg passed both times.

Rerunning the job cannot fix this, because the disk fills at the same point
every time. The fix is the `ci-full-build` label. It makes the PR run main's
forced, sharded full build, which splits the work across several Linux runners
and reclaims disk between packages.

Labelling alone does not start a run, and a rerun replays the old event's
labels. So add the label, then push a commit. When a PR touches a crate most of
the tree depends on (`lexer`, `parser`, `grammar-tools` and similar), add the
label before the first push. Don't spend a 2.5-hour unsharded run finding out.
