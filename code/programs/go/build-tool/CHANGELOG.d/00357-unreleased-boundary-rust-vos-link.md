## Unreleased - source-input boundary sync (rust vos-link program)

- `code/programs/rust/vos-link` added to `rust-root-workspace-manifest-cross-consumers`: the new standalone cargo root runs cargo against the Rust workspace from its BUILD without being listed in the repository boundary. The checked repository source-input boundary added the root, the projection was regenerated, and the pinned boundary digest moved from `2bfdfd58...` to `f20e5840...`. The boundary has 505 scopes and 519 authorizations (were 504 and 518). No behaviour change in the tool.
