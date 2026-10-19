# X.509 Certificate Policy

## Status

Implemented as a zero-external-dependency Rust composition above the complete
`x509-certificate` structural decoder.

## Purpose

Apply small, auditable RFC 5280 policy gates to one already decoded complete
certificate. The first gate implements the certificate-wide unknown-critical-
extension rule: every critical extension must be recognized by the caller's
data-driven policy, while an unrecognized non-critical extension may be
ignored. The second gate decodes Basic Constraints, Key Usage, and Extended
Key Usage under one explicit semantic budget and enforces only their
algorithm-independent issuing-CA or end-entity role relationships.
The third gate selects exactly one caller-supplied certificate algorithm
profile and checks its exact AlgorithmIdentifier parameter encodings and
encoded public-key and signature BIT STRING length bounds.
The fourth gate decodes the complete certificate's Subject Alternative Name
extension and applies the repository-owned RFC 9525 DNS-ID/IP-ID matcher to one
already validated, caller-selected reference identity.
The fifth gate classifies the certificate's already decoded inclusive validity
interval at one explicit, already validated caller-supplied observation.

## Input contract

The caller supplies an immutable `X509Certificate` and a recognition function.
The function receives each critical extension OID exactly once in certificate
wire order. It is not invoked for non-critical extensions. Evaluation stops at
the first `false` result.

An absent extension sequence succeeds. An unrecognized critical extension
fails closed. Its error reports only the zero-based index within the extension
sequence and retains no OID, extension value, or certificate bytes.

## Public contract

```rust
pub fn validate_critical_extensions<'a>(
    certificate: &X509Certificate<'a>,
    recognizes: impl FnMut(ObjectIdentifier<'a>) -> bool,
) -> Result<(), X509CertificatePolicyError>;
```

Recognition means only that the caller has selected an implementation for the
OID. It does not prove that the implementation decoded the encapsulated value
or enforced its semantics. Those remain mandatory, separate gates before a
certificate can receive any role or trust.

## Certificate-role extension contract

The caller supplies the decoded certificate, one explicit intended role, and
an independent `Asn1Limits` budget for second-stage extension-value decoding.
One decoder shares that budget across every present Basic Constraints, Key
Usage, and Extended Key Usage value in certificate wire order. The complete
certificate decoder has already rejected duplicate extension OIDs.

For `IssuingCertificateAuthority`, policy requires:

- a present, critical Basic Constraints extension with `cA = TRUE`; and
- a present Key Usage extension with `keyCertSign` asserted.

Key Usage criticality is not required here because RFC 5280 says conforming
CAs SHOULD, rather than MUST, mark it critical. Extended Key Usage is decoded
when present but remains uninterpreted for later path-purpose intersection.

For `EndEntity`, policy requires:

- Basic Constraints, when present, to have `cA = FALSE`; and
- Key Usage, when present, not to assert `keyCertSign`.

The end-entity role deliberately does not select a signature/key algorithm,
require one TLS key-usage bit, or authorize an Extended Key Usage purpose.
Those choices depend on later data-driven algorithm and path-purpose policy.

Every present known role extension must decode successfully even when it is
non-critical. A role-policy error exposes only a stable failure category and,
for an extension-local failure, its zero-based extension sequence index. It
retains no OID, extension bytes, key material, or certificate bytes.

The role gate does not replace `validate_critical_extensions`. A caller must
still reject every unsupported critical extension and semantically validate
every other recognized critical extension before granting any role or trust.

```rust
pub enum X509CertificateRole {
    EndEntity,
    IssuingCertificateAuthority,
}

pub fn validate_certificate_role_extensions<'a>(
    certificate: &X509Certificate<'a>,
    role: X509CertificateRole,
    limits: Asn1Limits,
) -> Result<ValidatedCertificateRoleExtensions<'a>, X509CertificateRoleError>;
```

