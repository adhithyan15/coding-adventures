# tls13-signature-scheme-policy

Allocation-free, data-driven policy for the RFC 8446 RSASSA-PSS RSAE signature
schemes.

The package binds each recognized wire identifier to its message hash, MGF1
hash, digest-sized salt length, and required `rsaEncryption` public-key form.
Callers provide an ordered non-empty, duplicate-free allowlist, and selection
accepts only an exact known identifier present in that policy.

```rust
use tls13_signature_scheme_policy::{SignatureScheme, SignatureSchemePolicy};

let allowed = [SignatureScheme::RsaPssRsaeSha256];
let policy = SignatureSchemePolicy::new(&allowed).unwrap();
let selected = policy.select(0x0804).unwrap();
assert_eq!(selected.parameters().salt_length(), 32);
```

## Boundary

This registry is routing metadata, not implementation evidence. It does not
encode or parse a TLS extension, advertise a scheme, hash a transcript, parse
keys, perform RSA or MGF1, validate RSA-PSS encoding, verify signatures,
process certificates, establish trust, check revocation, run TLS/HTTP, or open
a socket. EdDSA is absent pending hardening of the repository implementation;
ECDSA and RSASSA-PSS-key-OID schemes are also intentionally absent.

## Development

```bash
bash BUILD
```
