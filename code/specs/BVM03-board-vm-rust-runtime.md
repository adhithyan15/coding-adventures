# BVM03 - Board VM Rust Runtime

## Overview

The Board VM Rust runtime is the firmware-side implementation of the protocol
and bytecode interpreter. It is written as `no_std` Rust so the same core can
run on constrained microcontrollers and richer embedded platforms.

The runtime owns:

- protocol frame receive/send,
- bytecode upload buffers,
- bytecode validation,
- the interpreter loop,
- resource handle tables,
- dispatch from portable capability ids to board HAL traits,
- optional stored-program boot.

Board-specific crates provide the actual HAL implementation and flashing/eject
packaging.

## Layer Position

```
BVM01 protocol frames
        |
        v
board-vm-runtime
        |
        +--> BVM02 bytecode decoder/interpreter
        +--> handle table
        +--> capability dispatch
        |
        v
Board HAL adapter
        |
        v
Physical board peripherals
```

## Runtime Crates

| Crate | Purpose |
|---|---|
| `board-vm-protocol` | frame parser, payload encoder/decoder |
| `board-vm-ir` | bytecode module parser, decoder, validator |
| `board-vm-runtime` | VM state, run loop, capability dispatch |
| `board-vm-uno-r4` | Arduino Uno R4 / Renesas RA4M1 adapter |
| `board-vm-avr` | AVR/ATmega/ATtiny shared target support |
| `board-vm-arduino-uno-r3` | Classic Arduino Uno/Nano style ATmega328P adapter |
| `board-vm-arduino-mega-2560` | ATmega2560 adapter |
| `board-vm-mcs51` | 8051/MCS-51 shared target support |
| `board-vm-at89` | Atmel/Microchip AT89 8051-compatible adapter |
| `board-vm-rp2040` | RP2040 adapter |
| `board-vm-rp2350` | RP2350 Arm/RISC-V adapter |
| `board-vm-esp32` | ESP32 adapter |
| `board-vm-stm32-*` | STM32 family adapters |
| `board-vm-sam-*` | Atmel/Microchip SAM Arm Cortex-M adapters |
| `board-vm-mbed-*` | MBed-style board adapters |
| `board-vm-rust-backend-*` | Rust compiler/codegen support for targets without usable Rust support |

If classic AVR, ATtiny, or 8051 targets are awkward for Rust or too constrained
for the first vertical slice, the architecture still supports them. Missing Rust
compiler support is a backend task, not an exclusion criterion. The first
implemented board may be whichever hardware and Rust target are practical, but
the runtime cannot rely on assumptions that make `tiny` targets impossible.

See `BVM06-board-target-matrix.md` for target families, runtime profiles, and
backend policy.

## Memory Model

The runtime must run without heap allocation by default. Board ports choose
static buffer sizes at compile time:

```rust
pub struct RuntimeConfig {
    pub max_frame_payload: usize,
    pub max_program_bytes: usize,
    pub max_stack_values: usize,
    pub max_handles: usize,
    pub max_log_bytes: usize,
}
```

The runtime stores:

```
rx_frame_buffer: [u8; max_frame_payload]
tx_frame_buffer: [u8; max_frame_payload]
program_buffer: [u8; max_program_bytes]
stack: [Value; max_stack_values]
handles: [HandleSlot; max_handles]
```

Boards with a heap may opt into dynamic buffers later, but tests must cover the
fixed-buffer path.

## HAL Traits

The runtime should depend on narrow traits rather than a specific embedded HAL.

