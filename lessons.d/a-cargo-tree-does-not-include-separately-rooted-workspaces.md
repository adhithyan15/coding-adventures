---
category: Cargo & dependency graphs
---

# Cargo tree does not include separately rooted workspaces

I checked the Rust root workspace's reverse-dependency tree and missed the
standalone `code/packages/wasm/heap` crate because it has its own `[workspace]`.
Search every tracked Cargo manifest for path dependencies resolving to the
changed package directory, then inspect the source import and its own lockfile.
The workspace tree is useful for transitive Rust consumers, but it is not a
repository-wide manifest inventory.
