# board-vm-protocol (C++)

A **host↔board VM wire protocol codec** — header-only, ISO C++17. A faithful
port of the Rust [`board-vm-protocol`](../../rust/board-vm-protocol) crate, in
namespace `ca::board_vm_protocol`.

## What it does

Defines the framing and message payloads a host uses to talk to a tiny "board
VM" (a microcontroller running a bytecode interpreter) over a byte stream such
as a serial line. Three layers stack up:

1. **Message payloads** — `encode_hello` / `decode_hello`, plus `hello_ack`,
   `caps_report`, `program_begin`/`chunk`/`end`, `run_request`,
   `run_report_header`, `store_program`, `error_payload`, `ping`/`pong`, and a
   tagged `Value`. Each serialises to a compact little-endian payload.
2. **Frames** — `encode_frame` wraps a payload with a version byte, flags, a
   message-type tag, a request id, a ULEB128 length, and a trailing
   CRC-16/CCITT-FALSE.
3. **Wire frames** — `encode_wire_frame` COBS-encodes a raw frame and appends a
   `0x00` terminator so frames self-delimit on a raw stream.

## API

```cpp
#include "board_vm_protocol.hpp"
namespace bvm = ca::board_vm_protocol;

bvm::Hello hello;
hello.min_version = 1;
hello.max_version = 1;
hello.host_name = "bvm";
hello.host_nonce = 0x1234ABCDu;

std::vector<std::uint8_t> payload = bvm::encode_hello(hello);

bvm::Frame frame;
frame.flags = bvm::FLAG_RESPONSE_REQUIRED;
frame.message_type = bvm::MessageType::Hello;
frame.request_id = 0x1234;
frame.payload = bvm::ByteView(payload.data(), payload.size());

std::vector<std::uint8_t> wire = bvm::encode_stream_frame(frame); // COBS + 0x00
```

Encoders return `std::vector<std::uint8_t>` (they grow, so the encode path
cannot report `OutputTooSmall`). Decoders take a `ByteView` and return borrowed
views (`std::string_view` for strings, `ByteView` for raw bytes) into the
caller's buffer. Where the Rust crate returns `Result`, this port throws a
`ProtocolError` carrying an `Error` code. RAII throughout.

## Building

```sh
sh BUILD          # POSIX: g++ and/or clang++, via the shared iso-harness
```

Each compiler prints `N checks, 0 failed`. Verified clean under ASan + UBSan;
decoders are exercised by a 40k-iteration random/byte-flip fuzz sweep.
