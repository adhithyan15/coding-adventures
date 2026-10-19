# X.509 DirectoryString

## Status

Implemented as a zero-external-dependency Rust semantic layer above
`der-asn1` and `der-tlv`.

## Purpose

Recognize and validate the bounded value representation of one already framed
RFC 5280 `DirectoryString` without confusing representation decoding with the
much larger distinguished-name matching policy.

The layer recognizes exactly these primitive, non-empty universal choices:

- UTF8String, with strict UTF-8 validation;
- PrintableString, with the ASN.1 PrintableString character repertoire;
- TeletexString, retained as opaque non-empty T.61 bytes;
- UniversalString, as big-endian four-octet Unicode scalar values; and
- BMPString, as big-endian two-octet non-surrogate Unicode scalar values.

UTF8String, PrintableString, UniversalString, and BMPString expose a borrowed,
allocation-free Unicode scalar iterator. TeletexString deliberately does not:
the package has no T.61 mapping table and refuses to reinterpret opaque bytes
as ASCII or Unicode.

## Comparison boundary

`compare_x509_directory_strings_by_scalar` compares the exact Unicode scalar
sequence of any two supported representations and fails closed if either value
is TeletexString. This is a stable equivalence relation for the four supported
representations, but it is not RFC 5280 distinguished-name equality.

In particular, the package does not perform RFC 4518 StringPrep, Unicode case
folding or normalization, insignificant-space handling, domainComponent
matching, or attribute-OID schema selection. A later policy must provide those
rules before this value layer can be used by `x509-name-comparison` for issuer
selection or path validation.

## Bounds and errors

The decoder receives one element already framed by the caller's shared
`Asn1Decoder`. It makes one linear validation pass over the borrowed contents,
then scalar comparison makes one linear pass over each value. No input-sized
allocation occurs. Errors retain only a stable category and element-local byte
offset.

## Deliberate exclusions

- complete T.61 decoding;
- RFC 4518 StringPrep, caseIgnoreMatch, normalization, case folding, or space
  preparation;
- built-in attribute OID registries or schema-specific length constraints;
- complete distinguished-name equality, issuer selection, path construction,
  path validation, or trust-anchor processing;
- signature verification, algorithms, clocks, revocation, hostname
  verification, trust-root loading, TLS, sockets, HTTP, or OAuth transport.
