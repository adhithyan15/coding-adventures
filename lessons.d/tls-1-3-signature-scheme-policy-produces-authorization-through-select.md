---
category: Rust
---

# TLS 1.3 signature-scheme policy produces authorization through select

`SignatureSchemePolicy` exposes `select(wire_id)`, not an inferred
`authorize(wire_id)` method. A new composition test guessed the latter name and
failed at compile time after the full RSA dependency stack had built.

Before wiring an existing policy evidence type into a new Rust composition,
read its exact public constructor and selector names. Reuse the package's own
tests as the call-site reference instead of inferring an API verb from the
returned evidence type.
