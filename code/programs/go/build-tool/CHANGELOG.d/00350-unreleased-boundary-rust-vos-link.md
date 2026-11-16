## Unreleased - source-input boundary sync (rust vos-link program)

- `code/programs/rust/vos-link` added to `rust-root-workspace-manifest-cross-consumers`: the new standalone cargo root runs cargo against the Rust workspace from its BUILD without being listed in the repository boundary. The checked repository source-input boundary added the root, the projection was regenerated, and the pinned boundary digest moved from `16dddd56...` to `c519201d...`. The boundary has 502 scopes and 516 authorizations (were 501 and 515). No behaviour change in the tool.
