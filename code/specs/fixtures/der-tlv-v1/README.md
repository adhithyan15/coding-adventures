# DER TLV v1 fixtures

This directory is the language-neutral conformance contract for the bounded
X.690 DER identifier and definite-length framing package.

## Files

- `schema.json` closes the document shape, limits, stable error identifiers,
  input-segment grammar, operations, and normalized results.
- `cases.json` covers identifier classes, constructed bits, tag and length
  canonicality, truncation, configured limits, exact/remainder behavior,
  cursor state, and payload-blind errors.
- `consumers.schema.json` closes the established-lane registry shape and fixes
  the 15-language denominator.
- `consumers.json` binds every established lane to its production source,
  package-native fixture test, build fronts, empty capability manifest, and
  required public surface.
- `CHANGELOG.md` records fixture-contract changes.

Input is an ordered array of bounded segments. A segment is either lowercase,
byte-aligned hexadecimal or one repeated byte plus a bounded count. Consumers
must materialize the segments in memory and must not interpret fixture data as
paths, commands, or code.

The fixture-only `"host-max"` value for `limits.max_value_len` means the
largest value representable by the consumer's native index type. It exists
solely to prove that a wire `u64::MAX` declaration reports the same
`length-host-overflow` kind and length-prefix offset on 32-bit and 64-bit
hosts. It is not a production configuration value.

The fixture result is deliberately smaller than an implementation's public
element object. Byte ranges are derived from `element_offset`, `header_len`,
`encoded_len`, and `remainder_offset`, which lets each package prove that its
header, value, encoded value, and remainder are exact slices of the supplied
input without copying large expected payloads into JSON.

Errors are normalized to one of the 17 closed identifiers plus an input byte
offset. Diagnostic strings are implementation-specific, but must remain
payload-blind. In particular, the `der-tlv-v1-redacted-hostile-input` case must
not cause `deadbeef` or a byte dump to appear in a public error.

This corpus is structural and behavioral evidence, not a substitute for
running each package's real tests, coverage, lint, BUILD, and BUILD_windows
front doors.

`code/scripts/tests/test_der_tlv_portable_coverage.py` is the aggregate closure
gate. It rejects missing, extra, duplicate, traversing, or cross-wired
consumers, requires every registered fixture test to name this exact corpus,
and validates each empty capability manifest. Package-native BUILD execution
provides the runtime proof that every lane consumes all cases.
