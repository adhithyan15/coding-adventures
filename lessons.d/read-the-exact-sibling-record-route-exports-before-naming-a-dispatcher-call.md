---
category: Rust
---

# Read the exact sibling record route exports before naming a dispatcher call

The first TLS record dispatcher compile guessed that the existing ChaCha20-
Poly1305 package exported `seal_tls13_chacha20_poly1305_record` and
`open_tls13_chacha20_poly1305_record`, mirroring the full package name. The
reviewed package deliberately exports the shorter `*_chacha20_record` names,
so the new crate failed before its tests could run.

The fix was only to call the exact existing exports. Before writing a sibling
algorithm dispatcher, inspect each public function signature rather than
deriving names from package names or the neighboring route. Similar record
compositions share semantics, not necessarily identifiers; compiling early is
the check that prevents a guessed API from spreading into tests and docs.
