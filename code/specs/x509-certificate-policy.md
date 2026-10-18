# X.509 Certificate Policy

## Status

Implemented as a zero-external-dependency Rust composition above the complete
`x509-certificate` structural decoder.

## Purpose

Apply small, auditable RFC 5280 policy gates to one already decoded complete
certificate. The first gate implements the certificate-wide unknown-critical-
extension rule: every critical extension must be recognized by the caller's
data-driven policy, while an unrecognized non-critical extension may be
ignored.

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

## Tests

Tests must cover:

- certificates with no extensions;
- unknown non-critical extensions without recognizer invocation;
- recognized critical extensions;
- failure on the first unknown critical extension in wire order;
- skipping non-critical entries when reporting the original sequence index;
- stopping recognizer evaluation at the first failure; and
- redacted errors that expose neither OIDs nor extension contents.

## Deliberate exclusions

- an embedded extension registry or provider-specific OID table;
- decoding or semantically validating recognized extension values;
- Basic Constraints, Key Usage, Extended Key Usage, or certificate-role policy;
- signature or public-key algorithm policy and signature verification;
- certificate name comparison, path construction, or path validation;
- clocks, hostname validation, trust roots, revocation, TLS records,
  handshakes, sockets, HTTP, or OAuth transport.

This slice also adds no process, filesystem, environment, clock, randomness,
network, credential, native-link, or external-service authority.
