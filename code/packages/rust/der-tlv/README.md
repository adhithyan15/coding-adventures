# der-tlv

`der-tlv` is the repository-owned, allocation-free boundary for splitting
ASN.1 Distinguished Encoding Rules (DER) values into identifier, header,
value, and encoded borrowed slices.

```rust
use der_tlv::{decode_exact, DerLimits, TagClass};

let integer = decode_exact(&[0x02, 0x01, 0x2a], DerLimits::default())?;
assert_eq!(integer.tag().class, TagClass::Universal);
assert_eq!(integer.tag().number, 2);
assert_eq!(integer.value(), &[0x2a]);
# Ok::<(), der_tlv::DerError>(())
```

## Security boundary

The decoder accepts only minimally encoded definite-length DER framing. It
rejects BER indefinite lengths and end-of-contents markers, non-minimal or
overflowing tags and lengths, truncated values, trailing bytes in exact mode,
and configured work-limit violations. Decoded elements borrow their input;
declared wire lengths never cause allocation. Errors disclose only a category
and byte offset, never hostile input.

This crate does not interpret ASN.1 types, validate their contents, recurse
through constructed values, parse certificates, verify signatures or paths,
select trust roots, perform TLS, or access the network. Callers that open
constructed values must enforce shared whole-document depth and work budgets.
