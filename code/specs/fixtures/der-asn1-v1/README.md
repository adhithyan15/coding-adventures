# Typed DER value v1 fixtures

This directory is the language-neutral behavior contract for the bounded typed
ASN.1 layer above the repository's DER TLV framing package.

## Composition

The corpus does not copy framing vectors. Its 46 `decode-exact` delegation rows
name exact `decode-exact` cases in `../der-tlv-v1/cases.json`. The contract
validator resolves those references, rejects missing, duplicate, cyclic, or
non-exact references, and requires the typed consumer to project the same raw
element or framing failure. Changes to canonical identifier, length, limit, or
trailing-data behavior therefore remain single-owned by DER TLV.

The remaining cases own typed semantics: BOOLEAN, INTEGER and checked `u64`
conversion, BIT STRING, OCTET STRING, IA5String, NULL, OBJECT IDENTIFIER,
schema-selected primitive context-specific values, SEQUENCE, SET, explicit
wrappers, shared depth and element budgets, cursor transactionality, local
offset domains, and payload-blind errors.

## Portable projections

Expected results describe values rather than runtime object identity. Byte
projections use lowercase hexadecimal. Integers preserve their signed DER
contents and use decimal strings for `u64` values. OID arcs are decimal strings
so JavaScript, Dart, JSON, and other numeric runtimes cannot silently round
values beyond their exact integer domain. The contract requires byte equality,
not a particular borrowed-view or copying strategy.

Every error names an `offset_scope`. `operation-input` starts at the outer
element's identifier. `container-value` starts at the first contents octet of a
SEQUENCE, SET, or explicit wrapper. A framing failure uses the typed
`framing` identifier plus one of DER TLV's 17 exact framing identifiers. Public
diagnostics may contain the stable kind and numeric offset but never hostile
payload bytes.

## Limits and exclusions

The defaults are 32 levels including the root, 16,384 total successfully read
elements, 128 OID arcs, and DER TLV's v1 framing defaults. Failures consume no
element budget and do not advance a cursor.

The portable corpus does not require a host-size BIT STRING overflow vector:
the bounded fixture cannot materialize that input safely on every supported
runtime. Implementations retain the stable `bit-length-overflow` category and
package-native tests may exercise it where practical. SET ordering, BER/CER,
allocation identity, certificate schemas, cryptography, trust, TLS, transport,
credentials, and ambient authority remain outside this contract.

