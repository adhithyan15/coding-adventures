# x509-authority-key-identifier

This package recognizes the RFC 5280 Authority Key Identifier extension OID
`2.5.29.35` and decodes one deliberately narrow profile: a non-critical exact
SEQUENCE containing only the implicit `[0]` keyIdentifier. The identifier is
non-empty, remains borrowed from caller-owned input, and must fit a validated
caller-selected byte ceiling.

The optional authorityCertIssuer `[1]` and authorityCertSerialNumber `[2]`
forms are not silently ignored; this version rejects them as unsupported.

```rust
use x509_authority_key_identifier::AuthorityKeyIdentifierPolicy;

let policy = AuthorityKeyIdentifierPolicy::new(64).unwrap();
assert_eq!(policy.maximum_identifier_bytes(), 64);
```

## Security boundary

This syntax layer does not derive or compare key identifiers, decode Subject
Key Identifier, select or authenticate an issuer, construct or validate a
certificate path, assign certificate roles, verify signatures, select trust,
process revocation, perform TLS or HTTP, access credentials, or perform I/O.
Successful decoding grants no certificate, path, trust, TLS, OAuth, or network
authority.
