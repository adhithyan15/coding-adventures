# coding_adventures_der_tlv

Bounded, payload-blind DER tag-length-value framing for Ruby. The package
implements the portable profile in `code/specs/der-tlv.md`, including canonical
identifier and length checks, stable error IDs and offsets, explicit resource
limits, exact-one decoding, and an iterative sibling cursor.

The shared language-neutral fixture corpus lives in
`code/specs/fixtures/der-tlv-v1`.

```ruby
element = CodingAdventures::DerTlv.decode_exact("\x04\x01\x2a".b)
element.tag.number # => 4
element.value      # => "\x2a".b
```