```rust
pub trait Transport {
    fn read_byte(&mut self) -> nb::Result<u8, TransportError>;
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), TransportError>;
}

pub trait Gpio {
    fn open(&mut self, pin: PinId, mode: GpioMode) -> VmResult<HandleToken>;
    fn write(&mut self, token: HandleToken, level: Level) -> VmResult<()>;
    fn read(&mut self, token: HandleToken) -> VmResult<Level>;
    fn close(&mut self, token: HandleToken) -> VmResult<()>;
}

pub trait Clock {
    fn now_ms(&self) -> u32;
    fn sleep_ms(&mut self, ms: u16) -> VmResult<()>;
}

pub trait ProgramStore {
    fn is_available(&self) -> bool;
    fn write_slot(&mut self, slot: u8, module: &[u8]) -> VmResult<()>;
    fn read_slot(&mut self, slot: u8, out: &mut [u8]) -> VmResult<usize>;
    fn boot_slot(&self) -> Option<u8>;
}
```

The current no-heap Rust HAL exposes this first tranche as
`BoardHal::store_program(program_id, slot, boot_policy, module)`. Board adapters
may lower that protocol-level request into their existing bounded storage writes,
while leaving language frontends as thin builders for the `STORE_PROGRAM` frame.

The first network tranches keep the same no-heap shape: `network.tcp.open` and
`network.udp.open` accept an interface id, an IPv4 address encoded as a `u32`,
and a remote port, then return a portable socket handle. `network.tcp.write`,
`network.tcp.read`, `network.tcp.close`, `network.udp.write`,
`network.udp.read`, `network.udp.available`, and `network.udp.close` operate
on those handles and delegate to narrow `BoardHal::network_tcp_*` and
`BoardHal::network_udp_*` hooks. Host SDKs build bytecode for these calls;
board ports decide when a concrete backend can actually drive a WiFi or
Ethernet module.
`network.tcp.connected` is the first socket-control follow-up: it accepts an
existing TCP handle and returns a boolean connection status without consuming
that handle. `network.tcp.available` extends the same bounded control shape for
read readiness: it accepts an existing TCP handle and returns whether the
backend has at least one byte ready to read without consuming the handle.
`network.udp.available` mirrors that bounded control shape for UDP: it accepts
an existing UDP handle and returns whether the backend has a packet or byte
ready to read without consuming the handle.
`network.udp.write_bytes` and `network.udp.read_bytes` add bounded UDP payload
I/O over that same persistent handle shape. The write operation accepts a UDP
handle plus a `ByteBuffer` and returns unit after delegating to
`BoardHal::network_udp_write_bytes`; the read operation accepts a UDP handle
plus a requested byte count and returns a `ByteBuffer` from
`BoardHal::network_udp_read_bytes`. These operations are the transport-side
counterpart to the DNS query/response message payload tranches, without moving
DNS policy or message parsing into language frontends.

WiFi association control uses the same bounded data path as storage and bus
transfers. `network.wifi.associate` accepts an interface id, SSID bytes, and
passphrase bytes, then returns a backend-defined status byte. `network.wifi.status`
returns that status byte without changing the association, while
`network.wifi.disconnect` drops the association for the selected interface.
The first tranche intentionally keeps credentials in `ByteBuffer` operands so
language frontends stay thin over Rust-owned bytecode assembly and board ports
can later decide whether to swap in a credential-store handle.

