# Changelog

## Unreleased

- Add the first consumer for the 39-case language-neutral WebSocket core
  fixture contract, including exact success, incremental, limit, masking,
  fragmentation, control, UTF-8, and payload-free error behavior.

## 0.1.0

- Add strict bounded RFC 6455 server and client upgrade handshakes.
- Add incremental frame decoding with masking, canonical-length, opcode,
  control-frame, and size validation.
- Add bounded fragmented text/binary message assembly with interleaved control
  events.
- Add validated close payloads and automatic pong/close reply construction.
