# coding-adventures-der-tlv

Bounded, payload-blind DER tag-length-value framing for Lua 5.4. The module
implements the portable profile in `code/specs/der-tlv.md`, including canonical
identifier and length checks, stable zero-based error offsets, explicit resource
limits, exact-one decoding, and an iterative sibling cursor.

The test suite consumes all cases in `code/specs/fixtures/der-tlv-v1`.
