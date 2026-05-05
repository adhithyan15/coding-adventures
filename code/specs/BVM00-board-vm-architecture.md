# BVM00 - Board VM Architecture

## Overview

Board VM is an interactive runtime for physical microcontroller boards. It lets
a host program connect to a board over UART, USB CDC, BLE, TCP, or another byte
transport, upload compact bytecode, inspect board capabilities, run commands,
and later eject the explored behavior into a standalone firmware image.

The core idea is deliberately split in two:

1. The **IR and protocol are portable**. A Python client, Ruby client, Java
   client, Lua client, JS client, or CLI all produce the same binary messages
   and the same bytecode.
2. The **runtime adapter is board-specific**. Arduino Uno, Arduino Nano 33,
   RP2040, ESP32, MBed, and other boards expose different pins, timers, PWM
   channels, storage limits, and flashing tools. Each board port maps the
   portable VM calls onto its real hardware.

This spec uses the name **Board VM** to avoid confusion with
`hardware-vm.md`, which is the HDL event-driven simulator for Verilog/VHDL.
Board VM talks to real physical boards.

## Goals

- Make hardware development feel interactive: connect, type, run, observe,
  adjust.
- Keep the wire protocol language-agnostic and transport-agnostic.
- Keep the board runtime small enough for constrained microcontrollers.
- Support a useful first end-to-end flow: blink a GPIO pin interactively, then
  eject it as a stored program.
- Make later features obvious: PWM motors, ADC sensors, I2C/SPI devices,
  sound, events, streaming logs, and board-specific drivers.

## Non-Goals

- It is not a full JavaScript, Python, Ruby, or Lua interpreter on the board.
  Host languages are clients that compile or encode portable operations.
- It is not a replacement for Rust/C/C++ firmware for performance-critical
  loops.
- It does not hide board limits. The board reports what it can do, and host
  SDKs must adapt to that report.
- It is not tied to Arduino. Arduino is the first likely target because it is
  accessible, not because the protocol depends on it.

## Layer Position

```
Host language SDKs
  JS / Python / Ruby / Java / Lua / CLI
        |
        v
Board VM protocol and bytecode IR
        |
        v
Transport: UART / USB CDC / BLE / TCP / WebSerial
        |
        v
Board VM runtime in Rust
        |
        v
Board adapter: Arduino / MBed / RP2040 / ESP32 / ...
        |
        v
Physical pins, timers, buses, sensors, motors, speakers
```

Board VM sits beside the existing VM and simulator work in the repository, but
its output target is a live microcontroller rather than a local software
simulator.

## Package Map

The initial implementation should be split into small packages so the portable
pieces can be reused by every board and every host language.

| Package | Role | Target |
|---|---|---|
| `board-vm-protocol` | Frame codec, message types, errors, golden vectors | Rust `no_std` + host use |
| `board-vm-ir` | Bytecode instruction format, decoder, validator | Rust `no_std` |
| `board-vm-runtime` | Interpreter, handle table, command loop | Rust `no_std` |
| `board-vm-host` | Reference CLI/REPL and uploader | Desktop Rust |
| `board-vm-arduino-*` | Board-specific HAL adapter and firmware package | Embedded Rust |
| `board-vm-conformance` | Shared protocol and bytecode test vectors | Host tests |

Later language SDK packages should depend on the specification and test vectors,
not on one another:

```
board-vm-js
board-vm-python
board-vm-ruby
board-vm-java
board-vm-lua
```

## Concepts

### Board Descriptor

Every connected board reports a descriptor before accepting programs:

```
BoardDescriptor:
  protocol_version: u16
  board_id: string
  runtime_id: string
  max_frame_payload: u16
  max_program_bytes: u32
  max_stack_values: u8
  max_handles: u8
  transports: list[TransportKind]
  capabilities: list[CapabilityDescriptor]
```

The descriptor is the host's source of truth. If a board does not report PWM,
the host SDK must not offer PWM on that connection except as an unsupported
operation that fails before upload.

### Capabilities

A capability is a stable, portable operation family exposed by the board:

| Capability | Example operation | Board mapping |
|---|---|---|
| `gpio.digital` | configure pin, write pin, read pin | pin registers / HAL GPIO |
| `time.sleep` | sleep or yield for milliseconds | timer peripheral / RTOS delay |
| `pwm.output` | set duty cycle and frequency | timer channel |
| `adc.input` | sample analog input | ADC peripheral |
| `i2c.master` | write/read I2C transactions | I2C peripheral |
| `serial.log` | stream text or binary logs | current transport or extra UART |
| `program.store` | save bytecode for boot | flash / EEPROM / filesystem |

Capabilities are not permissions in the security sense for the first version.
They are a compact feature contract between host and board.

### Resource Handles

Host and bytecode programs do not directly own hardware. They own handles:

```
pin 13 as GPIO output -> handle 1
PWM on pin 5          -> handle 2
I2C bus 0             -> handle 3
```

Handles keep the bytecode compact and allow the runtime to validate lifetime,
pin conflicts, and board-specific resource allocation.

