# x509-rsa-public-key

Allocation-free validation for the RFC 3279 `rsaEncryption`
`SubjectPublicKeyInfo` representation.

The crate requires the exact RSA algorithm identifier with canonical NULL
parameters, decodes the nested PKCS #1 public modulus and exponent, and applies
an explicit caller-supplied modulus-bit range and exponent allowlist. Successful
values borrow the normalized public modulus from the input and can only be
constructed by the complete parser and policy gate.

```rust
use der_asn1::Asn1Limits;
use x509_rsa_public_key::{validate_rsa_public_key, RsaPublicKeyPolicy};

# fn validate(
#     spki: x509_subject_public_key_info::X509SubjectPublicKeyInfo<'_>,
# ) -> Result<(), Box<dyn std::error::Error>> {
let allowed_exponents = [65_537];
let policy = RsaPublicKeyPolicy::new(2048, 4096, &allowed_exponents)?;
let key = validate_rsa_public_key(spki, policy, Asn1Limits::default())?;
assert!((2048..=4096).contains(&key.modulus_bits()));
# Ok(())
# }
```

This package performs no RSA arithmetic, signature verification, certificate
path or hostname validation, trust-root selection, revocation checking, TLS,
HTTP, network I/O, or private-key operation. Passing this gate is structural
and policy evidence only.

## Dependencies

- der-asn1
- x509-subject-public-key-info

## Development

```bash
# Run tests
bash BUILD
```
