# X.509 Name Comparison

## Status

Specified for a zero-external-dependency Rust semantic layer above
`x509-name`.

## Purpose

Compare the bounded structure of two already decoded RFC 5280 distinguished
names while leaving attribute-value semantics data driven. The comparison
owns only these rules:

- the RDN sequences have the same length and are compared in sequence order;
- corresponding RDN SETs have the same number of attributes;
- attributes within one RDN are matched without depending on their DER SET
  order;
- matched attributes have the same canonical object identifier; and
- one caller-supplied policy accepts their already framed DER values.

This split lets later DirectoryString and schema-specific policies compare
different permitted encodings without teaching the structural layer an
algorithm registry or silently treating exact DER equality as complete
distinguished-name equality.

## Public contract

```rust
pub fn compare_x509_names_by(
    left: &X509Name<'_>,
    right: &X509Name<'_>,
    values_equal: impl FnMut(
        X509AttributeTypeAndValue<'_>,
        X509AttributeTypeAndValue<'_>,
    ) -> bool,
) -> bool;
```

The callback is invoked only for unmatched attributes in corresponding RDNs
whose validated attribute-type OIDs are equal. It may be invoked more than
once for one left attribute when an RDN repeats an attribute type. Callers must
supply a stable equivalence relation; stateful, directional, or
non-transitive callbacks do not define a valid name-comparison policy.

The function performs no allocation. It uses the existing bounds of at most
32 RDNs and 64 total attributes and a fixed-width matched-attribute bitmap.
It returns `false` on any sequence, RDN-size, OID, or caller-value mismatch.
It returns no partial-match evidence and retains no input bytes.

## Tests

Tests cover:

- equal single- and multi-RDN names;
- RDN sequence order and RDN-size mismatches;
- OID mismatch without invoking the value policy;
- multi-valued RDN equality when equivalent values use encodings whose
  canonical DER ordering differs;
- repeated attribute types and fail-closed unmatched values; and
- exact callback scope and bounded comparison behavior.

## Deliberate exclusions

- DirectoryString decoding, Unicode normalization or preparation, case
  folding, whitespace handling, or embedded attribute schemas;
- a built-in exact-DER policy represented as semantic name equality;
- certificate issuer selection, path construction, path validation, name
  constraints, or trust-anchor processing;
- signature verification, algorithms, clocks, revocation, hostname
  verification, trust-root loading, TLS, sockets, HTTP, or OAuth transport.