### Interactive Mode

Interactive mode is optimized for quick iteration:

1. Host connects and performs `HELLO`.
2. Host asks for board capabilities.
3. User enters a command in a REPL or SDK call.
4. Host compiles that command to one small bytecode program or direct control
   message.
5. Board executes it and returns a result, event, log, or error.

Interactive mode is allowed to be slower because UART round trips are in the
loop. It is for exploration and inspection.

### Stored Program Mode

Stored program mode is optimized for standalone behavior:

1. Host uploads a bytecode module.
2. Runtime validates it against the board descriptor and resource limits.
3. Host asks the board to store the module if supported.
4. Board boots into the VM and runs the stored module without the host.

If a board cannot write flash or EEPROM safely from the runtime, the host can
instead generate a firmware image with the VM and bytecode embedded.

### Eject

Eject converts an interactive session into standalone firmware. There are two
eject forms:

| Form | Use when | Output |
|---|---|---|
| Store on device | Board supports persistent program storage | Bytecode written to flash/EEPROM |
| Build firmware | Board requires host-side flashing | Runtime firmware with embedded bytecode |

The same bytecode module should work in both forms. The difference is where the
module is stored and how it is launched.

## Public API

The architectural API is a package-level contract rather than a single class.
The first Rust crates should expose these shapes.

```rust
pub struct BoardDescriptor {
    pub protocol_version: u16,
    pub board_id: &'static str,
    pub runtime_id: &'static str,
    pub max_frame_payload: u16,
    pub max_program_bytes: u32,
    pub max_stack_values: u8,
    pub max_handles: u8,
    pub capabilities: &'static [CapabilityDescriptor],
}

pub struct CapabilityDescriptor {
    pub id: u16,
    pub name: &'static str,
    pub version: u8,
    pub flags: u16,
}

pub trait BoardRuntime {
    fn descriptor(&self) -> BoardDescriptor;
    fn reset_vm(&mut self) -> VmResult<()>;
    fn upload_chunk(&mut self, program_id: u16, offset: u32, bytes: &[u8]) -> VmResult<()>;
    fn run_program(&mut self, program_id: u16, budget: RunBudget) -> VmResult<RunReport>;
    fn store_program(&mut self, program_id: u16, slot: u8) -> VmResult<()>;
}
```

Board-specific crates implement hardware traits consumed by `board-vm-runtime`:

```rust
pub trait GpioHal {
    fn open_output(&mut self, pin: PinId, initial: Level) -> VmResult<Handle>;
    fn write(&mut self, handle: Handle, level: Level) -> VmResult<()>;
    fn read(&mut self, handle: Handle) -> VmResult<Level>;
}

pub trait ClockHal {
    fn now_ms(&self) -> u32;
    fn sleep_ms(&mut self, ms: u16) -> VmResult<()>;
}
```

## Data Flow

Interactive command:

```
Host expression
  -> host SDK operation
  -> Board VM bytecode or control message
  -> protocol frame
  -> transport bytes
  -> board runtime
  -> HAL capability call
  -> physical hardware state
  -> response frame
```

Ejected program:

```
Host session history or source file
  -> bytecode module
  -> validate against BoardDescriptor
  -> store on board OR embed in firmware
  -> board reset
  -> VM boot
  -> physical hardware behavior
```

## Error Model

Errors must be structured and portable:

| Error | Meaning |
|---|---|
| `UnsupportedCapability` | Host requested a capability the board did not advertise |
| `InvalidFrame` | Frame could not be decoded or failed CRC |
| `InvalidBytecode` | Program failed validation |
| `StackOverflow` | Program exceeded configured stack depth |
| `StackUnderflow` | Instruction needed values that were not present |
| `HandleNotFound` | Program referenced a missing or closed resource |
| `ResourceBusy` | Pin, timer, bus, or storage slot is already owned |
| `BudgetExceeded` | Program exceeded instruction or time budget |
| `BoardFault` | HAL reported a board-specific hardware failure |

Error responses should include the request id, error code, bytecode offset when
available, and a compact message string when space permits.

## Test Strategy

- Protocol golden vectors: exact bytes for every control message.
- Bytecode golden vectors: exact decode and validation for every instruction.
- Fake board runtime: run bytecode against an in-memory GPIO/timer HAL.
- Loopback transport: host sends frames through an in-memory transport and
  receives board responses.
- Blink MVP: fake board observes pin 13 high, sleep, low, sleep in order.
- Hardware smoke test: supported Arduino board blinks onboard LED after upload.
- Eject smoke test: reboot board and verify stored blink program starts.

## Future Extensions

- PWM motors with frequency, duty cycle, direction pins, and ramping.
- ADC streaming and threshold events.
- I2C and SPI device descriptors for sensors.
- Sound/tone generation over PWM or DAC.
- Event subscriptions for pin changes and sensor updates.
- Debugging support: step, break, inspect stack, inspect handles.
- Capability schemas generated into host SDKs.
- Static verifier for programs that should be safe to run forever.
