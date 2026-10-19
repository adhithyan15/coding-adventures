---
category: Rust
---

# Integration tests must not construct public enum variants whose payload type is private

`der-asn1::Asn1ErrorKind::Framing` is public, but its `DerErrorKind` payload is
not re-exported. An integration test that tried to construct
`Framing(DerErrorKind::TruncatedValue)` therefore failed to compile. Match
`Asn1ErrorKind::Framing(_)` when the public contract is the framing category,
or add a direct dependency on the payload's owning crate only when the exact
nested value is itself part of the package contract.
