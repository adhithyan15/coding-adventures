# BVM02 - Board VM Bytecode IR

## Overview

Board VM bytecode is the portable instruction format executed by the board
runtime. It is compact, deterministic, and independent of host language and
board family.

The bytecode is intentionally not a general-purpose language runtime. It is a
small stack machine with capability calls. Host SDKs can expose friendly APIs in
JS, Python, Ruby, Java, Lua, or a CLI, but all of them lower to the same
bytecode bytes.

## Layer Position

```
Host expression or REPL command
        |
        v
BVM02 bytecode module
        |
        v
BVM01 protocol upload
        |
        v
board-vm-runtime interpreter
        |
        v
board HAL capability call
```

## Design Principles

- **Small interpreter**: one-byte opcodes, simple immediates, bounded stack.
- **Portable hardware access**: hardware is reached through capability calls,
  not board registers.
- **Ahead-of-run validation**: reject malformed bytecode before execution.
- **Bounded execution**: instruction and time budgets prevent host lockout.
- **Stable bytes**: the same source operation should encode identically in every
  host SDK.

## Module Format

The protocol can upload raw instruction bytes for tiny interactive commands, but
stored programs use a bytecode module:

```
magic: 4 bytes       # ASCII "BVM1"
version: u8          # 0x01
flags: u8
max_stack: u8
reserved: u8         # must be 0
code_len: uleb128
code: bytes[code_len]
const_len: uleb128
const_pool: bytes[const_len]
```

Flags:

```
bit 0: program_may_run_forever
bit 1: program_uses_events
bit 2: program_requests_persistent_handles
bits 3..7: reserved
```

The first MVP can set `const_len = 0` and use inline immediates only.

## Execution Model

The VM state contains:

```
ip: u32                    # instruction pointer into code
stack: Value[max_stack]    # operand stack
handles: HandleTable       # runtime-owned resources
halted: bool
last_error: Option<VmError>
```

Instructions either:

- mutate the stack,
- change `ip`,
- call a capability,
- halt.

The VM is cooperative. A capability call such as `time.sleep_ms` may yield to
the board runtime. Long-running programs must not block protocol handling
forever on boards that support background execution.

## Value Types

The VM value set is deliberately small:

| Type | Storage | Use |
|---|---|---|
| `unit` | no payload | no result |
| `bool` | 1 byte | digital levels, conditions |
| `u8` | 1 byte | small immediates, modes |
| `u16` | 2 bytes | pins, handles, milliseconds |
| `u32` | 4 bytes | counters, timestamps |
| `i16` | 2 bytes | signed sensor values |
| `handle` | 2 bytes | resources opened by capabilities |

The MVP should implement `unit`, `bool`, `u8`, `u16`, and `handle`.

## Core Opcodes

| Opcode | Mnemonic | Operands | Stack Effect |
|---:|---|---|---|
| `0x00` | `HALT` | none | stop execution |
| `0x01` | `NOP` | none | no change |
| `0x10` | `PUSH_FALSE` | none | `-> bool` |
| `0x11` | `PUSH_TRUE` | none | `-> bool` |
| `0x12` | `PUSH_U8` | `u8` | `-> u8` |
| `0x13` | `PUSH_U16` | `u16_le` | `-> u16` |
| `0x14` | `PUSH_U32` | `u32_le` | `-> u32` |
| `0x15` | `PUSH_I16` | `i16_le` | `-> i16` |
| `0x20` | `DUP` | none | `a -> a a` |
| `0x21` | `DROP` | none | `a ->` |
| `0x22` | `SWAP` | none | `a b -> b a` |
| `0x23` | `OVER` | none | `a b -> a b a` |
| `0x30` | `JUMP_S8` | `i8` | branch relative to next ip |
| `0x31` | `JUMP_IF_FALSE_S8` | `i8` | `bool ->` |
| `0x32` | `JUMP_IF_TRUE_S8` | `i8` | `bool ->` |
| `0x40` | `CALL_U8` | `capability_id: u8` | capability-defined |
| `0x41` | `CALL_U16` | `capability_id: u16_le` | capability-defined |
| `0x50` | `RETURN_TOP` | none | return top stack value to host, then halt |

Opcodes `0x80..0xFF` are reserved for future compact aliases and board-vendor
extensions. Portable bytecode must not require vendor opcodes.

## Portable Capability IDs

Capability calls are typed stack calls. Each call specifies its arguments and
returns in this spec. If the stack contains the wrong types, execution fails
with `TypeMismatch`.

### GPIO

