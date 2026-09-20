# X.509 Certificate Validity

## Status

Proposed zero-external-dependency Rust boundary above `der-asn1` and
`x509-time`.

## Purpose

Decode the RFC 5280 `Validity` structure without acquiring a clock or claiming
that a complete certificate is trusted:

```asn1
Validity ::= SEQUENCE {
    notBefore Time,
    notAfter  Time
}
```

This package is an independently reviewable certificate-schema prerequisite
for repository-owned TLS. It does not parse `Certificate` or
`TBSCertificate`, verify signatures, construct or validate paths, load trust
roots, process revocation, match server names, open sockets, or integrate with
`tls-platform`.

## Input contract

The caller supplies:

- one already-framed `Asn1Element`;
- the same mutable `Asn1Decoder` that framed that element, so child reads share
  its explicit depth and total-element budget.

The root must be one constructed universal `SEQUENCE`. Its value must contain
exactly two child elements in wire order:

1. `notBefore`;
2. `notAfter`.

Missing or additional children fail closed. Each child is decoded by
`x509-time`, which owns the strict RFC 5280 UTCTime and GeneralizedTime profile.
The two fields independently select their valid time encoding; the package
does not require both fields to use the same ASN.1 alternative.

## Semantic contract

`notAfter` must not precede `notBefore`. Equal endpoints are accepted because
RFC 5280 defines the validity period as inclusive at both ends.

The decoded value exposes its two validated endpoints and classifies only an
explicit caller-supplied validated `X509Time`:

- earlier than `notBefore` -> `NotYetValid`;
- from `notBefore` through `notAfter`, inclusive -> `Valid`;
- later than `notAfter` -> `Expired`.

No wall clock, monotonic clock, timezone database, epoch conversion, leap
second interpretation, or platform calendar API is consulted.

## Error and memory contract

Errors are closed categories with a local byte offset where one is available.
They may retain nested `Asn1ErrorKind` or `X509TimeErrorKind` values, but never
retain, format, or disclose input bytes.

The decoder borrows its input while parsing and returns only fixed-size calendar
fields. Production code performs no heap allocation and contains no unsafe
code.

## Limits

All DER framing, nesting, and total-element limits are inherited from the
caller's `Asn1Decoder`. A successful validity decode consumes exactly two child
elements from that shared document budget in addition to the already-decoded
root.

## Tests

Tests must cover:

- exact UTCTime, GeneralizedTime, and mixed-encoding endpoints;
- inclusive equality at both endpoints and classification immediately outside
  the range;
- an equal zero-duration interval;
- inverted ranges;
- primitive or non-SEQUENCE roots;
- missing `notBefore` or `notAfter`;
- an additional third element;
- malformed first and second time values with field-specific errors;
- propagation of shared decoder depth and element limits;
- redacted diagnostics that contain no source time bytes.

## Deliberate exclusions

- complete X.509 certificate or CRL schemas;
- current-time acquisition or certificate-valid-now policy;
- name constraints or server-identity matching;
- public-key or signature algorithm handling;
- certificate path construction or validation;
- revocation, trust-root sourcing, TLS records, handshakes, sockets, or OAuth
  transport.
