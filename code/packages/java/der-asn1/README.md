# der-asn1 (Java)

`der-asn1` provides bounded typed ASN.1 DER value decoding over the adjacent
payload-blind Java `der-tlv` package. It exposes read-only byte views, shared
depth and element budgets, transactional constructed cursors, stable
payload-free errors, primitive universal values, implicit values, and explicit
wrappers.

The package implements all 122 cases in the language-neutral `der-asn1-v1`
profile and executes every referenced `der-tlv-v1` framing case. It does not
parse X.509 schemas, perform cryptography, or access the host.
