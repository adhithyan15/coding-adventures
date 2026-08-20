# ZIP raw RFC 1951 v1 fixtures

This directory is the language-neutral contract for raw DEFLATE and CRC-32
owned by every established ZIP package under CMP09.

## Files

- `schema.json` closes the document shape, operations, limits, and stable error
  identifiers.
- `cases.json` contains stored, fixed, dynamic, multi-block, foreign-encoder,
  counted-consumption, malformed-stream, output-cap, encoder-interoperability,
  and incremental CRC-32 vectors.
- `consumers.schema.json` closes the established-lane adoption record.
- `consumers.json` binds every one of the 15 established ZIP roots to its
  production API, neutral-corpus test, BUILD front doors, and empty capability
  manifest.

## Operations

- `inflate` decodes `input_hex`, applies `max_output` when present, and compares
  both the bytes and `bytes_consumed`.
- `inflate-error` requires the exact stable `error_id` and no partial output.
- `deflate-interoperability` encodes `input_hex`; encoder bytes are deliberately
  not pinned, so an independent raw RFC 1951 decoder must recover `expected`.
- `crc32` applies the optional initial value and then every chunk in order.

Expected bytes are either `{ "hex": "..." }` or a compact repeated-byte form,
`{ "repeat_hex": "41", "count": 260 }`. Hex is lowercase and byte-aligned.

## Oracle boundary

`python-zlib` cases are independently decoded with Python's standard `zlib`
module. The one `rfc1951-hdist-32-zero-slots` case records an intentional oracle
exception: RFC 1951 section 3.2.7 permits `HDIST + 1` through 32 and reserves
distance symbols 30 and 31 only from actual use, while a default zlib build
rejects a header advertising those zero-length slots. Consumers must accept the
standards-conforming empty stream and must still reject either reserved symbol
if compressed data decodes it.

## Security invariants

- Production consumers are pure in-memory byte transforms. They do not read the
  filesystem, environment, clock, entropy, network, processes, or credentials.
- The output cap is a byte count, is validated before allocation, and is
  checked before append or copy. The hard ceiling is 256 MiB.
- Counted decoding excludes whole trailing bytes so containers can reject
  covert cavities.
- Error identifiers and messages are stable and payload-blind.
- CRC-32 detects accidental corruption; it is not authentication.

Run the independent fixture validation from the repository root:

```bash
python -m pytest code/scripts/tests/test_zip_raw_rfc1951_fixtures.py -q
```

Run the portable adoption gate from the repository root:

```bash
python -m pytest code/scripts/tests/test_zip_raw_rfc1951_portable_coverage.py -q
```

The adoption gate is structural evidence, not a substitute for executing the
real package front doors. It pins the established denominator to the parity
reporter, rejects path traversal and cross-wired records, verifies all 34 cases
and 14 error identifiers remain shared, and requires every production package
to declare an empty host-capability profile.
