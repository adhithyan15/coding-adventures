# SHA-512 portable conformance corpus v1

`cases.txt` is a UTF-8, LF-delimited, pipe-separated corpus. Blank lines and
lines beginning with `#` are ignored. Fields never contain whitespace or the
separator.

Two row types are defined:

```text
V|id|input_hex|chunk_sizes_csv|expected_digest_hex
L|id|byte_length_high_u64_hex|byte_length_low_u64_hex|expected_bit_length_hex_or_ERROR
```

`V` inputs and digests use lowercase hexadecimal with an even number of
digits. The empty input and empty chunk plan are represented by empty fields.
A non-empty chunk plan contains positive decimal integers whose sum equals the
decoded input length.

`L` rows model an unsigned 128-bit byte count as two fixed-width 64-bit words.
The expected value is its value multiplied by eight and encoded as exactly 16
big-endian octets. `ERROR` marks a byte count outside FIPS 180-4's
`message_bits < 2^128` domain.

Readers must apply HF04's row, file, decoded-input, digest, and chunk limits
before performing unbounded allocation. The checked-in file is intentionally
small (nine digest vectors and six length vectors) and has no executable or
authority-bearing content.

The digest witnesses were independently checked with Python's `hashlib.sha512`
while authoring the corpus; consumers must compare against the checked-in
values rather than regenerating them.
