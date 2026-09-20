# der-tlv

Bounded, payload-blind DER tag-length-value framing for Kotlin/JVM. The package
implements the portable profile in `code/specs/der-tlv.md`, with canonical
identifier and length checks, stable errors and offsets, explicit limits,
array-backed read-only views, exact-one decoding, and an iterative cursor.
Tests consume all cases in `code/specs/fixtures/der-tlv-v1`.
