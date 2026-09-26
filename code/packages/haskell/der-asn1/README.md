# der-asn1

Bounded typed ASN.1 DER value decoding for Haskell. The package implements the
language-neutral `der-asn1-v1` profile on top of `der-tlv`, including canonical
BOOLEAN, INTEGER, BIT STRING, OCTET STRING, IA5String, NULL, and OBJECT
IDENTIFIER values plus SEQUENCE, SET, and explicit-tag traversal.

Every decoder owns an opaque, concurrency-safe shared element budget. Elements
and cursors retain that exact owner, so a fresh same-limit decoder cannot reset
work accounting.
Failed framing and typed-value operations leave both cursor position and budget
unchanged. Constructors for validated elements, integers, bit strings, object
identifiers, and cursors remain private.

## Dependencies

- der-tlv

## Development

```bash
bash BUILD
# Windows
cmd /c BUILD_windows
```

The test suite consumes all 122 closed fixture cases, including the 46 delegated
`der-tlv-v1` framing references and all 22 stable redacted error categories.
