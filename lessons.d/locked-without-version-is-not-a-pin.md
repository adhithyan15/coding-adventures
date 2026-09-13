---
category: Supply chain & CI pinning
---

# `--locked` without `--version` is not a pin

It locks the dependency tree of whatever version resolves, so the tool itself still drifts. `cargo install --locked cargo-geiger` looked pinned and was not.
