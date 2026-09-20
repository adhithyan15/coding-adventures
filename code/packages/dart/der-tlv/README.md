# der-tlv (Dart)

`coding_adventures_der_tlv` provides bounded canonical DER framing with
`Uint8List.sublistView` byte views, BigInt wire-length arithmetic, explicit
limits, stable payload-blind errors, and an iterative sibling cursor. It does
not interpret ASN.1 values, parse X.509, perform cryptography, or access the
host.
