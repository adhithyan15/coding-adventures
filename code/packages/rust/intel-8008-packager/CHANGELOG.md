# Changelog — intel-8008-packager

All notable changes to this crate are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-04-28

### Added

- **`encode_hex(binary, origin) -> Result<String, PackagerError>`**: convert raw
  binary bytes to Intel HEX format.
  - Splits input into records of 16 bytes each (standard "ihex16")
  - Appends mandatory `:00000001FF` EOF record
  - Validates: non-empty binary, origin ≤ 0xFFFF, no 16-bit overflow
- **`decode_hex(text) -> Result<DecodedHex, PackagerError>`**: parse Intel HEX
  string back to `(origin, binary)`.
  - Handles Type 00 (Data) and Type 01 (EOF) records only
  - Verifies per-record checksums
  - Guards against adversarial large-span inputs (cap = 16 KB = 8008 address space)
  - Uses `BTreeMap<usize, Vec<u8>>` for in-order segment assembly
- **`PackagerError`**: public error type implementing `Display` + `Error`
- **`DecodedHex`**: public result struct with `origin: usize` and `binary: Vec<u8>`
- **Zero external dependencies**: hex encoding/decoding implemented with
  `format!("{b:02X}", ...)` and `char::to_digit(16)` — no `hex` crate needed
- **Security hardening in `decode_hex`**:
  - **EOF-record required**: rejects files that end without a type-0x01 sentinel
    (truncated or corrupt files previously returned partial/empty data silently)
  - **Overlap detection (backward)**: uses `BTreeMap::range(..=address).next_back()`
    to detect records that start inside a previously inserted segment
  - **Overlap detection (forward)**: uses `BTreeMap::range((address+1)..).next()` to
    detect out-of-order records that a later-inserted entry would collide with; catches
    the case where records arrive in descending address order
  - **Line-length cap** (`MAX_HEX_LINE_LEN = 1024`): rejects lines longer than 1 024
    chars before any allocation, preventing O(n) char-iterator work on adversarially
    long lines
- **`parse_hex_bytes` zero-allocation rewrite**: direct char-iterator pair loop instead
  of `s.chars().collect::<Vec<char>>()`, keeping per-call allocation bounded to the
  output slice length regardless of input length
- **43 tests** (38 unit + 5 doc-tests), all passing:
  - Checksum: classic example, verification property, EOF record
  - `encode_hex`: single byte, starts with colon, EOF always last, 3-byte format,
    16/17/32 byte splits, address increments, nonzero origin, large address, error cases
  - `decode_hex`: round-trips (1/3/17 bytes, with origin, full 16 KB), all error cases
    (missing colon, invalid hex, bad checksum, unsupported type, truncated record, too
    large, missing EOF, overlapping records, duplicate address, out-of-order overlap,
    line too long)
  - 8008-specific: address near top of address space (0x3FF0)
  - `parse_hex_bytes`: round-trip, odd length error, non-hex error
