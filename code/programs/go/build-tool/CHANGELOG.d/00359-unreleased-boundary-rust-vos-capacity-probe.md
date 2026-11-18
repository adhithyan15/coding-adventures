## Unreleased - source-input boundary sync (rust vos-capacity-probe program)

- `code/programs/rust/vos-capacity-probe` added to `rust-root-workspace-manifest-cross-consumers`: the new standalone cargo root runs cargo against the Rust workspace from its BUILD without being listed in the repository boundary. The checked repository source-input boundary added the root, the projection was regenerated, and the pinned boundary digest moved from `f20e5840...` to `7e148f79...`. The boundary has 506 scopes and 520 authorizations (were 505 and 519). No behaviour change in the tool.
