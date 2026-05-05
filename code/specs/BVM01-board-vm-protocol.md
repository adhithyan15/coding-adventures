# BVM01 - Board VM Binary Protocol

## Overview

The Board VM protocol is the language-neutral binary contract between a host and
a board runtime. It is designed for byte streams such as UART, but it can also
run over packet transports such as BLE, TCP, USB bulk endpoints, or WebSerial.

The protocol has two layers:

1. **Transport framing**: turns unreliable byte streams into bounded frames with
   length, request ids, and CRC protection.
2. **Message payloads**: describe board discovery, capability reports, program
   upload, execution, logging, events, and errors.

The payloads are intentionally small and schema-like. Language SDKs should be
thin wrappers around the same messages, not bespoke remote-control APIs.

## Layer Position

```
Host SDK / CLI / REPL
        |
        v
BVM01 binary protocol
        |
        v
Transport adapter: UART / USB CDC / BLE / TCP / WebSerial
        |
        v
Board VM runtime
        |
        v
Board HAL
```

The protocol depends on BVM02 for bytecode payloads but does not depend on any
specific host language or board.

## Encoding Rules

All multi-byte fixed-width integers are little-endian.

Variable-length unsigned integers use ULEB128:

```
0..127        -> one byte
128..16383    -> two bytes
16384..2^21-1 -> three bytes
```

Strings are encoded as:

```
len: uleb128
bytes: UTF-8, no trailing NUL
```

Byte arrays are encoded as:

```
len: uleb128
bytes
```

Boolean values are one byte:

```
0x00 = false
0x01 = true
```

Unknown enum values must be rejected with `UnsupportedValue`, not silently
coerced.

## Transport Frame

For byte-stream transports, every frame is COBS encoded and terminated with
`0x00`.

```
wire_frame:
  cobs(raw_frame || crc16_le) 0x00

raw_frame:
  version: u8
  flags: u8
  message_type: u8
  request_id: u16
  payload_len: uleb128
  payload: bytes[payload_len]

crc16_le:
  CRC-16/CCITT-FALSE over raw_frame
```

COBS gives the receiver a reliable frame boundary and lets it recover after
garbage bytes. The terminating zero byte is not part of the CRC.

Packet transports may omit COBS if the packet layer already preserves message
boundaries, but they must preserve the same `raw_frame || crc16_le` bytes.

### Version

Version is a single byte for v1:

```
0x01 = Board VM protocol v1
```

Future incompatible revisions use a new version byte. Hosts must fail the
handshake if the board reports no mutually supported version.

### Flags

```
bit 0: response_required
bit 1: is_response
bit 2: is_error_response
bit 3: compressed_payload
bits 4..7: reserved, must be 0
```

Compression is reserved for larger future payloads. v1 senders must not set
`compressed_payload`.

### Request ID

`request_id` correlates responses with requests. The host allocates nonzero
request ids. The board copies the id into the response. `0` is reserved for
unsolicited events and logs.

At most one request may be in flight on boards that do not advertise
`transport.pipelining`. Hosts must assume no pipelining until the capability is
reported.

## Message Types

| Type | Name | Direction | Payload |
|---:|---|---|---|
| `0x01` | `HELLO` | host -> board | `Hello` |
| `0x02` | `HELLO_ACK` | board -> host | `HelloAck` |
| `0x03` | `CAPS_QUERY` | host -> board | empty |
| `0x04` | `CAPS_REPORT` | board -> host | `CapsReport` |
| `0x05` | `PROGRAM_BEGIN` | host -> board | `ProgramBegin` |
| `0x06` | `PROGRAM_CHUNK` | host -> board | `ProgramChunk` |
| `0x07` | `PROGRAM_END` | host -> board | `ProgramEnd` |
| `0x08` | `RUN` | host -> board | `RunRequest` |
| `0x09` | `RUN_REPORT` | board -> host | `RunReport` |
| `0x0A` | `STOP` | host -> board | empty |
| `0x0B` | `RESET_VM` | host -> board | empty |
| `0x0C` | `STORE_PROGRAM` | host -> board | `StoreProgram` |
| `0x0D` | `RUN_STORED` | host -> board | `RunStored` |
| `0x0E` | `READ_STATE` | host -> board | `ReadState` |
| `0x0F` | `STATE_REPORT` | board -> host | `StateReport` |
| `0x10` | `SUBSCRIBE` | host -> board | `Subscribe` |
| `0x11` | `EVENT` | board -> host | `Event` |
| `0x12` | `LOG` | board -> host | `Log` |
| `0x13` | `ERROR` | board -> host | `ErrorPayload` |
| `0x14` | `PING` | either | `Ping` |
| `0x15` | `PONG` | either | `Pong` |

Message types `0x80..0xFF` are board-vendor extensions. A portable host SDK
must not require them for core behavior.

## Payloads

### `Hello`

```
min_version: u8
max_version: u8
host_name: string
host_nonce: u32
```

The board replies with the selected version and echoes the nonce.

### `HelloAck`

```
selected_version: u8
board_name: string
runtime_name: string
host_nonce: u32
board_nonce: u32
max_frame_payload: u16
```

### `CapsReport`

```
board_id: string
runtime_id: string
max_program_bytes: u32
max_stack_values: u8
max_handles: u8
supports_store_program: bool
capability_count: uleb128
capabilities: CapabilityDescriptor[capability_count]
```

