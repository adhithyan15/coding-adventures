# x509-rsassa-pss-parameters

Bounded, allocation-free decoding of RFC 4055 / RFC 8017 RSASSA-PSS
parameters carried by an X.509 signature AlgorithmIdentifier.

The decoder requires the `id-RSASSA-PSS` OID and present parameters, handles
the four explicitly tagged fields in canonical order, and applies their SHA-1,
MGF1-SHA-1, 20-octet salt, and trailer-field-1 defaults. As RFC 4055 requires
for signature validation, explicitly present SHA-1, MGF1-SHA-1, and
trailer-field-1 values are recognized as equivalent to omission. An explicit
saltLength of 20 remains rejected under DER DEFAULT omission rules. SHA-1,
SHA-224, SHA-256, SHA-384, and SHA-512 identifiers accept the RFC-compatible
NULL or absent parameter forms.

## Boundary

This package decodes public structure only. It does not approve an algorithm
profile, select an implementation, parse a key, hash, run MGF1, validate PSS
encoding, apply RSAVP1, or establish a signature, certificate, path, trust,
TLS, HTTP, OAuth, or network verdict. It performs no external effect.

## Development

```bash
bash BUILD
```
