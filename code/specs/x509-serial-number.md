# X.509 Certificate Serial Number

## Status

Implemented as a zero-external-dependency Rust semantic layer above
`der-asn1`.

## Purpose

Decode the RFC 5280 certificate serial-number field without parsing the rest of
`TBSCertificate`:

```asn1
CertificateSerialNumber ::= INTEGER
```

The value must be a canonical DER INTEGER, strictly positive, nonzero, and no
longer than 20 content octets. The bound counts a required leading zero sign
octet when the unsigned magnitude begins with a set high bit.

## Public contract

```rust
pub const MAX_SERIAL_NUMBER_OCTETS: usize = 20;

pub struct X509SerialNumber<'a> { /* borrowed validated bytes */ }

impl<'a> X509SerialNumber<'a> {
    pub fn encoded_value(self) -> &'a [u8];
    pub fn magnitude(self) -> &'a [u8];
    pub fn encoded_len(self) -> usize;
}

pub fn decode_x509_serial_number(
    element: Asn1Element<'_>,
) -> Result<X509SerialNumber<'_>, X509SerialNumberError>;
```

The caller supplies one already-framed element. The result retains the exact
canonical INTEGER contents and a normalized unsigned magnitude view that omits
only a required zero sign-protection octet.

## Limits and diagnostics

Framing and input-size policy remain with the caller's `Asn1Decoder`. This
primitive adds the RFC 5280 20-content-octet ceiling and performs no allocation.
Errors contain only a stable category and local byte offset; Debug output for a
successful value reports lengths rather than serial bytes.

## Tests

Tests cover ordinary and sign-protected positive values, both exact 20-octet
forms, the 21st-octet boundary, zero, negative and non-minimal integers, wrong
tags, caller DER limits, and redacted diagnostics.

## Deliberate exclusions

- generating serial numbers or proving uniqueness per issuer;
- issuer or subject names and certificate-wide schema rules;
- certificate path construction, signature verification, or algorithms;
- clocks, revocation, hostname verification, trust-root loading, TLS, sockets,
  or OAuth transport.
