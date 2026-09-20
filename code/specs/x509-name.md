# X.509 Name

## Status

Implemented as a zero-external-dependency Rust semantic layer above
`der-asn1`.

## Purpose

Decode the structural RFC 5280 distinguished-name grammar without performing
string normalization or semantic name comparison:

```asn1
Name ::= CHOICE { rdnSequence RDNSequence }
RDNSequence ::= SEQUENCE OF RelativeDistinguishedName
RelativeDistinguishedName ::= SET SIZE (1..MAX) OF AttributeTypeAndValue
AttributeTypeAndValue ::= SEQUENCE {
  type  AttributeType,
  value AttributeValue
}
AttributeType ::= OBJECT IDENTIFIER
AttributeValue ::= ANY -- DEFINED BY AttributeType
```

An empty outer RDN sequence is structurally valid. Every present RDN must be a
non-empty SET whose complete child encodings are in canonical DER order. Each
attribute contains exactly one canonical OID and one already framed opaque DER
value.

## Public contract

```rust
pub fn decode_x509_name<'a>(
    decoder: &mut Asn1Decoder,
    element: Asn1Element<'a>,
) -> Result<X509Name<'a>, X509NameError>;
```

`X509Name` retains exact borrowed DER, RDN boundaries, and at most 32 RDNs and
64 total attributes. Attribute access exposes a validated OID and its exact
typed DER value without copying or interpreting value bytes.

## Limits and diagnostics

All nested elements share the caller's DER framing, depth, total-element, and
OID-arc budgets. Production decoding allocates nothing. Errors contain only a
stable category and Name-local byte offset and never retain or format source
bytes.

## Tests

Tests cover empty and multi-valued names, RDN boundary retention, exact outer
and nested tags, non-empty and sorted RDN sets, exact attribute arity, malformed
child framing, explicit collection bounds, shared depth and element budgets,
and redacted diagnostics.

## Deliberate exclusions

- DirectoryString character-set validation, Unicode preparation, case folding,
  whitespace handling, or RFC 5280 semantic name equality;
- attribute OID registries, schema-specific value interpretation, or display;
- complete certificate parsing or issuer/subject relationship decisions;
- certificate path construction, signature verification, algorithms, clocks,
  revocation, hostname verification, trust-root loading, TLS, sockets, or OAuth
  transport.
