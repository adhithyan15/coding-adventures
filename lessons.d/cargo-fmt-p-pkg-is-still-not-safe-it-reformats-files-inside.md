---
category: Rust
---

# `cargo fmt -p <pkg>` is still not safe — it reformats files *inside that package* you never touched, and `main` is not necessarily clean under YOUR local rustfmt

Hit twice on `task-core`: adding a struct to `model.rs` and running `cargo fmt -p task-core` also rewrote `scheduler.rs` (~30 lines of match-guard/assert re-wrapping) because the local rustfmt version disagrees with whatever formatted main. That churn is unrelated to the change, invites a "why is scheduler.rs in this diff?" review, and risks conflicting with concurrent PRs. **Always `git diff --stat` right after `cargo fmt -p <pkg>` and `git checkout -- <file>` anything you didn't intend to touch**, then re-run the tests. Corollary: don't "fix" a fmt diff in a file your change doesn't own — CI does not run `cargo fmt --check` as a blocking gate, so leave main's formatting alone. (Same shape as the generated-file lesson below: fmt, then selectively revert.)
