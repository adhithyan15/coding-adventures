# der-asn1 (Python)

`der-asn1` layers bounded ASN.1 value semantics over the payload-blind
`der-tlv` framing package. It validates BOOLEAN, INTEGER, BIT STRING, OCTET
STRING, IA5String, NULL, OBJECT IDENTIFIER, context-specific implicit values,
SEQUENCE, SET, and explicit wrappers while preserving one shared depth and
total-element budget.

```python
from der_asn1 import Asn1Decoder, decode_object_identifier

decoder = Asn1Decoder()
oid = decode_object_identifier(decoder.decode_exact(bytes.fromhex("06032a0304")))
assert oid.arcs == (1, 2, 3, 4)
```

Failures contain only a stable error kind and local byte offset. Framing
failures additionally retain the stable `der-tlv` error kind; none of the
errors interpolate hostile payload bytes.

## Dependencies

- der-tlv

The package consumes all 109 cases in the language-neutral DER ASN.1 v1
fixture. Run the native front door with `BUILD` or `BUILD_windows`.
