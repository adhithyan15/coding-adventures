# RSA Public Operation

## Status

Implemented as the zero-external-dependency Rust package
`rsa-public-operation`.

## Purpose

Apply RFC 8017 RSAVP1 to one modulus-width signature using a previously
validated RSA public key. The operation converts the signature octets to an
integer `s`, computes `m = s^e mod n`, and returns the unique `k`-octet
big-endian representation of `m`, where `k` is the normalized modulus length.

This is the arithmetic boundary between the shipped `x509-rsa-public-key`
parser/strength-policy evidence and the shipped
`rsa-pkcs1-v1-5-sha256` encoded-message validator. It is not by itself a
signature verifier: it neither hashes signed bytes nor interprets the recovered
encoded message.

## Input contract

The caller supplies:

- one `ValidatedRsaPublicKey` produced by `x509-rsa-public-key`;
- one signature byte string whose length is exactly the normalized modulus
  length; and
- one validated operational limit whose nonzero maximum modulus length is
  expressed in bytes.

The public-key evidence has already established that the modulus is positive,
odd, normalized, within caller-supplied bit-length policy, and larger than the
odd allowed `u64` exponent. This package does not accept raw key bytes and does
not repeat ASN.1 or key-strength policy.

Before any arbitrary-precision conversion or exponentiation, the operation
must reject a modulus longer than the operational limit and a signature whose
length differs from the modulus length. After conversion it must reject a
signature representative `s >= n`, as required by RSAVP1.

The modulus, exponent, signature, and recovered encoded message are public
verification data. No private exponent, prime, CRT coefficient, signing
operation, secret-dependent branch, or secret-derived temporary is accepted or
created by this package.

## Public contract

```rust
pub struct RsaPublicOperationLimits { /* private */ }

pub enum RsaPublicOperationLimitsErrorKind {
    ZeroMaximumModulusBytes,
}

pub struct RsaPublicOperationLimitsError { /* redacted private field */ }

impl RsaPublicOperationLimits {
    pub fn new(
        maximum_modulus_bytes: usize,
    ) -> Result<Self, RsaPublicOperationLimitsError>;

    pub const fn maximum_modulus_bytes(self) -> usize;
}

pub enum RsaPublicOperationErrorKind {
    ModulusTooLong,
    SignatureLengthMismatch,
    SignatureRepresentativeOutOfRange,
}

pub struct RsaPublicOperationError { /* redacted private field */ }

pub fn recover_encoded_message(
    public_key: ValidatedRsaPublicKey<'_>,
    signature: &[u8],
    limits: RsaPublicOperationLimits,
) -> Result<Vec<u8>, RsaPublicOperationError>;
```

The returned vector is exactly the modulus length. Leading zero octets are
restored after modular exponentiation, so downstream encoded-message validation
always sees the complete RFC 8017 `EM` value rather than a minimal integer
encoding.

## Arithmetic and resource bounds

The implementation uses the repository-owned `bignum-core` integer substrate.
Unsigned big-endian conversion is performed by bounded multiply-add over the
already length-checked input. Modular exponentiation uses square-and-multiply:
the base is reduced once, every multiplication is reduced immediately, and the
exponent is the validated public `u64` value. Consequently intermediates stay
bounded by approximately twice the caller-limited modulus width and the loop
runs at most 64 exponent-bit iterations.

The final nonnegative representative is rendered with the integer substrate's
reviewed lowercase-hexadecimal formatter, decoded nibble by nibble, and
left-padded to the exact modulus length. Producing more bytes than the modulus
length is an internal invariant violation, not a recoverable input state.

All arithmetic is over public verification data. This slice does not claim a
constant-time implementation and must not be reused for private RSA operations.

## Errors and retained data

Operational-limit and public-operation errors expose only stable categories.
They retain and format no modulus, exponent, signature, recovered message,
certificate, provider, or trace data. Length values are intentionally omitted
from errors so callers cannot accidentally promote them into sensitive logging
contracts later.

## Tests

Tests must cover:

- rejection of a zero maximum-modulus-byte limit;
- rejection before arithmetic when a validated modulus exceeds the operational
  limit;
- exact signature-length enforcement for shorter and longer inputs;
- rejection of representatives equal to and greater than the modulus;
- textbook small RSA public-operation examples;
- zero, one, and leading-zero recovered representatives padded to the exact
  modulus width;
- at least one multi-limb vector independently computed with another exact
  integer implementation and pinned as static bytes;
- sparse and dense permitted odd exponents, including `65537`;
- stable redacted errors and opaque validated-key-only entry; and
- source checks proving there is no `unsafe` code or private-RSA API.

## Deliberate exclusions

- raw RSA key parsing, DER, SPKI, or key-strength policy;
- private exponents, CRT, signing, decryption, blinding, or key generation;
- hashing signed bytes or selecting certificate `tbsCertificate` bytes;
- PKCS #1 v1.5, PSS, or any other encoded-message interpretation;
- a certificate-signature verifier or algorithm registry;
- certificate name comparison, path construction or validation, trust-root
  integration, clocks, revocation, TLS, HTTP, sockets, or OAuth transport.
