# X.509 RSA Public Keys

## Status

Planned as a zero-external-dependency Rust policy layer above
`x509-subject-public-key-info` and `der-asn1`.

## Purpose

Decode the RFC 3279 `rsaEncryption` SubjectPublicKeyInfo representation and
enforce one explicit caller-supplied RSA public-key strength policy. This is a
bounded prerequisite for repository-owned RSA signature verification; it is
not an RSA arithmetic or signature implementation.

The outer value has already been decoded as:

```asn1
SubjectPublicKeyInfo ::= SEQUENCE {
    algorithm         AlgorithmIdentifier,
    subjectPublicKey  BIT STRING
}

RSAPublicKey ::= SEQUENCE {
    modulus           INTEGER,
    publicExponent    INTEGER
}
```

## Input contract

The caller supplies one unforgeable decoded `X509SubjectPublicKeyInfo`, one
validated `RsaPublicKeyPolicy`, and independent `Asn1Limits` for the nested
`RSAPublicKey` document.

The algorithm identifier must contain the exact `rsaEncryption` OID
`1.2.840.113549.1.1.1` and a present canonical DER NULL parameter. The key BIT
STRING must be non-empty and byte-aligned. Its payload must be one exact
constructed universal SEQUENCE containing exactly two canonical INTEGERs.

The modulus must be positive, nonzero, and odd. Its normalized unsigned
magnitude excludes only the required DER sign-protection zero byte. Its exact
bit length must fall inside the caller's inclusive nonzero modulus-bit range.

The public exponent must fit `u64`, be odd, be at least three, be strictly less
than the modulus, and equal one entry in the caller's non-empty allowlist. The
policy constructor rejects an empty exponent list, invalid exponents,
duplicates, zero minimum modulus bits, and inverted modulus-bit ranges.

## Public contract

```rust
pub const RSA_ENCRYPTION_OID: &[u64] =
    &[1, 2, 840, 113549, 1, 1, 1];

pub struct RsaPublicKeyPolicy<'a> { /* validated private fields */ }

impl<'a> RsaPublicKeyPolicy<'a> {
    pub fn new(
        minimum_modulus_bits: usize,
        maximum_modulus_bits: usize,
        allowed_public_exponents: &'a [u64],
    ) -> Result<Self, RsaPublicKeyPolicyDefinitionError>;
}

pub struct ValidatedRsaPublicKey<'a> { /* borrowed private fields */ }

impl<'a> ValidatedRsaPublicKey<'a> {
    pub fn modulus(self) -> &'a [u8];
    pub fn modulus_bits(self) -> usize;
    pub fn public_exponent(self) -> u64;
}

pub fn validate_rsa_public_key<'key, 'policy>(
    subject_public_key_info: X509SubjectPublicKeyInfo<'key>,
    policy: RsaPublicKeyPolicy<'policy>,
    limits: Asn1Limits,
) -> Result<ValidatedRsaPublicKey<'key>, RsaPublicKeyError>;
```

Successful values borrow only the normalized public modulus from the caller's
certificate bytes. Public accessors expose public-key material, never private
or secret values. Construction remains private so callers cannot forge policy
evidence without passing the complete decoder and strength checks.

## Limits and errors

The nested exact document, sequence, and two INTEGER children share the
caller's depth, total-element, tag, and value-length limits. No bignum or heap
allocation is required. Bit-length calculation examines only the first
normalized modulus byte; exponent conversion is bounded to eight magnitude
bytes.

Key-decode errors contain only a stable category and byte offset local to the
inner `RSAPublicKey` payload. Algorithm and BIT STRING shape errors use offset
zero because the generic decoded SPKI intentionally does not expose their
original outer offsets. Policy-definition errors contain only a stable
category. Neither error type retains or formats OIDs, policy values, modulus,
exponent, key, or certificate bytes.

## Tests

Tests must cover:

- exact `rsaEncryption` with required NULL parameters;
- valid sign-protected moduli at both inclusive bit-length boundaries;
- multiple caller-allowed exponents and closed rejection of unlisted values;
- empty, inverted, invalid-exponent, and duplicate policy definitions;
- wrong algorithm, absent/non-NULL parameters, empty/non-byte-aligned keys;
- primitive or malformed inner roots, missing fields, and a third field;
- zero, negative, even, undersized, and oversized moduli;
- zero, negative, even, too-small, overflowing, and modulus-sized exponents;
- depth, element, tag, and value-length limit exhaustion; and
- stable redacted errors that retain no policy or public-key bytes.

## Deliberate exclusions

- private-key parsing, generation, custody, or signing;
- primality, factor, totient, or coprimality proofs;
- modular arithmetic, PKCS #1 v1.5, PSS, or signature verification;
- a general algorithm registry or automatic algorithm choice;
- certificate signatures, path construction or validation, trust roots,
  hostname checks, clocks, revocation, TLS, HTTP, sockets, or OAuth transport.
