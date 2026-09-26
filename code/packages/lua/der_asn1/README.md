# der-asn1 (Lua)

`coding_adventures.der_asn1` is a bounded, payload-blind typed ASN.1 DER layer
above the repository-owned `coding_adventures.der_tlv` framing package. It
validates canonical BOOLEAN, INTEGER, BIT STRING, OCTET STRING, IA5String,
NULL, OBJECT IDENTIFIER, SEQUENCE, SET, and context-specific implicit and
explicit values.

Validated objects keep their trusted state in private weak-key registries.
Decoder owner tokens bind descendants and cursors to one shared transactional
element budget. Unsigned 64-bit INTEGER and OID projections use exact 32-bit
limbs, so Lua's signed host integer cannot round the portable u64 boundary.

The package has an empty capability manifest. It does not parse X.509 schema,
validate paths or signatures, access trust stores, perform TLS, decode PEM, or
use filesystem, network, process, clock, entropy, environment, credential, or
console authority.

## Dependencies

- der-tlv

## Development

```bash
bash BUILD
```

The neutral suite executes all 122 DER ASN.1 cases and all 46 referenced DER
TLV framing cases. LuaCov enforces at least 95 percent production line
coverage.