`CapabilityDescriptor`:

```
id: u16
version: u8
flags: u16
name: string
```

Capability reports include both bytecode-callable operations and protocol-level
runtime features. The `flags` field identifies which kind a descriptor is:

```
bit 0: bytecode_callable
bit 1: protocol_feature
bit 2: board_metadata
bits 3..15: reserved
```

Portable bytecode-callable ids below `0x1000` are assigned by BVM02. Portable
protocol feature ids use `0x7000..0x7FFF`. IDs at or above `0x8000` are
board-vendor extensions.

Initial protocol feature ids:

| ID | Name | Meaning |
|---:|---|---|
| `0x7001` | `program.ram_exec` | upload and run volatile bytecode programs |
| `0x7002` | `program.store` | store bytecode in board nonvolatile memory |
| `0x7003` | `transport.pipelining` | more than one request may be in flight |

### `ProgramBegin`

```
program_id: u16
format: u8        # 0x01 = BVM bytecode module
total_len: u32
program_crc32: u32
```

The board allocates or clears a temporary upload buffer for `program_id`.

### `ProgramChunk`

```
program_id: u16
offset: u32
bytes: byte_array
```

Chunks may be retransmitted. The board must treat a retransmit of identical
bytes at the same offset as success.

### `ProgramEnd`

```
program_id: u16
```

The board validates length, CRC, and bytecode structure. A successful response
means the program is ready to run from volatile storage.

### `RunRequest`

```
program_id: u16
flags: u8
instruction_budget: u32
time_budget_ms: u32
```

Flags:

```
bit 0: reset_vm_before_run
bit 1: keep_handles_after_run
bit 2: background_run
bits 3..7: reserved
```

Interactive commands normally set `reset_vm_before_run`. Long-running stored
programs normally use `background_run`.

### `RunReport`

```
program_id: u16
status: u8
instructions_executed: u32
elapsed_ms: u32
stack_depth: u8
open_handles: u8
return_count: uleb128
returns: Value[return_count]
```

Status:

| Value | Name | Meaning |
|---:|---|---|
| `0x00` | `halted` | Program reached `HALT` |
| `0x01` | `running` | Program is still running in background |
| `0x02` | `stopped` | Program stopped by host |
| `0x03` | `budget_exceeded` | Instruction or time budget expired |
| `0x04` | `faulted` | Program halted with an error |

### `StoreProgram`

```
program_id: u16
slot: u8
boot_policy: u8
```

Boot policy:

```
0x00 = store only, do not run at boot
0x01 = run at boot after runtime initialization
0x02 = run at boot only if no host connects during grace period
```

### `ErrorPayload`

```
code: u16
request_id: u16
program_id: u16
bytecode_offset: u32
message: string
```

If no program or bytecode offset applies, `program_id` is `0xFFFF` and
`bytecode_offset` is `0xFFFFFFFF`.

## Value Encoding

`Value` is used in run returns, state reports, and events.

```
tag: u8
payload: depends on tag
```

| Tag | Type | Payload |
|---:|---|---|
| `0x00` | `unit` | empty |
| `0x01` | `bool` | u8 |
| `0x02` | `u8` | u8 |
| `0x03` | `u16` | u16 |
| `0x04` | `u32` | u32 |
| `0x05` | `i16` | i16 |
| `0x06` | `handle` | u16 |
| `0x07` | `bytes` | byte_array |
| `0x08` | `string` | string |

The first MVP should only require `unit`, `bool`, `u8`, `u16`, and `handle`.

## Error Codes

| Code | Name |
|---:|---|
| `0x0001` | `InvalidFrame` |
| `0x0002` | `UnsupportedVersion` |
| `0x0003` | `UnsupportedMessage` |
| `0x0004` | `PayloadTooLarge` |
| `0x0005` | `BadCrc` |
| `0x0100` | `UnsupportedCapability` |
| `0x0101` | `ResourceBusy` |
| `0x0102` | `HandleNotFound` |
| `0x0200` | `InvalidProgram` |
| `0x0201` | `InvalidBytecode` |
| `0x0202` | `StackOverflow` |
| `0x0203` | `StackUnderflow` |
| `0x0204` | `BudgetExceeded` |
| `0x0300` | `StorageUnavailable` |
| `0x0301` | `StorageFull` |
| `0x0400` | `BoardFault` |

## Flow Control

The board advertises `max_frame_payload` during `HELLO_ACK`. The host must keep
encoded payloads at or below that value. For larger bytecode modules, the host
uses `PROGRAM_CHUNK`.

If the board is busy running a foreground program, it may return `BoardBusy`.
If the board is running a background program, it must still accept `STOP`,
`PING`, and `READ_STATE` unless it has faulted.

## Test Strategy

- Encode/decode golden vectors for every payload.
- CRC failure test with one flipped payload bit.
- COBS recovery test with garbage bytes before a valid frame.
- Request id round-trip tests.
- Max payload boundary tests.
- Retransmitted `PROGRAM_CHUNK` tests.
- Unknown message and unknown enum tests.
- Cross-language conformance: every SDK encodes the same payload to identical
  bytes before COBS and CRC.

## Future Extensions

- Authenticated sessions for wireless transports.
- Optional payload compression for large constant tables.
- Multiplexed request streams for boards with RTOS support.
- Binary trace streaming for instruction-level debugging.
- Negotiated vendor extension schemas.
