# x509-subject-key-identifier

This package recognizes the RFC 5280 Subject Key Identifier extension OID
`2.5.29.14` and decodes its exact encapsulated primitive OCTET STRING. The
identifier remains borrowed from caller-owned input. A caller-selected nonzero
byte ceiling limits accepted identifiers, and empty identifiers fail closed.

```rust
use x509_subject_key_identifier::SubjectKeyIdentifierPolicy;

let policy = SubjectKeyIdentifierPolicy::new(64).unwrap();
assert_eq!(policy.maximum_identifier_bytes(), 64);
```

## Security boundary

This package performs no identifier derivation or comparison and does not
parse Authority Key Identifier, choose or authenticate an issuer, construct or
validate a certificate path, enforce certificate roles or criticality, verify
signatures, select trust, process revocation, perform TLS or HTTP, access
credentials, or perform I/O. Successful syntax decoding grants no certificate,
path, trust, TLS, OAuth, or network authority.
