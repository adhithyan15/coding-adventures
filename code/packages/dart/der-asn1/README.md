# der-asn1 (Dart)

`coding_adventures_der_asn1` provides bounded typed ASN.1 DER value decoding
over the adjacent payload-blind `der-tlv` package. It exposes immutable byte
views, shared depth and element budgets, transactional constructed cursors,
stable payload-free errors, primitive universal values, implicit values, and
explicit wrappers.

The package implements the language-neutral `der-asn1-v1` fixture profile and
also replays every referenced `der-tlv-v1` framing case. It does not parse
X.509 schemas, perform cryptography, or access the host.
