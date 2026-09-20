# CodingAdventures::DerTlv

Bounded, payload-blind DER tag-length-value framing for Perl. The module
implements the portable profile in `code/specs/der-tlv.md`: canonical identifier
and length checks, stable error IDs and offsets, explicit resource limits,
exact-one decoding, and an iterative sibling cursor.

The test suite consumes all cases in `code/specs/fixtures/der-tlv-v1` using
only Perl core modules.
