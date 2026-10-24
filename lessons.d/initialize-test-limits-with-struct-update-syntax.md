---
category: Rust
---

# Initialize test limits with struct update syntax

A decoder-budget test created `Asn1Limits::default()` and reassigned one field,
which failed the workspace's warning-denied Clippy gate under
`field_reassign_with_default`. Build altered test policies with a struct
literal and `..Default::default()` so focused tests already satisfy the final
lint contract.
