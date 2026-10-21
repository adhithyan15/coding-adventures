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

Its validity-at-time gate classifies the already decoded inclusive certificate
interval against one explicit validated `X509Time`. The caller retains all
authority and responsibility for clock acquisition, UTC conversion, freshness,
and audit.

Its purpose gate consumes already validated role evidence and authorizes one
exact caller-supplied Extended Key Usage OID only for an end entity. Missing
Extended Key Usage fails closed. The gate embeds no purpose registry, and
`anyExtendedKeyUsage` is an ordinary exact OID rather than an implicit
wildcard.

Its issuer-link gate invokes one caller-supplied distinguished-name comparator
exactly once with a child certificate's issuer followed by a candidate issuer
certificate's subject. It fixes the link direction without embedding or
claiming a complete RFC 5280 name-normalization rule.

Its self-issued classifier invokes the same kind of caller-owned comparator
exactly once with one certificate's issuer followed by its subject. The
non-copyable result retains a reference to that exact decoded certificate so a
later path-policy layer does not need to accept an unrelated boolean. The
classification does not imply that the certificate is self-signed or trusted.

An intended issuing CA must carry critical `cA = TRUE` Basic Constraints and
Key Usage with `keyCertSign`. An intended end entity may omit those extensions,
but it must not assert either CA status or certificate-signing usage. Extended
Key Usage is decoded when present and retained as borrowed evidence without
authorizing any purpose until the separate purpose gate succeeds.

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
role or trust. It provides no built-in issuer/subject name normalization or
semantic equality and does not construct or validate paths, verify signatures,
interpret keys, check validity against a clock, process revocation, select
trust roots, or perform TLS, HTTP, or OAuth transport. Passing an algorithm
profile proves only exact identifier, parameter, and bit-length checks;
supplying an Ed25519, RSA,
or other OID does not enable the algorithm or authorize a cryptographic
implementation. Algorithm-specific key parsing and strength policy remain
separate mandatory work for a repository-owned parser and verifier.

Passing the server-identity gate proves only that one already validated caller
reference matched a typed Subject Alternative Name value. It neither replaces
the critical-extension gate nor authenticates the certificate until separate
path, signature, validity, revocation, purpose, and trust-root checks also
succeed.

Passing the validity-at-time gate proves only that one explicit caller-owned
observation fell within the inclusive interval. The gate reads no ambient clock
and applies no tolerance. Its opaque evidence does not prove the observation's
source or freshness and grants no other certificate or transport authority.

Passing the purpose gate proves only that one exact OID was present in the
already decoded Extended Key Usage of end-entity role evidence. It does not
authorize a key operation and grants no signature, path, identity, validity,
trust, revocation, TLS, HTTP, or OAuth transport authority.

Passing the issuer-link gate proves only that one caller-owned comparator
accepted one directed child-issuer to candidate-subject pair. The opaque
evidence retains neither Name nor certificate bytes. It does not prove
standards-complete name equality, CA role, signature, path, constraints,
validity, purpose, identity, trust, or revocation.

Passing the self-issued classifier proves only that the caller-owned comparator
was applied in issuer-to-subject order for the exact retained certificate. The
caller still owns complete Name normalization and equality semantics. The
classification grants no role, signature, path, constraint, trust, revocation,
TLS, HTTP, or network authority.

## Example

```rust,no_run
use der_asn1::Asn1Limits;
use tls_server_identity::ReferenceIdentity;
use x509_certificate::decode_x509_certificate;
use x509_certificate_policy::{
    classify_certificate_self_issued, validate_certificate_algorithm_profile,
    validate_certificate_extended_key_purpose, validate_certificate_issuer_link,
    validate_certificate_role_extensions, validate_certificate_server_identity,
    validate_critical_extensions, X509AlgorithmParametersPolicy,
    X509CertificateAlgorithmProfile, X509CertificatePurposePolicy, X509CertificateRole,
};

# fn check(encoded: &[u8], candidate_issuer_encoded: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
let certificate = decode_x509_certificate(encoded, Asn1Limits::default())?;
let candidate_issuer =
    decode_x509_certificate(candidate_issuer_encoded, Asn1Limits::default())?;
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
// Purpose OIDs remain caller-owned policy data. This exact serverAuth value
// does not select a key operation or enable any cryptographic implementation.
let purpose = X509CertificatePurposePolicy::new(&[1, 3, 6, 1, 5, 5, 7, 3, 1])?;
let _purpose_evidence = validate_certificate_extended_key_purpose(&evidence, purpose)?;
// Exact DER is one intentionally strict caller policy, not a claim of complete
// RFC 5280 distinguished-name normalization or semantic equality.
let _issuer_link = validate_certificate_issuer_link(
    &certificate,
    &candidate_issuer,
    |issuer, subject| issuer.encoded() == subject.encoded(),
)?;
let self_issued = classify_certificate_self_issued(&certificate, |issuer, subject| {
    issuer.encoded() == subject.encoded()
});
let _caller_classification = self_issued.is_self_issued();
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
