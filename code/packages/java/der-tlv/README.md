# der-tlv

Bounded, payload-blind DER tag-length-value framing for Java 21. The package
implements the portable profile in `code/specs/der-tlv.md`, including canonical
identifier and length checks, stable error IDs and offsets, explicit resource
limits, array-backed read-only views, exact-one decoding, and an iterative
sibling cursor. Tests consume all cases in `code/specs/fixtures/der-tlv-v1`.
