---
category: Rust
---

# Sibling cryptographic hash packages may expose different public function names

An OCSP SHA-512 verifier copied the neighboring SHA-256 adapter's call shape
and imported a nonexistent `sha512` function. The repository-owned SHA-512
crate exposes `sum512`, while the SHA-256 crate exposes `sha256`. Before
adapting one cryptographic composition to another algorithm, inspect both the
target primitive's package metadata and its exported API. Similar algorithms
and similarly shaped crates do not imply uniform public names.