DNS resolution is the first hostname-oriented network tranche. `network.dns.resolve`
accepts an interface id plus hostname bytes and returns an IPv4 address encoded
as a `u32`. The VM keeps the operation transport-neutral and delegates the
actual resolver policy to the board adapter through `BoardHal::network_dns_resolve`,
so frontends can keep emitting bounded bytecode while board ports decide whether
to use WiFi firmware DNS helpers, a later DNS message stack, or cached entries.
`network.dns.set_server` is the first DNS client-policy follow-up: it accepts
an interface id plus a resolver IPv4 address encoded as a `u32`, then delegates
to `BoardHal::network_dns_set_server` without changing the `network.dns.resolve`
operand shape. Board ports may map it to firmware resolver configuration,
ignore it when the active link owns resolver policy, or cache it for a later
bounded DNS message stack.
`network.dns.query` is the first bounded DNS message-payload tranche: it accepts
a DNS transaction id plus hostname bytes and returns a standard recursive
A-record query message in a `ByteBuffer`. It is implemented in the Rust runtime
instead of a board adapter hook so frontends and board ports can share the same
wire payload before later tranches add UDP transport and response parsing.
`network.dns.response_ipv4` is the matching bounded response-payload tranche:
it accepts the expected transaction id plus DNS response bytes, validates the
response envelope, skips question records, and returns the first IPv4 A-answer
as a `u32`. It remains a Rust-runtime operation so UDP transport can be layered
later without duplicating DNS message parsing across language frontends or board
ports.
`network.dns.exchange_udp` is the first bounded DNS-over-UDP transport tranche:
it accepts an interface id, resolver IPv4 address, transaction id, hostname
bytes, and requested response length, builds the standard A-record query in the
Rust runtime, opens UDP port 53 through the board adapter, writes the query,
reads a bounded response `ByteBuffer`, and closes the UDP handle. Response
parsing remains separate through `network.dns.response_ipv4` so resolver
policy, retry/backoff, and larger multi-packet DNS behavior can land as later
thin host/runtime layers instead of being duplicated in language frontends.
`network.dns.exchange_udp_retry` is the first retry-policy follow-up for that
same bounded transport path. It adds a total attempt count and millisecond
backoff operand, retries transient UDP exchange failures inside the Rust
runtime, sleeps between failed attempts through the board HAL, and still returns
raw DNS response bytes for the separate parser.
`network.dns.exchange_udp_fallback` adds the resolver-fallback policy layer for
the same bounded DNS-over-UDP exchange. It accepts primary and fallback resolver
IPv4 operands, applies the bounded retry/backoff loop to each resolver, falls
through only after transient primary transport failures, and still returns raw
DNS response bytes for `network.dns.response_ipv4`.

`HandleToken` is private to the board adapter. The portable VM only sees compact
`Handle` ids.

## Capability Dispatch

The interpreter dispatches `CALL_U8` and `CALL_U16` through a capability table.

```
CALL_U8 0x01 -> gpio.open
CALL_U8 0x02 -> gpio.write
CALL_U8 0x10 -> time.sleep_ms
CALL_U8 0x3a -> network.tcp.open
CALL_U8 0x3e -> network.udp.open
CALL_U8 0x42 -> network.wifi.associate
CALL_U8 0x45 -> network.dns.resolve
CALL_U8 0x46 -> network.tcp.connected
CALL_U8 0x47 -> network.udp.available
CALL_U8 0x48 -> network.dns.set_server
CALL_U8 0x49 -> network.tcp.available
CALL_U8 0x4a -> network.dns.query
CALL_U8 0x4b -> network.dns.response_ipv4
CALL_U8 0x4c -> network.udp.write_bytes
CALL_U8 0x4d -> network.udp.read_bytes
CALL_U8 0x4e -> network.dns.exchange_udp
CALL_U8 0x4f -> network.dns.exchange_udp_retry
CALL_U8 0x50 -> network.dns.exchange_udp_fallback
```

Each handler:

1. Checks stack types and count.
2. Pops arguments.
3. Resolves portable handles to board tokens.
4. Calls the HAL trait.
5. Pushes the result, if any.
6. Converts HAL errors into portable VM errors.

Board ports may omit handlers for unsupported capabilities. The descriptor must
match the registered handlers.

## Handle Table

The handle table maps portable handles to board resources:

```rust
pub struct HandleSlot {
    pub generation: u8,
    pub kind: HandleKind,
    pub token: HandleToken,
    pub open: bool,
}

pub struct Handle {
    pub index: u8,
    pub generation: u8,
}
```

Generation counters prevent stale handles from accidentally controlling a newly
opened resource in the same slot.

When a program ends:

- Interactive runs close non-persistent handles unless `keep_handles_after_run`
  was requested.
- Stored programs keep handles until they halt, fault, or are stopped.
- `RESET_VM` closes all handles.

## Main Loop

The firmware main loop is transport-driven:

