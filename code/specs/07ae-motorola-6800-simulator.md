# Layer 07ae — Motorola MC6800 Functional Simulator

## Status and sources

This specification defines the repository's checked, instruction-level model
of the original Motorola MC6800 introduced in 1974. It is the behavioral
reference for the later gate-level MC6800 cell. It is not the already-complete
Motorola 68000/68k simulator: the MC6800 is an earlier 8-bit architecture with
a 16-bit address bus.

The primary sources are Motorola's *M6800 Programming Reference Manual*, form
M68PRM(D) (November 1976), and *M6800 Systems Reference and Data Sheets* (May
1975). The implementation follows their programming model, seven addressing
modes, big-endian 16-bit operands, condition-code rules, stack order, interrupt
vectors, and complete base-MC6800 instruction table. MC6801, 6802, 6803, 6808,
6809, and later-family extensions are outside this layer.

## Architectural state

`Motorola6800State` is a complete owned snapshot containing:

- 8-bit accumulators A and B;
- 16-bit index register X, stack pointer, and program counter;
- the condition-code register bits H, I, N, Z, V, and C, with its two reserved
  high bits reading as one;
- the complete 65,536-byte address space;
- wait/halt state; and
- the installed program origin and byte length.

Every data and stack address is a native 16-bit address and therefore wraps at
the address-bus boundary. Multi-byte values are stored most-significant byte
first. Public register, memory, state-restoration, and lifecycle operations are
checked and never expose a partially changed state after an error.

## Program transport and execution lifecycle

Programs are raw MC6800 bytes. `load(program, origin)` rejects an empty image
or an image that does not fit contiguously in the 64 KiB address space before
changing state. A successful load resets the CPU and memory, copies the entire
image, records its exact range, and starts the program counter at `origin`.

Instruction fetch must begin inside the installed range, and every operand byte
must also be installed. Data and stack accesses may use the entire address
space. Fetch outside the image, truncated instructions, unknown opcodes,
invalid restored state, post-wait execution, and exhausted step limits are
distinct typed faults.

`step` is atomic. `run(max_steps)` and `execute(program, origin, max_steps)` are
transactional as a whole. Every successful step returns the before/after PC,
exact raw instruction bytes, width, conventional uppercase mnemonic, and the
complete state after the instruction.

The MC6800 `WAI` instruction is the repository's deterministic halt sentinel.
It performs the manual's status-stack operation, sets the waiting state, and
returns a successful `WAI` trace. With no asynchronous device model in this
CPU-only cell, a later step returns `Halted` without mutation. `SWI` remains a
real software interrupt: it stacks the documented frame, sets I, and loads the
big-endian vector at `$FFFA-$FFFB`; `RTI` restores that frame. External IRQ and
NMI pins, cycle timing, and peripherals belong to the later system/SoC layer.

## Addressing modes

The base processor's seven modes are implemented exactly:

- **accumulator**: A or B is the operand;
- **immediate**: the following one or two bytes are the operand;
- **direct**: the following byte addresses zero page `$0000-$00FF`;
- **indexed**: the following unsigned byte is added to X modulo 16 bits;
- **extended**: the following two big-endian bytes are the absolute address;
- **implied**: the opcode identifies every operand; and
- **relative**: the following signed byte is added to the post-instruction PC.

## Required instruction surface

All 72 documented base-MC6800 operations and every documented opcode/addressing
combination are required. The remaining first-byte values are undefined and
return `UnknownOpcode` atomically.

### Data, arithmetic, and logic

- loads/stores: `LDAA`, `LDAB`, `LDS`, `LDX`, `STAA`, `STAB`, `STS`, `STX`;
- transfers/stack: `TAB`, `TBA`, `TAP`, `TPA`, `TSX`, `TXS`, `PSHA`, `PSHB`,
  `PULA`, `PULB`, `INS`, `DES`;
- arithmetic: `ABA`, `SBA`, `ADCA/B`, `ADDA/B`, `SBCA/B`, `SUBA/B`, `CMPA/B`,
  `CBA`, `CPX`, `INCA/B`, `DECA/B`, `INX`, `DEX`, `NEGA/B`, and `DAA`;
- logic/test: `ANDA/B`, `ORAA/B`, `EORA/B`, `BITA/B`, `COMA/B`, `CLRA/B`;
- shifts/rotates: `ASLA/B`, `ASRA/B`, `LSRA/B`, `ROLA/B`, and `RORA/B`;
- memory read-modify-write forms: `NEG`, `COM`, `LSR`, `ROR`, `ASR`, `ASL`,
  `ROL`, `DEC`, `INC`, `TST`, `JMP`, and `CLR` in indexed and extended modes.

### Control and flags

- branches: `BRA`, `BCC/BHS`, `BCS/BLO`, `BEQ`, `BNE`, `BPL`, `BMI`, `BVC`,
  `BVS`, `BGE`, `BLT`, `BGT`, `BLE`, `BHI`, `BLS`, and `BSR`;
- subroutines/interrupts: `JSR`, `RTS`, `RTI`, `SWI`, and `WAI`;
- flags: `CLC`, `SEC`, `CLI`, `SEI`, `CLV`, and `SEV`; and
- `NOP`.

H is carry from bit 3 for addition and decimal adjustment. N is the result's
high bit, Z marks zero, V follows the manual's signed-overflow equations, and C
is carry or not-borrow as specified per operation. Loads and logical operations
update N/Z/V but preserve unrelated flags. Stores update N/Z/V from the stored
value. Sixteen-bit load/store/compare operations use X or SP and big-endian
memory. `DAA` applies the MC6800's documented BCD correction to accumulator A.

## Stack, calls, and interrupts

The stack grows downward. A push stores at SP and then decrements SP; a pull
increments SP before reading. Sixteen-bit return addresses and registers use
the byte order and push/pull sequence from the Motorola status-stack diagram.
`BSR` and `JSR` stack the post-instruction return address. `RTS` restores it.

`SWI` and `WAI` stack PC, X, A, B, and the condition-code register in the
documented order. `SWI` then sets I and loads its vector. `RTI` restores CC, B,
A, X, and PC in reverse stack order. An interrupt vector target outside the
installed program is permitted to be loaded into PC, but the next instruction
fetch still enforces the installed-image boundary.

## Errors and acceptance criteria

Errors have stable, nonempty display text and distinguish empty/oversized
programs, program range overflow, fetch outside the installed image, truncated
instructions, unknown opcodes, invalid state, halted execution, and exhausted
step limits. No program byte may reach a panic-only path.

Completion requires:

1. a table-driven classification test over all 256 first-byte values;
2. positive witnesses for every documented opcode/addressing combination and
   condition-code family, plus both outcomes of every conditional branch;
3. stack, subroutine, SWI/RTI, WAI, direct/indexed/extended wrap, BCD, and
   big-endian 16-bit boundary tests;
4. exact complete-state, raw-byte, mnemonic, and transactional-fault tests;
5. line coverage above 95% where practical and never below 80%; and
6. formatting, strict Clippy, rustdoc, release tests, package BUILD, lessons
   validation, repository dry-run, workspace build, and security review before
   publication.
