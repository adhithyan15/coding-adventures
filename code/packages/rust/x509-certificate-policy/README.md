# x509-certificate-policy

`x509-certificate-policy` supplies small RFC 5280 policy gates above the
repository-owned structural certificate decoder. Its first gate rejects the
first critical extension whose OID is not recognized by a caller-supplied
policy. Its certificate-role gate composes the repository-owned Basic
Constraints, Key Usage, and Extended Key Usage decoders under one explicit
semantic budget.

Its algorithm-profile gate selects exactly one caller-supplied combination of
signature AlgorithmIdentifier, subject-public-key AlgorithmIdentifier, exact
parameter encodings, and inclusive bit-length ranges. It embeds no algorithm
registry and does not parse keys or signatures or choose a verifier.
The subject-public-key range measures its complete encoded BIT STRING, not an
algorithm-specific strength such as an RSA modulus size or EC group.

Its server-identity gate requires and decodes the complete certificate's
Subject Alternative Name extension under a caller-supplied budget, then passes
only typed DNS-ID and IP-ID values to the repository-owned RFC 9525 matcher.
There is no subject Common Name input or fallback.

An intended issuing CA must carry critical `cA = TRUE` Basic Constraints and
Key Usage with `keyCertSign`. An intended end entity may omit those extensions,
but it must not assert either CA status or certificate-signing usage. Extended
Key Usage is decoded when present and retained as borrowed evidence without
authorizing any purpose.

## Security boundary

The recognizer is invoked only for critical extensions, in certificate wire
order. An absent extension sequence and unrecognized non-critical extensions
are accepted. A rejected critical extension fails closed with only its
zero-based sequence index; the error retains no OID or certificate bytes.

Recognition is not semantic validation. The caller must separately decode and
enforce every recognized extension according to the certificate's role. This
crate's role result proves only the extension relationships above; callers must
still run the unknown-critical-extension gate and validate every other
recognized extension. The role gate does not select algorithm-dependent TLS
key usages, authorize an Extended Key Usage purpose, or grant a certificate
role or trust. It does not validate issuer/subject name relationships, paths,
signatures, algorithms, keys, validity against a clock, revocation, trust
roots, TLS, HTTP, or OAuth transport. Passing an algorithm profile proves only
exact identifier, parameter, and bit-length checks; supplying an Ed25519, RSA,
or other OID does not enable the algorithm or authorize a cryptographic
implementation. Algorithm-specific key parsing and strength policy remain
separate mandatory work for a repository-owned parser and verifier.

Passing the server-identity gate proves only that one already validated caller
reference matched a typed Subject Alternative Name value. It neither replaces
the critical-extension gate nor authenticates the certificate until separate
path, signature, validity, revocation, purpose, and trust-root checks also
succeed.

## Example

```rust,no_run
use der_asn1::Asn1Limits;
use tls_server_identity::ReferenceIdentity;
use x509_certificate::decode_x509_certificate;
use x509_certificate_policy::{
    validate_certificate_algorithm_profile, validate_certificate_role_extensions,
    validate_certificate_server_identity, validate_critical_extensions,
    X509AlgorithmParametersPolicy, X509CertificateAlgorithmProfile, X509CertificateRole,
};

# fn check(encoded: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
let certificate = decode_x509_certificate(encoded, Asn1Limits::default())?;
// This deliberately closed example recognizes only the three extensions its
// next gate decodes. A broader profile must validate every additional OID it
// recognizes before assigning trust.
validate_critical_extensions(&certificate, |oid| {
    oid.equals(&[2, 5, 29, 19])
        || oid.equals(&[2, 5, 29, 15])
        || oid.equals(&[2, 5, 29, 37])
        || oid.equals(&[2, 5, 29, 17])
})?;
let evidence = validate_certificate_role_extensions(
    &certificate,
    X509CertificateRole::EndEntity,
    Asn1Limits::default(),
)?;
assert_eq!(evidence.role(), X509CertificateRole::EndEntity);
// These identifiers and encoded BIT STRING bounds are application policy data,
// not an embedded registry, key-strength check, or permission to perform
// cryptography.
let profiles = [X509CertificateAlgorithmProfile::new(
    &[1, 2, 840, 113_549, 1, 1, 11],
    X509AlgorithmParametersPolicy::ExactDer(&[0x05, 0x00]),
    &[1, 2, 840, 113_549, 1, 1, 1],
    X509AlgorithmParametersPolicy::ExactDer(&[0x05, 0x00]),
    2_048,
    8_192,
    2_048,
    8_192,
)?];
let algorithm_evidence = validate_certificate_algorithm_profile(&certificate, &profiles)?;
assert_eq!(algorithm_evidence.profile_index(), 0);
let reference = ReferenceIdentity::parse("api.example.test")?;
let identity_evidence = validate_certificate_server_identity(
    &certificate,
    &reference,
    Asn1Limits::default(),
)?;
let _matched_kind = identity_evidence.match_kind();
# Ok(())
# }
```

## Verification

```bash
bash BUILD
cargo clippy -p x509-certificate-policy --all-targets -- -D warnings
cargo doc -p x509-certificate-policy --no-deps
```
