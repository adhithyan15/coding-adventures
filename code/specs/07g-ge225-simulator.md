# 07g — GE-225 Simulator (IR Backend Target)

## Overview

The GE-225 simulator is a **behavioral simulator** for the General Electric
GE-225 mainframe instruction set, scoped specifically as a **native backend
target** for the repository's compiler pipeline.

This spec does **not** attempt to recreate the full Dartmouth Time-Sharing
System (DTSS), the DATANET-30 communications processor, or every historical
peripheral attached to a GE-225 installation. The goal is narrower and more
useful for the compiler stack:

- provide a real historical ISA target with a documented 20-bit instruction word
- expose enough machine state to run compiled programs deterministically
- define a stable backend contract for lowering Semantic IR to GE-225 code
- create a path from `Dartmouth BASIC frontend -> Semantic IR -> GE-225 machine code`

In other words, this package is the "native machine" half of the Dartmouth BASIC
story, not the entire 1964 campus time-sharing environment.

The simulator begins as a **minimal executable GE-225 core**. It should be able
to load words into memory, execute a practical subset of the instruction set,
branch, read/write memory, and halt. Full historical fidelity is a future
extension.

## Layer Position

```
Logic Gates → Arithmetic → CPU → [YOU ARE HERE] → Assembler → IR Backend → Frontend
```

Within the Dartmouth BASIC path, the intended compiled flow is:

```
Dartmouth BASIC source
  → lexer
  → parser
  → Semantic IR
  → GE-225 backend
  → GE-225 machine code
  → GE-225 simulator
```

This is an alternative Layer 4 alongside RISC-V, ARM, WASM, Intel 4004, ARM1,
and Intel 8008.

## Why the GE-225?

- **Historically appropriate** — Dartmouth BASIC was first deployed on the GE-225
- **Architecturally interesting** — 20-bit, single-address, pre-microprocessor ISA
- **Good compiler target** — small enough to understand, real enough to matter
- **Bridges the stack** — gives the Dartmouth BASIC frontend a concrete machine target
- **Different from later hobbyist CPUs** — this is not another 8-bit microprocessor;
  it is an early mainframe architecture with a distinct execution model

## Scope Boundary

### Included in this package

- GE-225 word-addressed memory model
- GE-225 instruction-word decoding
- core arithmetic/data movement/control-flow execution
- minimal machine state (`A`, `Q`, `M`, `PC`, indicators, memory)
- loading machine words into memory and stepping/running execution
- enough instruction support to serve as a backend target for Semantic IR
- a host-side trap model for development-time I/O (`print`, `input`, `halt`)

### Explicitly excluded from MVP

- full DTSS emulation
- DATANET-30 emulation
- time-sharing scheduler behavior
- teleprinter timing and device protocols
- cycle-accurate core-memory timing
- exact historical operator console behavior
- full peripheral set (card readers, tape, disk, printers)
- full floating-point fidelity on day one

The MVP is "compiled code can run on a GE-225-like CPU core", not "a full 1964
computer center in software".

## Architecture

The GE-225 is modeled as a **20-bit, word-addressed, single-address machine**.
The historical manuals define a one-word instruction format with a base opcode
field, a 2-bit modification field, and a 13-bit address field.

### Core machine properties

| Feature | Value |
|---------|-------|
| Word width | 20 bits |
| Addressing unit | 1 word |
| Instruction width | 1 word (20 bits) |
| Base memory size | 4,096 words |
| Extended memory size | up to 16,384 words |
| Address field | 13 bits |
| Execution style | fetch-decode-execute |
| Instruction family | single-address, memory-reference oriented |

### Minimum exposed machine state

The simulator exposes the smallest useful architectural state needed for execution
and debugging:

```text
A   = primary arithmetic accumulator
Q   = extension / quotient register used by multi-word arithmetic families
M   = memory buffer / memory operand register
PC  = program counter (word address)
IR  = current 20-bit instruction word
IND = condition / indicator bits used by branches and tests
MEM = word-addressed core memory
```

This spec intentionally avoids overcommitting to every console-visible latch and
maintenance register in the historical hardware. The exposed state is the contract
required by the compiler, simulator, tracer, and tests.

### Word format

Every memory word stores an unsigned 20-bit pattern:

```text
bit 19 .............................................. bit 0
```

The simulator preserves raw 20-bit words exactly. Higher-level signed or numeric
meaning comes from the executed instruction, not from the storage container itself.

### Instruction format

The historical GE-225 instruction word is modeled as:

```text
┌────────────┬──────────────┬──────────────────────────┐
│ opcode     │ modifier     │ address                  │
│ 5 bits     │ 2 bits       │ 13 bits                  │
└────────────┴──────────────┴──────────────────────────┘
```

- **opcode** selects the instruction family
- **modifier** selects indexing / automatic modification behavior
- **address** identifies the operand word or branch target

The MVP simulator must decode all three fields even if the initial compiler target
only uses a subset of modifier modes.

## Execution Model

### Fetch-decode-execute

Each `step()` performs:

```text
1. Fetch instruction word from memory[PC]
2. Decode opcode, modifier, address
3. Resolve effective address (including modifier/index rules when implemented)
4. Execute instruction semantics
5. Update registers / indicators / memory
6. Advance PC unless control flow overrides it
```

### Addressing model

The simulator is **word-addressed**, not byte-addressed. Address `100` refers to
word `100`, not byte `100`.

For MVP:

- all loader inputs are expressed as arrays of 20-bit words
- all disassembly is reported in word addresses
- all branch targets are word addresses

### Halt behavior

Execution stops when any of the following occur:

- a historical stop/halt instruction executes
- a simulator-defined development trap requests halt
- `max_steps` is exceeded
- an illegal/unimplemented opcode is encountered

