# Changelog

## Unreleased

### Added

- The D18K frame protocol between the supervisor and one agent's broker
  (D18S P2.6d-1):
  - Frames to the broker: `Bootstrap`, `Request`, `CallbackResult` and
    `Terminate`.
  - Frames from the broker: `Ready`, `Callback` and `Response`.
  - Eight per-operation storage callbacks, each with its reply.
  - Ten refusal codes.
  - `definition_digest`, which names an exact definition.
- A bounded, total codec, plus `write_frame` and `read_frame` (4-byte
  big-endian length, 1 byte to 1 MiB). Nested records use their owning
  crates' codecs: D18B bindings, D18C definitions, D18M messages, D18H
  headers, D18G grants and data-plane records.
- Payload-blind `Debug` for callbacks and replies.
- Tests:
  - every frame round-trips;
  - every truncation and any trailing byte is refused;
  - a frame sent the wrong way, or with the wrong magic or version, is
    refused;
  - bounds are enforced on encode and on decode (page limit, content type,
    slot count, a master key in `Ready`, page count and page bytes);
  - hostile counts are refused before allocation;
  - framing refuses empty, oversized and truncated frames;
  - debug output carries no payload.