The returned borrowed evidence contains the decoded optional Basic
Constraints, Key Usage, and Extended Key Usage values plus the checked role.
It is evidence only for the relationships above, not proof of a valid chain,
trusted CA, authenticated server, or usable public key.

## Certificate algorithm profile contract

The caller supplies an ordered slice of immutable profiles. Each profile
contains:

- one exact signature algorithm OID and either absent parameters or one exact
  canonical DER parameter element;
- one exact subject-public-key algorithm OID and the same parameter policy;
- an inclusive, nonzero encoded subject-public-key BIT STRING length range; and
- an inclusive, nonzero certificate-signature BIT STRING length range.

Profile construction rejects empty OID arc slices, zero minimum lengths, and
inverted ranges. The library embeds no algorithm registry, preferred profile,
or provider-specific OID table.

Validation compares both complete certificate AlgorithmIdentifiers against
every profile. Parameters match by presence and exact encoded DER bytes. Zero
identifier matches fail as unsupported. More than one identifier match fails
as ambiguous before any BIT STRING length policy is applied. For exactly one
match, the subject-public-key and certificate-signature BIT STRING lengths
must fall within that profile's inclusive bounds.

The subject-public-key bound measures the complete encoded BIT STRING payload,
not an algorithm-specific strength such as an RSA modulus size or EC group.
Those semantics require a later repository-owned key parser and verifier.

```rust
pub enum X509AlgorithmParametersPolicy<'a> {
    Absent,
    ExactDer(&'a [u8]),
}

pub struct X509BitLengthRange { /* private fields */ }
pub struct X509CertificateAlgorithmProfile<'a> { /* private fields */ }

pub fn validate_certificate_algorithm_profile<'certificate, 'policy>(
    certificate: &X509Certificate<'certificate>,
    profiles: &[X509CertificateAlgorithmProfile<'policy>],
) -> Result<ValidatedCertificateAlgorithmProfile<'certificate>,
            X509CertificateAlgorithmError>;
```

Successful evidence contains the selected profile index and borrowed signature
AlgorithmIdentifier, subject-public-key AlgorithmIdentifier, subject public
key BIT STRING, and certificate signature BIT STRING. It neither parses nor
verifies either bit string and grants no cryptographic or trust authority.
Supplying an Ed25519, RSA, or other OID merely supplies policy data; it does
not enable that algorithm or choose a verifier.

## Certificate server-identity contract

The caller supplies a complete decoded certificate, one already validated
`ReferenceIdentity`, and an independent `Asn1Limits` budget for decoding the
Subject Alternative Name extension. The certificate structure has already
rejected duplicate extension OIDs.

The gate requires one present, well-formed Subject Alternative Name extension,
decodes all of its GeneralName values under the explicit budget, and passes
only its typed DNS-ID and IP-ID values to `tls-server-identity`. DNS references
therefore match only dNSName entries and IP references only iPAddress entries.
The API has no subject Common Name input, so CN fallback is impossible.

```rust
pub fn validate_certificate_server_identity<'a>(
    certificate: &X509Certificate<'a>,
    reference: &ReferenceIdentity,
    limits: Asn1Limits,
) -> Result<ValidatedCertificateServerIdentity,
            X509CertificateServerIdentityError>;
```

Errors distinguish an absent extension, malformed Subject Alternative Name,
and closed mismatch. They expose at most the Subject Alternative Name
extension's zero-based wire index and retain no certificate bytes or presented
names. Successful evidence exposes only the matched identity kind and
extension index.

This gate does not replace `validate_critical_extensions` and does not prove
the certificate's path, signature, validity, revocation status, key usage, or
trust. It adds no clock, trust-root, TLS, socket, HTTP, or OAuth authority.

## Certificate validity-at-time contract

The caller supplies a complete decoded certificate and one validated
`X509Time` observation. Time acquisition, UTC conversion, trust, freshness,
and audit remain caller-owned; this gate never reads an ambient clock or
accepts a tolerance window.

