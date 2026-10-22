---
category: Repo policy / workflow reminders
---

# Expand all declared consumer paths before running portable coverage

The DER portable-coverage unittest was run in a Rust-focused sparse worktree,
so it failed when it audited the first omitted C# source and fixture. This gate
validates every consumer listed in the portable manifest, not just the edited
language; add every declared consumer package to the target worktree's sparse
checkout before running it.