| ID | Name | Args | Returns |
|---:|---|---|---|
| `0x01` | `gpio.open` | `pin: u16/u8`, `mode: u8` | `handle` |
| `0x02` | `gpio.write` | `handle`, `level: bool` | `unit` |
| `0x03` | `gpio.read` | `handle` | `bool` |
| `0x04` | `gpio.close` | `handle` | `unit` |

GPIO modes:

| Value | Mode |
|---:|---|
| `0x00` | input |
| `0x01` | output |
| `0x02` | input_pullup |
| `0x03` | input_pulldown |

### Time

| ID | Name | Args | Returns |
|---:|---|---|---|
| `0x10` | `time.sleep_ms` | `duration_ms: u16` | `unit` |
| `0x11` | `time.now_ms` | none | `u32` |

### PWM

PWM is not required for the blink MVP, but its ids are reserved early so host
SDKs can grow without renumbering.

| ID | Name | Args | Returns |
|---:|---|---|---|
| `0x20` | `pwm.open` | `pin: u16/u8`, `frequency_hz: u16` | `handle` |
| `0x21` | `pwm.set_duty_u16` | `handle`, `duty: u16` | `unit` |
| `0x22` | `pwm.close` | `handle` | `unit` |

### Program

| ID | Name | Args | Returns |
|---:|---|---|---|
| `0x70` | `program.emit_event` | `event_id: u16`, `value` | `unit` |
| `0x71` | `program.log_u16` | `value: u16` | `unit` |

## Validation

Before execution, the runtime validates:

- Every opcode is known.
- Every immediate operand is fully present.
- Every jump target lands on an instruction boundary.
- Reserved flag bits are zero.
- Declared `max_stack` is within the board descriptor.
- Static stack analysis proves no unconditional underflow.
- Capability ids are either advertised by the board or rejected.

Static stack analysis may be conservative. If the validator cannot prove a
program is safe, it may reject the program even if it would happen to run.

## Blink Bytecode Example

This bytecode opens pin 13 as output, then blinks forever with 250 ms high and
250 ms low.

```
12 0D       PUSH_U8 13
12 01       PUSH_U8 1              # GPIO output
40 01       CALL_U8 gpio.open      # -> handle
20          DUP                    # loop_start
11          PUSH_TRUE
40 02       CALL_U8 gpio.write
13 FA 00    PUSH_U16 250
40 10       CALL_U8 time.sleep_ms
20          DUP
10          PUSH_FALSE
40 02       CALL_U8 gpio.write
13 FA 00    PUSH_U16 250
40 10       CALL_U8 time.sleep_ms
30 EC       JUMP_S8 -20            # back to loop_start
```

Raw bytes:

```
12 0D 12 01 40 01 20 11 40 02 13 FA 00 40 10 20
10 40 02 13 FA 00 40 10 30 EC
```

The only board-specific detail is whether logical pin 13 maps to an actual LED.
If the board reports a different onboard LED pin, the host should substitute
that pin before upload.

## Public API

```rust
pub enum Op {
    Halt,
    Nop,
    PushFalse,
    PushTrue,
    PushU8(u8),
    PushU16(u16),
    PushU32(u32),
    PushI16(i16),
    Dup,
    Drop,
    Swap,
    Over,
    JumpS8(i8),
    JumpIfFalseS8(i8),
    JumpIfTrueS8(i8),
    CallU8(u8),
    CallU16(u16),
    ReturnTop,
}

pub struct Module<'a> {
    pub flags: u8,
    pub max_stack: u8,
    pub code: &'a [u8],
    pub const_pool: &'a [u8],
}

pub fn decode_next(code: &[u8], ip: usize) -> Result<(Op, usize), DecodeError>;
pub fn validate(module: &Module, caps: &CapabilitySet) -> Result<(), ValidateError>;
```

## Test Strategy

- Decode every opcode and operand width.
- Reject truncated immediates.
- Reject jumps into the middle of an instruction.
- Reject unknown opcodes.
- Validate stack effects for straight-line programs.
- Validate stack effects across conditional branches.
- Execute blink bytecode on a fake board and assert ordered GPIO operations.
- Golden byte tests for host SDKs.

## Future Extensions

- `CALL_COMPACT_0..63` opcodes that encode common capability ids in one byte.
- Event wait instructions for pin interrupts and sensor thresholds.
- Small local variables for loops without stack juggling.
- Constant pool entries for strings and byte arrays.
- Signed arithmetic and fixed-point helpers.
- Program sections for debug symbols and source maps.
