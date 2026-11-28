# TLS `HkdfLabel` encoding contract

This contract specifies the existing Rust `tls13-hkdf-label::HkdfLabel`
public framing value and its `encode` operation. The normative baseline is
[RFC 9846 §7.1](https://www.rfc-editor.org/rfc/rfc9846.html#section-7.1),
specifically the presentation syntax `struct { uint16 length = Length;
opaque label<7..255> = "tls13 " + Label; opaque context<0..255> = Context; }
HkdfLabel;`. The Rust crate is an implementation target, not the source of
the contract.

## Domain and mathematical model

Inputs are a `u16` output length `L`, caller label byte string `A`, context
byte string `C`, and mutable output byte slice `O`. The constructor accepts
exactly `1 <= |A| <= 249` and `0 <= |C| <= 255`. Label bytes are opaque and need not be ASCII. Every `u16` value of `L`, including 0 and 65535, is valid
for this framing value. This two-byte wire-field domain is independent of the
HKDF-Expand limit `255 * HashLen`. That limit belongs to expansion and sits
outside this contract.

Let `P` be the six bytes `74 6c 73 31 33 20` (`"tls13 "`). A valid value's
encoding is the concatenation

`BE16(L) || U8(6 + |A|) || P || A || U8(|C|) || C`.

The prefixed label length is 7 through 255 and the complete encoding length
is `10 + |A| + |C|`, ranging from 11 through 514 bytes. The 514-byte maximum
comprises a two-byte length, one-byte label length, 255-byte prefixed label,
one-byte context length, and 255-byte context.

The constructor checks label conditions before context length. An empty label
yields `EmptyLabel`, a label longer than 249 yields `LabelTooLong`, and a
context longer than 255 yields `ContextTooLong`. Otherwise it returns the
value. For a valid value, `encoded_len()` is the exact required capacity.
`encode(O)` first checks `|O| >= encoded_len`. Insufficient capacity yields
`OutputTooSmall` and leaves every byte of `O` unchanged. Success returns the
encoded prefix, writes that prefix according to the model above, and leaves any
tail bytes unchanged. No other errors are in this operation contract.

## Public surface and boundary

This contract covers `HkdfLabel::new`, `output_length`, `label`, `context`,
`encoded_len`, and `encode`, including their current stable error categories.
It does not specify `hkdf_expand_label_fixed`,
`derive_secret_from_transcript_hash_fixed`, HKDF, transcript hashing, secret
handling, or any decode/round-trip API.
It adds no decoder, API, or replacement encoder. The input fields are public
metadata; no secret is accepted by this framing operation.

## Evidence requirements for the follow-up corpus child

The corpus/evidence child must record RFC 9846 edition and §7.1 provenance for
each case and derive expected bytes with an independent candidate oracle that
implements the concatenation model above, not by calling this Rust encoder.
Include empty and 1-byte labels, lengths 248/249/250, contexts 0/1/254/255/256,
output lengths 0, `0x1234`, and 65535, non-ASCII opaque label bytes, exact
expected encodings, and output capacities one byte short, exact, and with sentinel tail bytes.
Include competing-invalid-input cases to pin constructor error precedence,
and sentinel-buffer checks for both failure non-mutation and success tail
non-mutation. Mutation controls must corrupt each field, include a wrong-endianness encoding
for `0x1234`, corrupt lengths or the prefix, and truncate or extend bytes.
Demonstrate oracle or runner detection for each mutation. Corpus cases
are finite evidence for their listed inputs; this document establishes no
proof, exhaustive run, CI corpus support, or tooling capability.

## Non-goals

This contract does not claim conformance of a TLS endpoint, correctness of
HKDF, compliance with RFC 9846 §9, constant-time behavior, secret erasure,
interoperability, or a proof result. It does not require a decoder or infer
round-trip properties from the encoding model.
