# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17, allocation-free port of the Rust `board-vm-protocol` crate: a
  host↔board VM wire protocol codec.
- Three layers — per-message payload codecs (`hello`, `hello_ack`,
  `capability_descriptor`, `caps_report`, `program_begin`/`chunk`/`end`,
  `run_request`, `run_report_header`, `store_program`, `error_payload`,
  `ping`/`pong`, and a tagged `value`), frames (version + flags + message type
  + request id + ULEB128 length + trailing CRC-16/CCITT-FALSE), and COBS wire
  frames with a `0x00` terminator.
- Public `bvm_encoder_t` / `bvm_decoder_t` for streaming/advanced use, a
  `crc16_ccitt_false` primitive, and standalone `cobs_encode`/`cobs_decode`.
- Faithful bounds/overflow discipline: every read/write is overflow-checked,
  ULEB128 rejects overflow and truncation, decoded strings are UTF-8-validated,
  and reserved frame/run flags are rejected.
- Fallible routines return a `bvm_error_t` status code (`BVM_OK == 0`), mirroring
  the Rust `ProtocolError` variants.
- 117 checks mirroring the crate's unit tests (golden vectors, CRC check value,
  ULEB128 boundaries, frame/wire round-trips, error paths) plus a
  40k-iteration random/byte-flip decoder fuzz sweep, run under every ISO C
  compiler via the shared `iso-harness`. Verified clean under ASan + UBSan and
  macOS `leaks`.