## MVP Instruction Subset

The full GE-225 ISA is larger than the minimum needed for a first backend. The
compiler-facing MVP is therefore a **canonical executable subset** called
**GE225/Core**.

### GE225/Core must include

- load from memory into `A`
- store `A` to memory
- transfer between `A` and `Q` where needed by code generation
- add/subtract using memory operands
- compare/test instructions sufficient for conditional branching
- unconditional branch/jump
- conditional branch
- stop/halt

### GE225/Core may initially defer

- full multiply/divide families
- exact floating-point operations
- full automatic modification/index repertoire
- device-specific I/O instructions
- decimal and scientific library behavior

The key requirement is that Semantic IR backends can target GE225/Core without
needing to emulate DTSS or every optional instruction family.

## Backend Contract

This package exists primarily to support a GE-225 backend for `IR00-semantic-ir.md`.

### Backend lowering strategy

The GE-225 backend should lower IR in two stages:

```text
Semantic IR
  → symbolic GE-225 assembly / machine-level blocks
  → encoded 20-bit machine words
```

This separation matters because:

- labels and forward branches are easier to patch symbolically
- the backend can stay readable
- the assembler becomes reusable outside the BASIC pipeline

### Storage model for MVP

The initial backend uses a **static frame / spill-slot model**:

- globals live at fixed word addresses
- temporaries spill to fixed words allocated by the backend
- there is no requirement for a general-purpose stack in phase 1
- function calls may be deferred or lowered via runtime conventions later

This is a good fit for early Dartmouth BASIC, which is line-oriented, global-state
heavy, and does not require a modern re-entrant call stack for the first compiled path.

### Runtime boundary

Some features are better expressed as runtime helpers than as raw inline machine code
in the first implementation.

The backend may therefore target well-known runtime entry points for:

- numeric printing
- numeric input
- string literal output
- bounds errors / runtime traps
- future floating-point helpers

The simulator must support calling such helpers either as loaded machine code or as
host-provided development traps.

## Public API

```python
@dataclass
class GE225State:
    a: int                      # 20-bit
    q: int                      # 20-bit
    m: int                      # 20-bit
    pc: int                     # word address
    ir: int                     # current 20-bit instruction
    indicators: dict[str, bool]
    halted: bool
    memory: list[int]           # each entry masked to 20 bits


@dataclass
class GE225TraceEntry:
    address: int
    instruction_word: int
    opcode: int
    modifier: int
    address_field: int
    mnemonic: str


class GE225Simulator:
    def __init__(self, memory_words: int = 4096) -> None: ...

    @property
    def state(self) -> GE225State: ...

    def reset(self) -> None: ...

    def load_words(self, words: list[int], start_address: int = 0) -> None: ...
        # Load raw 20-bit words into memory.

    def read_word(self, address: int) -> int: ...
    def write_word(self, address: int, value: int) -> None: ...

    def step(self) -> GE225TraceEntry: ...
        # Execute one instruction.

    def run(self, max_steps: int = 100000) -> list[GE225TraceEntry]: ...
        # Run until halt or max_steps.

    def disassemble_word(self, word: int) -> str: ...
        # Human-readable instruction rendering.
```

### Assembler-facing API

The eventual assembler package targeting this simulator should expose:

```python
def encode_instruction(opcode: int, modifier: int, address: int) -> int: ...
def decode_instruction(word: int) -> tuple[int, int, int]: ...
```

The simulator and assembler must agree on a single canonical instruction encoding.

## Data Flow

```text
Input:
  - raw 20-bit words
  - or assembly lowered and encoded by a GE-225 backend

Execution:
  - load words into core memory
  - run fetch/decode/execute loop

Output:
  - final machine state
  - optional execution trace
  - memory image for inspection
```

In the Dartmouth BASIC path, the intended end-to-end flow becomes:

```text
BASIC source
  → tokens
  → AST
  → Semantic IR
  → GE-225 backend
  → machine words
  → GE-225 simulator
  → output / trace
```

## Test Strategy

### Decoder tests

- decode known 20-bit instruction words into `(opcode, modifier, address)`
- verify field masking and packing
- verify illegal field combinations are rejected cleanly

### Execution tests

- load / store round-trips through memory
- add / subtract update `A` correctly
- unconditional branch updates `PC`
- conditional branch respects indicator state
- halt stops execution
- memory is always masked back to 20 bits

### Backend integration tests

- lower a tiny IR program (`x = 1 + 2`) into GE-225 words and verify result in memory
- lower a branch (`if x == 0 goto L1`) and verify control flow
- lower a simple print helper call through the runtime boundary

### Dartmouth BASIC pipeline tests

Once the frontend exists:

- compile `10 LET X = 1`
- compile `20 LET Y = X + 2`
- compile `30 END`
- verify generated code executes and stores the expected values

The first goal is not perfect historical fidelity. The first goal is a working,
debuggable native target that makes the Dartmouth BASIC pipeline real.

## Future Extensions

- full transcription of the documented GE-225 instruction set
- exact automatic modification / indexed-addressing behavior
- full fixed-point and floating-point instruction families
- GE-225 assembler and disassembler packages
- symbolic object format for GE-225 programs
- teletype I/O simulation
- punched-card / paper-tape loaders
- DATANET-30 / DTSS environment emulation
- Dartmouth BASIC runtime library targeting native GE-225 code

## Non-Goals

This spec deliberately does **not** require:

- a museum-grade recreation before the compiler can ship
- cycle-accurate hardware timing
- complete operating-system simulation
- every peripheral from day one

The success condition is simpler:

> If we can lower Semantic IR into real GE-225 instruction words, load them into
> a simulator, and execute them correctly, we have a historically meaningful and
> compiler-useful machine target.
