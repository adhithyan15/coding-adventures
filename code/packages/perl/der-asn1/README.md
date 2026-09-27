# DER ASN.1 for Perl

This package implements the language-neutral `der-asn1-v1` contract on top of
`CodingAdventures::DerTlv`. It validates canonical BOOLEAN, INTEGER, BIT STRING,
OCTET STRING, IA5String, NULL, and OBJECT IDENTIFIER values and provides bounded
SEQUENCE, SET, and explicit-tag traversal.

Each decoder owns an exact shared work budget. Validated elements and cursors
retain that unforgeable owner, so a same-limit decoder cannot reset accounting.
Wrapper state lives in private field hashes; caller-created blessed references
cannot become validated values, and returned tags, limits, bytes, and OID arcs
are defensive snapshots. Failed reads leave both work count and cursor position
unchanged. Diagnostics contain stable identifiers and local offsets, never
payload bytes.

Exact unsigned values use core `Math::BigInt`, including the full `u64` range.
The runtime has no ambient-authority capability requirements.

## Dependencies

- `CodingAdventures::DerTlv`
- Perl core modules only (`Math::BigInt`, `Hash::Util::FieldHash`, and
  `Scalar::Util`)

## API

Create a `CodingAdventures::DerAsn1::Decoder`, call `decode_exact`, then use the
exportable typed decoder functions. `sequence`, `set`, and `explicit` preserve
the originating decoder owner and share its total-element budget. Integer,
bit-string, OID, element, cursor, and error objects expose read-only accessors.

## Development

```bash
bash BUILD
```

On Windows, put Strawberry Perl first on `PATH`, then execute each command in
`BUILD_windows`. The tests execute all 122 neutral cases and all 46 referenced
DER TLV cases plus native provenance, budget, transactionality, exact-integer,
snapshot, limit, and redaction checks.
