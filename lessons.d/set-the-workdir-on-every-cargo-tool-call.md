---
category: Repo policy / workflow reminders
---

# Set the workdir on every Cargo tool call

After identifying the Rust workspace root, a later `cargo fmt` still ran from
the repository root because its tool call omitted the intended working
directory. For every Cargo invocation in a multi-root validation sequence, set
`workdir` explicitly to `code/packages/rust`; do not rely on the previous
command's directory or a remembered shell state.
