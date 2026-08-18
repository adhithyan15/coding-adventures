# Intel 8080 Simulator (Rust)

Behavioral simulator for the Intel 8080 (1974) — Intel's first widely
successful 8-bit microprocessor, direct successor to the 8008, and the CPU
inside the Altair 8800 that launched the personal-computer revolution.
CP/M targeted the 8080; Microsoft was founded to write BASIC for it; its
ISA is the direct ancestor of the Z80 and the Intel 8086 (and therefore
all of x86). Rust port of `code/packages/python/intel8080-simulator`
(Layer 07i); see
[`code/specs/07i-intel8080-simulator.md`](../../../specs/07i-intel8080-simulator.md)
for the full ISA writeup.

## Supported Instructions

- **Data transfer**: mov, mvi, lxi, sta, lda, shld, lhld, stax, ldax, xchg
- **Arithmetic**: add, adc, sub, sbb, adi, aci, sui, sbi, inr, dcr, inx, dcx, dad, daa
- **Logical**: ana, xra, ora, cmp, ani, xri, ori, cpi, rlc, rrc, ral, rar, cma, cmc, stc
- **Branch**: jmp + 8 conditional jumps, call + 8 conditional calls, ret + 8
  conditional returns, rst 0-7, pchl
- **Stack**: push/pop (incl. `PUSH PSW`/`POP PSW`), xthl, sphl
- **I/O**: in, out (256 ports each direction)
- **Control**: nop, hlt, ei, di

## Architecture

```
opcodes.rs   -- opcode / register / condition-code constant tables
encoding.rs  -- encode_* helpers to construct machine code byte sequences
decode.rs    -- variable-length instruction decoder (1, 2, or 3 bytes)
execute.rs   -- instruction executor + named-register state
simulator.rs -- top-level Intel8080Simulator with fetch-decode-execute
```

## What differs from the Python original (and from `mips-r2000-simulator`)

- **Named registers, not an indexed register file.** The 8080 has seven
  individually named 8-bit registers (A, B, C, D, E, H, L) plus a 16-bit
  SP, not MIPS's 32 numbered GPRs — `execute::Registers` is a plain
  named-field struct rather than `cpu_simulator::RegisterFile`.
- **Variable-length instructions.** 1, 2, or 3 bytes depending on opcode,
  unlike MIPS R2000's fixed 32-bit words — `decode::decode` takes the
  already-fetched opcode byte plus a `fetch` closure for any remaining
  operand bytes, rather than decoding a single fixed-width value.
- **Masked-first flag arithmetic.** The Python original computes S/Z/P
  from the unmasked (possibly >255) sum/difference and masks afterward;
  this port computes the masked `u8` result first via `u16`-widened
  arithmetic, which is equivalent (masking to 8 bits never changes bits
  0-7) and more idiomatic Rust.
- **Fail-closed halt on undefined opcodes** instead of the Python
  original's `ValueError` — no exception channel through `step() ->
  String`, so an undefined opcode halts the simulator, matching the
  fail-closed convention `mips-r2000-simulator` uses for signed-overflow /
  divide-by-zero.

## Usage

```rust
use intel8080_simulator::Intel8080Simulator;
use intel8080_simulator::encoding::*;
use intel8080_simulator::opcodes::*;

let mut sim = Intel8080Simulator::new(65536);
sim.run_instructions(&[
    encode_mvi(REG_B, 1),
    encode_mvi_a(2),
    vec![encode_alu_reg(ALU_ADD, REG_B)],
    vec![HLT],
]);
assert_eq!(sim.regs.a, 3);
```
