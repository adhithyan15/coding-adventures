# der-tlv

Bounded, payload-blind DER tag-length-value framing for Swift. The package
implements the portable profile in `code/specs/der-tlv.md`, including canonical
identifier and length checks, stable errors and zero-based offsets, explicit
resource limits, slice-backed frame ranges, exact-one decoding, and an
iterative transactional cursor. Tests consume all 54 cases in the shared
`code/specs/fixtures/der-tlv-v1` corpus.