```
loop:
  poll transport for complete frame
  if frame available:
    decode protocol frame
    dispatch message
    send response

  if background program is running:
    run a small instruction slice
    process yielded sleeps/events
```

Boards without background execution can run only foreground programs and return
`UnsupportedCapability` or `BoardBusy` for background requests.

## Instruction Budget

Every run has an instruction budget. The runtime decrements it after each
decoded instruction. If the budget reaches zero, foreground execution returns
`BudgetExceeded`.

Stored programs that are intended to run forever must either:

- run as background tasks with periodic yields, or
- be compiled into a firmware image where the board's normal watchdog policy
  applies.

## Boot Sequence

```
reset vector
  -> board HAL init
  -> transport init
  -> Board VM runtime init
  -> optional host grace period
  -> if stored boot program exists and no host interrupts:
       validate and run stored program
     else:
       enter command loop
```

The grace period lets a host recover a board with a bad stored program.

## Eject Packaging

There are three implementation paths.

### Stored Bytecode

If `ProgramStore` is available:

1. Host uploads a validated module.
2. Host sends `STORE_PROGRAM(slot, boot_policy)`.
3. Runtime writes bytes and boot metadata to nonvolatile storage.
4. On reset, runtime loads the slot and runs the module.

### Embedded Firmware

If runtime storage is unavailable or unsafe:

1. Host builds a board-specific firmware package.
2. The bytecode module is embedded as a static byte array.
3. Board flashing tool installs the firmware.
4. Runtime starts the embedded module at boot.

The embedded path is also useful when a project is ready to become a normal
firmware artifact.

### AOT Firmware

If a target has a native backend:

1. Host freezes a validated bytecode module as the source of truth.
2. The AOT backend lowers bytecode to target-native code, target assembly, or a
   linkable compiler artifact.
3. A conformance runner compares the lowered program's fake-HAL trace against
   the interpreter's trace for the same descriptor and runtime profile.
4. The firmware package includes the board startup code, HAL bindings, and the
   lowered program entrypoint, but may omit the interpreter loop and bytecode
   decoder.

AOT eject must preserve bytecode semantics. If a target lacks AOT support, or
if validation cannot prove the selected module is supported by that backend,
the host must use stored bytecode or embedded-bytecode firmware instead.

## Public API

```rust
pub struct Runtime<B: BoardHal, T: Transport, const CFG: RuntimeConfig> {
    board: B,
    transport: T,
    vm: VmState<CFG>,
}

pub trait BoardHal {
    type Gpio: Gpio;
    type Clock: Clock;
    type Store: ProgramStore;

    fn descriptor(&self) -> BoardDescriptor;
    fn gpio(&mut self) -> &mut Self::Gpio;
    fn clock(&mut self) -> &mut Self::Clock;
    fn store(&mut self) -> &mut Self::Store;
}

impl<B, T, const CFG: RuntimeConfig> Runtime<B, T, CFG>
where
    B: BoardHal,
    T: Transport,
{
    pub fn poll(&mut self) -> RuntimePollResult;
    pub fn run_slice(&mut self, max_instructions: u16) -> VmResult<RunSliceReport>;
    pub fn reset_vm(&mut self) -> VmResult<()>;
}
```

The exact const-generic syntax may change during implementation if stable Rust
or embedded targets make another shape cleaner.

## Test Strategy

- Unit test VM execution with fake GPIO and fake clock.
- Unit test handle generation and stale handle rejection.
- Unit test capability descriptor matches registered handlers.
- Unit test foreground and background run budget behavior.
- Protocol integration test with in-memory transport.
- Recovery test: bad stored program can be bypassed during host grace period.
- Board smoke test: blink onboard LED.

## Future Extensions

- RTOS integration for true background tasks.
- Interrupt-backed event subscriptions.
- DMA-backed serial transports for high-throughput logging.
- Persistent handle declarations for long-lived peripherals.
- Firmware image generator for popular boards.
- AOT eject pipeline for targets that can remove the VM from final firmware.
- Board self-test command for pins, timers, and storage.
