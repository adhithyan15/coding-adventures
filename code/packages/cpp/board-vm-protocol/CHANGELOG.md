# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Header-only, ISO C++17 port of the Rust `board-vm-protocol` crate in
  namespace `ca::board_vm_protocol`: a host↔board VM wire protocol codec.
- Three layers — per-message payload codecs (`hello`, `hello_ack`,
  `capability_descriptor`, `caps_report`, `program_begin`/`chunk`/`end`,
  `run_request`, `run_report_header`, `store_program`, `error_payload`,
  `ping`/`pong`, and a tagged `Value`), frames (version + flags + message type
  + request id + ULEB128 length + trailing CRC-16/CCITT-FALSE), and COBS wire
  frames with a `0x00` terminator.
- Public `Encoder` / `Decoder` classes for streaming/advanced use, a
  `crc16_ccitt_false` primitive, and standalone `cobs_encode`/`cobs_decode`.
  Encoders return growable `std::vector<std::uint8_t>`; decoders return borrowed
  `std::string_view` / `ByteView` into the caller's buffer.
- Faithful bounds/overflow discipline: overflow-checked reads/writes, ULEB128
  overflow/truncation rejection, UTF-8 validation of decoded strings, and
  rejection of reserved frame/run flags. Where Rust returns `Result`, this port
  throws a `ProtocolError` carrying an `Error` code.
- 59 checks mirroring the crate's unit tests (golden vectors, CRC check value,
  ULEB128 boundaries, frame/wire round-trips, error paths) plus a
  40k-iteration random/byte-flip decoder fuzz sweep, run under every ISO C++
  compiler via the shared `iso-harness`. Verified clean under ASan + UBSan.
