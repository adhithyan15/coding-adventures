# der-asn1 (Go)

`derasn1` layers bounded ASN.1 value semantics over the payload-blind
`dertlv` framing package. It validates BOOLEAN, INTEGER, BIT STRING, OCTET
STRING, IA5String, NULL, OBJECT IDENTIFIER, context-specific implicit values,
SEQUENCE, SET, and explicit wrappers while preserving one shared depth and
total-element budget.

Errors expose only a stable kind and local offset. Framing failures retain the
stable upstream DER error kind without interpolating hostile payload bytes.

## Dependencies

- der-tlv

The package-native suite executes all 122 language-neutral DER ASN.1 v1 cases,
including the 46 referenced DER TLV cases. Run `BUILD` for race, coverage, vet,
and trimpath build validation.