The gate reuses the decoded certificate's `X509Validity::classify` semantics.
An observation equal to `notBefore` or `notAfter` succeeds because RFC 5280
defines both endpoints as inclusive. An earlier observation fails as
`NotYetValid`; a later observation fails as `Expired`.

```rust
pub fn validate_certificate_validity_at(
    certificate: &X509Certificate<'_>,
    observed: &X509Time,
) -> Result<ValidatedCertificateValidity,
            X509CertificateValidityError>;
```

Errors expose only one stable failure category and retain no certificate bytes,
encoded time, calendar fields, or caller observation. Success returns opaque
evidence that cannot be constructed outside the crate. The evidence proves
only this one interval comparison at this one caller-owned observation; it
does not assert that the observation came from a trustworthy clock and grants
no path, signature, identity, purpose, revocation, trust-root, TLS, socket,
HTTP, or OAuth authority.

## Tests

Tests must cover:

- certificates with no extensions;
- unknown non-critical extensions without recognizer invocation;
- recognized critical extensions;
- failure on the first unknown critical extension in wire order;
- skipping non-critical entries when reporting the original sequence index;
- stopping recognizer evaluation at the first failure; and
- redacted errors that expose neither OIDs nor extension contents.

Certificate-role tests must additionally cover:

- successful strict issuing-CA evidence with critical `cA = TRUE` and
  `keyCertSign`;
- missing, non-critical, false, or malformed Basic Constraints for an issuing
  CA;
- missing, non-signing, or malformed Key Usage for an issuing CA;
- successful end-entity evidence with absent role extensions or decoded
  `cA = FALSE`, non-signing Key Usage, and arbitrary bounded Extended Key
  Usage;
- rejection of an end entity asserting `cA = TRUE` or `keyCertSign`;
- rejection of malformed Extended Key Usage for either role;
- accepting critical or non-critical Key Usage and Extended Key Usage where
  the role invariants otherwise hold;
- sharing one explicit semantic decoder budget across all present known role
  extensions; and
- redacted role errors that expose neither OIDs nor extension contents.

Certificate-algorithm tests must additionally cover:

- rejection of invalid profile definitions;
- exact OID and absent-versus-exact-DER parameter matching;
- unsupported and ambiguous profile selection;
- inclusive public-key and signature bit-length bounds;
- borrowed evidence for the uniquely selected profile; and
- redacted errors that expose no OID, parameter, key, signature, or
  certificate bytes.

Certificate server-identity tests must additionally cover:

- absent and malformed Subject Alternative Name extensions;
- exact and wildcard DNS-ID matches without Common Name fallback;
- exact IP-ID matches and rejection of textual IP values encoded as dNSName;
- closed identity mismatch;
- critical and non-critical Subject Alternative Name encodings; and
- redacted errors and evidence that retain no presented names or certificate
  bytes.

Certificate validity-at-time tests must additionally cover:

- observations immediately before and after the interval;
- success exactly at both inclusive endpoints and inside the interval;
- certificates whose endpoints use different valid ASN.1 time encodings;
- a zero-duration interval at its single accepted instant; and
- redacted failures that expose neither certificate time values nor the caller
  observation.

## Deliberate exclusions

- an embedded extension registry or provider-specific OID table;
- decoding or semantically validating extensions other than Basic Constraints,
  Key Usage, and Extended Key Usage;
- TLS key-usage selection or Extended Key Usage purpose authorization;
- an embedded signature or public-key algorithm registry;
- public-key or signature parsing, hashing, or signature verification;
- issuer/subject distinguished-name comparison, path construction, or path
  validation;
- clocks, trust roots, revocation, TLS records,
  handshakes, sockets, HTTP, or OAuth transport.

This slice also adds no process, filesystem, environment, clock, randomness,
network, credential, native-link, or external-service authority.
