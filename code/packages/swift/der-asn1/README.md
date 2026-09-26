# der-asn1

Bounded, payload-blind ASN.1 DER value decoding for Swift. The package layers
typed BOOLEAN, INTEGER, BIT STRING, OCTET STRING, NULL, IA5String, object
identifier, constructed-container, and explicit-tag decoding over `der-tlv`.
All aggregate work is governed by depth, element, and OID-arc limits.

## Dependencies

- der-tlv

## Development

```bash
# Run tests
bash BUILD
```
