---
category: Rust
---

# Workspace Cargo checks must account for deliberately platform-gated packages

The Rust monorepo contains packages that intentionally fail compilation on the
wrong host, such as `paint-vm-direct2d` on macOS. An unconditional
`cargo check --workspace` therefore reached an unrelated platform guard after
the changed package had already passed its focused tests, clippy, and rustdoc.
Use the changed package's complete target set and repository build script as
the portable validation boundary, and leave the platform-specific workspace
matrix to its matching CI hosts. When reporting a local workspace-wide check,
name the exact platform gate instead of presenting it as a product regression.
