# board-vm-protocol (C)

A **host↔board VM wire protocol codec** — pure ISO C17, allocation-free. A
faithful port of the Rust [`board-vm-protocol`](../../rust/board-vm-protocol)
crate.

## What it does

Defines the framing and message payloads a host uses to talk to a tiny "board
VM" (a microcontroller running a bytecode interpreter) over a byte stream such
as a serial line. Three layers stack up:

1. **Message payloads** — `bvm_encode_hello` / `bvm_decode_hello`, plus
   `hello_ack`, `caps_report`, `program_begin`/`chunk`/`end`, `run_request`,
   `run_report_header`, `store_program`, `error_payload`, `ping`/`pong`, and a
   tagged `value`. Each serialises to a compact little-endian payload.
2. **Frames** — `bvm_encode_frame` wraps a payload with a version byte, flags,
   a message-type tag, a request id, a ULEB128 length, and a trailing
   CRC-16/CCITT-FALSE.
3. **Wire frames** — `bvm_encode_wire_frame` COBS-encodes a raw frame and
   appends a `0x00` terminator so frames self-delimit on a raw stream.

Every routine writes into a caller-supplied buffer and returns how many bytes
it produced, or decodes a caller-supplied buffer and hands back *borrowed*
pointers into it — usable on a board with no heap at all.

## API

```c
#include "board_vm_protocol.h"

/* Encode a HELLO payload, wrap it in a frame, COBS-frame it for the wire. */
bvm_hello_t hello = { 1, 1, "bvm", 3, 0x1234ABCDu };
uint8_t payload[16];
size_t n = 0;
if (bvm_encode_hello(&hello, payload, sizeof payload, &n) == BVM_OK) {
    bvm_frame_t frame = { BVM_FLAG_RESPONSE_REQUIRED, BVM_MSG_HELLO,
                          0x1234, payload, n };
    uint8_t raw[32], wire[40];
    size_t wire_len = 0;
    bvm_encode_stream_frame(&frame, raw, sizeof raw, wire, sizeof wire, &wire_len);
    /* wire[..wire_len] is a self-delimited COBS frame ending in 0x00 */
}
```

Every fallible function returns a `bvm_error_t` status code (`BVM_OK == 0` on
success). Decoded strings/byte-slices point into the *input* buffer and are not
NUL-terminated; read exactly the paired `*_len` field.

## Building

```sh
sh BUILD          # POSIX: gcc and/or clang, via the shared iso-harness
```

Each compiler prints `N checks, 0 failed`. Verified clean under ASan + UBSan
and macOS `leaks`; decoders are exercised by a 40k-iteration random/byte-flip
fuzz sweep.
