---
category: Workspace & package metadata
---

# Rust workspace `Cargo.toml` `members` must match what's pushed

Listing a member whose dir hasn't been pushed breaks the entire workspace in CI (`failed to load manifest`). Crates with their own `[workspace]` (node-bridge, python-bridge, ruby-bridge) must be EXCLUDED from the parent — including them gives "multiple workspace roots". After merge conflicts on `members`, dedupe — modern CI rejects duplicate entries even though older Cargo tolerated them. Run `cargo build --workspace` to catch missing exports; expect platform-only crates (paint-vm-direct2d, paint-vm-gdi) to fail compile on the wrong OS — that's not a regression.
