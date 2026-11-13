# Changelog

## [Unreleased]

Warrant-derived evidence and a jailed runtime gate (LZ01, VER04h, VER08b), and two
findings fixed on the way (SEC-002, SEC-002a, below). The wire format is unchanged:
`compress` output is byte for byte what it was. (Cargo.toml stays at 0.1.0: this whole
entry is unreleased and the version is bumped at release.)

### Fixed

- **SEC-002, decompression bomb (CWE-409).** `decompress` delivers at most
  `MAX_OUTPUT_LEN` (16 MiB, new public constant) bytes and returns `Err` before it
  writes the entry that would pass the limit. Before, a 15 KB stream of chained
  "tricky token" codes expanded to about 40 MB and a 128 KB one to two gigabytes,
  whatever the header declared (it is attacker-controlled). Only *delivered* bytes
  count: a header that declares fewer bytes still stops the decoder early, and a
  stream under the cap decodes whatever its header says (`max_declared_length_on_small_stream`
  still passes). This is the absolute backstop; a `max_output` parameter is the better
  API and a breaking change across every port (backlog LP-003), left to the owner.
- **SEC-002, quadratic dictionary.** Decoder dictionary entries were a `Vec<u8>` each,
  and each entry of a chain is one byte longer than the one before, so `k` codes held
  `k^2 / 2` bytes: a 12 MiB output had a 41 MB peak (3.4x). Entries are now
  `(prefix code, first byte, last byte, length)` links, 8 bytes each (at most 512 KiB for
  the whole dictionary), and a string is written once, into the output, by walking the
  chain from its end. Peak for the same stream: 16.8 MB (1.4x).
- **SEC-002a, quadratic encoder.** `compress` hashed the whole current prefix for every
  input byte (`HashMap<Vec<u8>, u16>`): 8 MiB of zeros took about 35 s in a debug build.
  The dictionary is keyed by `(prefix code, byte)` and each byte costs one lookup. Output
  unchanged (pinned against the old algorithm kept as a test reference).
- **SEC-002a test, runner-speed independent.** `compress_of_redundant_input_is_linear_time`
  asserted `< 10 s` for 8 MiB of zeros; the linear encoder took 12.3 s in a debug build on
  the Windows CI runner. It now times 256 KiB and 4 MiB of zeros (best of 5 and 2) and
  requires the ratio to stay under 32 (linear measures 16, the old prefix-hashing encoder
  54 to 56), so runner speed cancels out. Verified to fail against the encoder from
  `783810d4e89`. No library change; the wire format is unchanged.
- **SEC-002a, wrapping header.** `compress` wrote `data.len() as u32`, so a 4 GiB + 10
  byte input got the header 10 and `decompress` returned ten bytes as a success. The
  header saturates at `u32::MAX`, which `decompress` rejects at the cap.

### Added

- Add required_capabilities.json (pure computation).
- `pub const MAX_OUTPUT_LEN`.
- `code/specs/LZ01-lzw-contract.md` (the package's contract, findings and verification
  status) and `code/specs/lzw.warrant`, the third Warrant v1 document of a real package
  and the first decompressor: the round trip, every `decompress` input of length <= 3
  (16,843,009), hostile streams, the exact wire image of every one- and two-byte input,
  the declared length as an upper bound, `output` and `heap` bound rows with every
  instance observed (the first document to use WX-H), and a real decompression bomb built
  inside the document from a 279-byte unit (512 units decode, 1,024 are rejected).
- `evidence.bindings` (`errors=std::string/String=CORRUPT`: the first binding of a Rust
  `String` error), `tests/warrant_evidence.rs` (shim) and the generated
  `tests/generated/warrant_evidence.rs`; `evidence/test-manifest.json` is written from a
  real run, `evidence/proof-manifest.json` by `warrant-ir --test lzw_proofs` (every
  steps-bound obligation `not-live`: the Rust-subset front end refuses `BitReader<'a>`),
  `effect_certificate.json` and `assurance_report.json` (`--profile lossless-compression`,
  report-only; CWE-409 *discharged* from the bound rows, never excluded).
- `tests/runtime_capability.rs` (VER08b): compress and decompress, including hostile input
  and a 40 MB bomb rejected at the cap, run inside a seccomp/Landlock jail derived from
  `required_capabilities.json` and watched by `strace`. The armed phase is served from the
  `brk` heap (the test warms glibc's mmap threshold in phase A) because the baseline allows
  `mmap`/`munmap` but not `mremap`, which a growing output `Vec` would otherwise call.
- Tests: `tests/decompression_cap.rs`, `tests/decoder_memory.rs` (counting allocator),
  `tests/encoder_cost.rs` and `tests/decoder_equivalence.rs` (the old encoder and decoder kept
  as references and compared byte for byte, and Ok/Err, on seeded garbage), `header_length`
  and mid-entry-truncation unit tests. All written first for SEC-002/SEC-002a and failing
  at the previous commit.
- BUILD (macOS), BUILD_linux (strict jail) and BUILD_windows (no jail), as for wasm-leb128.

### Security

- `decompress` stops expanding codes once the declared `original_length` is reached (each code can emit ~4 KiB, so a small stream could previously expand far beyond it before the final truncate). Valid streams are unchanged.
- Added `tests/hostile_headers.rs` (allocation guard, hostile headers, exhaustive <=2-byte and seeded random round trips).

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZW encoding and decoding (CMP03 spec).
- `BitWriter` and `BitReader` helpers for LSB-first variable-width bit packing.
- `compress` / `decompress` one-shot CMP03 wire-format API.
- Variable-width codes (9–16 bits) with automatic code-size growth.
- Dictionary reset (CLEAR_CODE) when `next_code` reaches 2^16.
- Tricky-token edge case (`code == next_code`) handled in decoder.
- Security: invalid code, missing CLEAR_CODE, and short-header errors returned
  as `Err(String)` rather than panics.
- 30 unit tests covering spec vectors, round-trips, bit I/O, and error paths.
