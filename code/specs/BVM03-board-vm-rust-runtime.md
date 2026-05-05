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
| `board-vm-arduino-uno` | AVR/Arduino Uno adapter, if Rust target support is viable |
| `board-vm-rp2040` | RP2040 adapter |
| `board-vm-esp32` | ESP32 adapter |
| `board-vm-mbed-*` | MBed-style board adapters |

If a classic AVR Arduino is awkward for Rust or too constrained for the first
vertical slice, the architecture should still support it later. The first
implemented board may be whichever gift board and Rust target are practical.

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

`HandleToken` is private to the board adapter. The portable VM only sees compact
`Handle` ids.

## Capability Dispatch

The interpreter dispatches `CALL_U8` and `CALL_U16` through a capability table.

```
CALL_U8 0x01 -> gpio.open
CALL_U8 0x02 -> gpio.write
CALL_U8 0x10 -> time.sleep_ms
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

There are two implementation paths.

### Stored Bytecode

If `ProgramStore` is available:

1. Host uploads a validated module.
2. Host sends `STORE_PROGRAM(slot, boot_policy)`.
3. Runtime writes bytes to nonvolatile storage.
4. On reset, runtime loads the slot and runs the module.

### Embedded Firmware

If runtime storage is unavailable or unsafe:

1. Host builds a board-specific firmware package.
2. The bytecode module is embedded as a static byte array.
3. Board flashing tool installs the firmware.
4. Runtime starts the embedded module at boot.

The embedded path is also useful when a project is ready to become a normal
firmware artifact.

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
- Board self-test command for pins, timers, and storage.
