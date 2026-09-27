# x509-extension (Python)

`x509-extension` decodes the generic RFC 5280 `Extension` container: an
OBJECT IDENTIFIER, an omitted-default critical flag, and an opaque OCTET
STRING. It composes with `der-asn1`, shares its depth and element budgets, and
deliberately owns no extension registry, certificate policy, or cryptographic
authority.

Successful values are decoder-created and own immutable copies of their OID
encoding and opaque extension bytes.

```python
from der_asn1 import Asn1Decoder
from x509_extension import decode_x509_extension

decoder = Asn1Decoder()
root = decoder.decode_exact(bytes.fromhex("30070603551d110400"))
extension = decode_x509_extension(decoder, root)
assert extension.extension_id.arcs == (2, 5, 29, 17)
assert not extension.critical
```

Failures expose only a stable category, nested ASN.1 category, and local byte
offset. The native test front door consumes all 48 language-neutral X.509
Extension v1 cases.

## Dependencies

- der-asn1
- der-tlv (stable framing error categories)
