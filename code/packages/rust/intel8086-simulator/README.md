# Intel 8086 Simulator (Rust)

Complete behavioral simulator for the Intel 8086 (1978), ported from the
repository's Python oracle. The 8086 introduced the segmented memory and ModRM
addressing model inherited by every later x86 generation.

## Architectural surface

The simulator implements the oracle's complete specified instruction surface:

- all 24 ModRM effective-address forms, byte/word register and memory operands,
  direct addresses, and ES/CS/SS/DS overrides;
- MOV, XCHG, segment-register moves, LEA, LDS, LES, XLAT, CBW/CWD, LAHF/SAHF;
- ADD/ADC/SUB/SBB/CMP, AND/OR/XOR/TEST, INC/DEC, NEG/NOT, shifts and rotates,
  MUL/IMUL/DIV/IDIV, and all six BCD/ASCII adjust instructions;
- PUSH/POP/PUSHF/POPF, near/far CALL/JMP/RET, all sixteen conditional jumps,
  LOOP/LOOPE/LOOPNE/JCXZ, IRET, and the oracle's halt-on-INT convention;
- MOVS/CMPS/STOS/LODS/SCAS with REP/REPE/REPNE and direction control;
- HLT, WAIT, LOCK, carry/direction/interrupt flag controls, and byte/word I/O.

All instruction fetch and data access uses the real 20-bit segmented formula:

```text
physical = ((segment << 4) + offset) & 0xFFFFF
```

The machine always owns the architectural 1 MiB memory and two 256-byte port
banks. Checked APIs provide atomic loads, complete snapshot/restore, typed
invalid-opcode/range/port failures, transactional bounded runs, and complete
before/after traces including prefixes and operand bytes. The legacy
`run(&[u8])` and `step() -> String` entry points remain source-compatible with
existing encoder/backend consumers.

## Usage

```rust
use intel8086_simulator::Intel8086Simulator;

let mut sim = Intel8086Simulator::new(1 << 20);
let result = sim.run_checked(&[
    0xB8, 42, 0x00, // MOV AX,42
    0xF4,           // HLT
], 10)?;
assert!(result.halted);
assert_eq!(result.final_state.ax, 42);
# Ok::<(), intel8086_simulator::Intel8086Error>(())
```

## Verification

The ordinary unit and lifecycle suites are joined by 461 deterministic
full-state differential vectors generated from
`code/packages/python/intel-8086-simulator`. They classify all 256 first bytes,
exercise every extension of dense opcode groups, cover all effective-address
forms and widths/directions, and cover prefixes, strings, control flow, stack,
and I/O. Each vector compares every register and flag plus hashes of all 1 MiB
of memory and both port banks. The generator is checked in beside the fixture.

See [`code/specs/07m-intel-8086-simulator.md`](../../../specs/07m-intel-8086-simulator.md)
for the normative ISA contract.
