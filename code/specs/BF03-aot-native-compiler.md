# BF03 — AOT Native Compiler (Decomposed Architecture)

## Overview

This spec describes a **native AOT (Ahead-of-Time) compiler pipeline** that
produces real, runnable executables. The pipeline uses a **general-purpose
intermediate representation (IR)** designed to serve any compiled language.
Brainfuck is the first frontend; BASIC is next. Each frontend compiles its
AST into the same IR, which then flows through a shared optimizer, backend,
and packager. This spec covers the pipeline architecture and the Brainfuck
frontend. It completes Path B of the architecture (spec 00):

```
Source → Lexer → Parser → AST → IR → Optimised IR → Machine Code → Executable
```

The pipeline is decomposed into **seven independent packages**, each with a
single responsibility and a well-defined interface. A **source-mapping sidecar**
flows through every stage, so that at any point — even inside the final
executable — we can trace a machine code byte back to the exact character in
the original Brainfuck source.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        Full Pipeline (7 packages)                          │
│                                                                             │
│  ┌──────────┐   ┌───────────┐   ┌──────────┐   ┌──────────┐   ┌────────┐  │
│  │ Brainfuck│   │           │   │          │   │          │   │        │  │
│  │ IR       │──▶│ IR        │──▶│ Machine  │──▶│ Exec     │──▶│  .elf  │  │
│  │ Compiler │   │ Optimizer │   │ Code Gen │   │ Packager │   │  .exe  │  │
│  │          │   │           │   │ (RISC-V) │   │          │   │  etc.  │  │
│  └────┬─────┘   └─────┬─────┘   └────┬─────┘   └────┬─────┘   └────────┘  │
│       │               │              │              │                      │
│  ┌────▼───────────────▼──────────────▼──────────────▼─────┐               │
│  │              SourceMap Sidecar (flows through all)      │               │
│  │  source pos → AST node → IR index → opt IR → MC offset │               │
│  └────────────────────────────────────────────────────────┘               │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Package Decomposition

### Package 1: `compiler-ir` — The Intermediate Representation

**What it is:** A **general-purpose**, language-agnostic, target-independent IR
type library. The IR is designed to serve as the compilation target for *any*
compiled language — not just Brainfuck. Brainfuck uses a small subset of the
instruction set; BASIC (the next planned frontend) will use more (variables,
strings, subroutines, floating-point arithmetic); future languages will use
the full set.

**What it is NOT:** It does not know about Brainfuck, BASIC, RISC-V, ELF, or
any specific language or target. It is pure data types.

**Design philosophy:** Start with the minimal instruction set needed for
Brainfuck (memory, arithmetic, control flow, syscalls). Each instruction is
designed with generality in mind — `LOAD_WORD`/`STORE_WORD` aren't "tape
operations", they're general memory access. When BASIC needs new capabilities
(e.g., `MUL`, `DIV`, `CALL_INDIRECT`, string operations), we add new opcodes
to this package. The existing opcodes never change semantics — only new ones
are appended. This keeps all existing frontends and backends forward-compatible.

**Why it's separate:** Every other package in the pipeline depends on these
types. Keeping them in their own package avoids circular dependencies and
makes the IR reusable across all source language frontends.

```
compiler-ir/
  src/
    types.{ext}          ← IrInstruction, IrRegister, IrImmediate, IrLabel, etc.
    opcodes.{ext}        ← IrOp enum (LOAD_IMM, ADD, BRANCH_Z, SYSCALL, ...)
    program.{ext}        ← IrProgram (instructions + data declarations + entry)
    printer.{ext}        ← IR → human-readable text (for debugging and tests)
    parser.{ext}         ← text → IR (for testing, golden-file comparisons)
    source_map.{ext}     ← SourceMap type (the sidecar that flows everywhere)
  tests/
```

#### IR Instruction Set

```
┌─────────────────────────────────────────────────────────────────────┐
│                        IR Instruction Set                           │
├──────────────┬──────────────────────────────────────────────────────┤
│ Category     │ Instructions                                        │
├──────────────┼──────────────────────────────────────────────────────┤
│ Constants    │ LOAD_IMM   reg, value                               │
│              │ LOAD_ADDR  reg, label                               │
│              │                                                      │
│ Memory       │ LOAD_BYTE  dst, base, offset                       │
│              │ STORE_BYTE src, base, offset                        │
│              │ LOAD_WORD  dst, base, offset                        │
│              │ STORE_WORD src, base, offset                        │
│              │                                                      │
│ Arithmetic   │ ADD        dst, lhs, rhs                            │
│              │ ADD_IMM    dst, src, value                          │
│              │ SUB        dst, lhs, rhs                            │
│              │ AND        dst, lhs, rhs                            │
│              │ AND_IMM    dst, src, value                          │
│              │                                                      │
│ Comparison   │ CMP_EQ     dst, lhs, rhs   (dst = lhs == rhs ? 1:0)│
│              │ CMP_NE     dst, lhs, rhs                            │
│              │ CMP_LT     dst, lhs, rhs                            │
│              │ CMP_GT     dst, lhs, rhs                            │
│              │                                                      │
│ Control Flow │ LABEL      name                                     │
│              │ JUMP       label                                    │
│              │ BRANCH_Z   reg, label       (jump if reg == 0)      │
│              │ BRANCH_NZ  reg, label       (jump if reg != 0)      │
│              │ CALL       label                                    │
│              │ RET                                                  │
│              │                                                      │
│ System       │ SYSCALL    number           (OS/simulator trap)     │
│              │ HALT                                                 │
│              │                                                      │
│ Meta         │ NOP                         (no operation)          │
│              │ COMMENT    text             (human-readable note)   │
└──────────────┴──────────────────────────────────────────────────────┘
```

#### Brainfuck Subset vs Full IR

Brainfuck uses only a subset of these instructions. The table below shows
which instructions each language frontend will use:

```
┌──────────────────────┬─────────┬───────┬────────────────────────────┐
│ Instruction          │ BF      │ BASIC │ Notes                      │
├──────────────────────┼─────────┼───────┼────────────────────────────┤
│ LOAD_IMM             │ ✓       │ ✓     │ constants                  │
│ LOAD_ADDR            │ ✓       │ ✓     │ data addresses             │
│ LOAD_BYTE            │ ✓       │       │ BF cell access             │
│ STORE_BYTE           │ ✓       │       │ BF cell access             │
│ LOAD_WORD            │         │ ✓     │ variable access            │
│ STORE_WORD           │         │ ✓     │ variable access            │
│ ADD / ADD_IMM        │ ✓       │ ✓     │ arithmetic                 │
│ SUB                  │         │ ✓     │ arithmetic                 │
│ AND / AND_IMM        │ ✓       │ ✓     │ masking / bitwise          │
│ CMP_EQ / NE / LT / GT│ ✓      │ ✓     │ comparisons                │
│ LABEL                │ ✓       │ ✓     │ jump targets               │
│ JUMP                 │ ✓       │ ✓     │ GOTO, loop back-edges      │
│ BRANCH_Z / NZ        │ ✓       │ ✓     │ conditionals, loops        │
│ CALL / RET           │         │ ✓     │ GOSUB/RETURN, DEF FN       │
│ SYSCALL              │ ✓       │ ✓     │ I/O (PRINT, INPUT)         │
│ HALT                 │ ✓       │ ✓     │ END / program exit         │
│ NOP / COMMENT        │ ✓       │ ✓     │ debugging                  │
└──────────────────────┴─────────┴───────┴────────────────────────────┘
```

#### Future Opcodes (added when needed)

The IR instruction set grows as new frontends require new capabilities.
These opcodes are **not implemented in v1** but are planned:

```
┌──────────────────────────────────────────────────────────────────────┐
│ Category     │ Planned Instructions         │ Needed For            │
├──────────────┼──────────────────────────────┼───────────────────────┤
│ Arithmetic   │ MUL       dst, lhs, rhs      │ BASIC: * operator    │
│              │ DIV       dst, lhs, rhs      │ BASIC: / and \ ops   │
│              │ MOD       dst, lhs, rhs      │ BASIC: MOD operator  │
│              │ NEG       dst, src            │ BASIC: unary minus   │
│              │                               │                       │
│ Bitwise      │ OR        dst, lhs, rhs      │ BASIC: OR operator   │
│              │ XOR       dst, lhs, rhs      │ BASIC: XOR           │
│              │ SHL       dst, src, amount    │ general bitwise      │
│              │ SHR       dst, src, amount    │ general bitwise      │
│              │                               │                       │
│ Comparison   │ CMP_LE    dst, lhs, rhs      │ BASIC: <= operator   │
│              │ CMP_GE    dst, lhs, rhs      │ BASIC: >= operator   │
│              │                               │                       │
│ Memory       │ LOAD_HALF  dst, base, off    │ 16-bit data access   │
│              │ STORE_HALF src, base, off    │ 16-bit data access   │
│              │ ALLOCA     reg, size          │ stack allocation     │
│              │                               │                       │
│ Control      │ CALL_INDIRECT  reg            │ function pointers    │
│              │ SWITCH    reg, [label, ...]   │ computed jumps       │
│              │                               │                       │
│ Conversions  │ INT_TO_FLOAT  dst, src        │ BASIC: numeric       │
│              │ FLOAT_TO_INT  dst, src        │ BASIC: INT(), FIX()  │
│              │ SIGN_EXTEND   dst, src, bits  │ type widening        │
│              │ ZERO_EXTEND   dst, src, bits  │ type widening        │
│              │                               │                       │
│ Float (opt)  │ FADD      dst, lhs, rhs      │ BASIC: float math    │
│              │ FSUB      dst, lhs, rhs      │ BASIC: float math    │
│              │ FMUL      dst, lhs, rhs      │ BASIC: float math    │
│              │ FDIV      dst, lhs, rhs      │ BASIC: float math    │
│              │ FCMP_*    dst, lhs, rhs      │ BASIC: float compare │
│              │                               │                       │
│ Strings      │ MEMCOPY   dst, src, len       │ BASIC: string ops   │
│              │ MEMSET    dst, val, len       │ BASIC: string init   │
└──────────────┴──────────────────────────────┴───────────────────────┘
```

**Rule for adding opcodes:** A new opcode is added only when a frontend
needs it *and* it cannot be efficiently expressed as a sequence of existing
opcodes. For example, `MUL` could be emulated with a loop of `ADD`s, but
that's absurdly inefficient — so it gets its own opcode. But `CMP_LE` could
be expressed as `CMP_GT + XOR 1` — we add it anyway for clarity and because
backends can emit a single instruction for it.

#### IR Register Types

Registers are **untyped in v1** — they hold machine-word-sized integers.
When floating-point support is added (for BASIC), we'll introduce typed
register classes:

```python
class RegisterClass(Enum):
    INTEGER = "i"     # general-purpose integer registers (v0, v1, ...)
    FLOAT   = "f"     # floating-point registers (f0, f1, ...) — future
```

This is a v2 concern. For now, all registers are integer.

The IR is **linear** (no basic blocks, no SSA, no phi nodes) and
**register-based** with infinite virtual registers (`v0`, `v1`, ...).
Backends are responsible for mapping virtual registers to physical ones.

#### IR Data Types

```python
@dataclass(frozen=True)
class IrRegister:
    index: int                           # v0, v1, v2, ...

@dataclass(frozen=True)
class IrImmediate:
    value: int

@dataclass(frozen=True)
class IrLabel:
    name: str

IrOperand = IrRegister | IrImmediate | IrLabel

@dataclass(frozen=True)
class IrInstruction:
    opcode: IrOp
    operands: tuple[IrOperand, ...]
    id: int                              # unique monotonic ID for source mapping

@dataclass
class IrDataDecl:
    label: str
    size: int
    init: int                            # initial byte value (usually 0)

@dataclass
class IrProgram:
    instructions: list[IrInstruction]
    data: list[IrDataDecl]
    entry_label: str
```

Every `IrInstruction` has a unique `id` field — a monotonically increasing
integer assigned at creation time. This ID is the key that the source map
uses to connect an IR instruction to its origin (AST node, source position)
and its destiny (optimised IR instruction, machine code offset).

#### IR Printer and Parser

The IR has a **canonical text format** that serves three purposes:
1. **Debugging** — humans can read the IR to understand what the compiler did
2. **Golden-file tests** — expected IR output is committed as `.ir` text files
3. **Roundtrip property** — `parse(print(program)) == program` is a testable invariant

```
; Example IR text format
.version 1

.data tape 30000 0

.entry _start

_start:
  LOAD_ADDR  v0, tape          ; #0
  LOAD_IMM   v1, 0             ; #1
  LOAD_BYTE  v2, v0, v1        ; #2
  ADD_IMM    v2, v2, 1          ; #3
  AND_IMM    v2, v2, 255        ; #4
  STORE_BYTE v2, v0, v1        ; #5
  HALT                          ; #6
```

The `.version N` directive is **required** as the first non-comment line.
It declares which IR version the file uses. Tools that encounter an
unsupported version must reject the file with a clear error rather than
silently misinterpreting opcodes. Version history:

| Version | Opcodes Added | Frontend |
|---------|---------------|----------|
| 1 | Core set (see table above) | Brainfuck |
| 2 | MUL, DIV, MOD, float ops, string ops, etc. | BASIC (planned) |

The `; #N` comments show the instruction IDs (informational, not parsed).

---

### Package 2: `compiler-source-map` — The Pipeline Sidecar

**What it is:** A chain of mappings that tracks every item's provenance as
it flows through the compiler pipeline, from original source character to
final machine code byte.

**Why it's separate:** Every stage of the pipeline reads and extends the
source map. Making it a shared dependency (rather than baking it into each
package) ensures a single, consistent data model.

**Design insight:** The source map is not one flat table. It is a **chain of
segment maps**, each connecting two adjacent representations:

```
┌───────────────────────────────────────────────────────────────────────────┐
│                    Source Map Chain                                       │
│                                                                           │
│  Segment 1       Segment 2        Segment 3        Segment 4             │
│  ──────────      ──────────       ──────────       ──────────            │
│  Source Pos      AST Node ID      IR Instr ID      IR Instr ID           │
│  (line, col)  →  (node_id)     →  (ir_id)       →  (opt_ir_id)          │
│                                                                           │
│  Segment 5                                                                │
│  ──────────                                                               │
│  IR Instr ID                                                              │
│  (opt_ir_id)  →  Machine Code Offset (byte offset in .text)              │
│                                                                           │
│  Composite query: source_pos → machine_code_offset                       │
│  Reverse query:   machine_code_offset → source_pos                       │
└───────────────────────────────────────────────────────────────────────────┘
```

Each segment is a **bidirectional mapping** between IDs in two adjacent
representations. Composing them gives end-to-end traceability.

#### Segment Types

```python
@dataclass
class SourcePosition:
    file: str
    line: int
    column: int
    length: int                  # character span in source

@dataclass
class SourceToAst:
    """Segment 1: source text positions → AST node IDs."""
    entries: list[tuple[SourcePosition, int]]   # (pos, ast_node_id)

@dataclass
class AstToIr:
    """Segment 2: AST node IDs → IR instruction IDs."""
    entries: list[tuple[int, list[int]]]         # (ast_node_id, [ir_ids])
    # One AST node may produce multiple IR instructions

@dataclass
class IrToIr:
    """Segment 3: IR instruction IDs → optimised IR instruction IDs."""
    entries: list[tuple[int, list[int]]]         # (original_ir_id, [new_ir_ids])
    # An instruction may be split, merged, or deleted
    deleted: set[int]                             # IDs that were optimised away
    pass_name: str                                # which pass produced this mapping

@dataclass
class IrToMachineCode:
    """Segment 4: IR instruction IDs → machine code byte offsets."""
    entries: list[tuple[int, int, int]]           # (ir_id, mc_offset, mc_length)

@dataclass
class SourceMapChain:
    """The full pipeline sidecar — a chain of segments."""
    source_to_ast: SourceToAst
    ast_to_ir: AstToIr
    ir_to_ir: list[IrToIr]                        # one per optimiser pass
    ir_to_machine_code: IrToMachineCode | None     # filled by backend

    def source_to_mc(self, pos: SourcePosition) -> int | None:
        """Compose all segments to get source → machine code offset."""

    def mc_to_source(self, mc_offset: int) -> SourcePosition | None:
        """Reverse: machine code offset → original source position."""

    def to_dwarf_line_table(self) -> bytes:
        """Export as DWARF .debug_line section."""

    def to_source_map_v3(self) -> str:
        """Export as JavaScript Source Map v3 JSON."""
```

#### Why a Chain, Not a Flat Table?

A flat table (machine code offset → source position) works for the final
consumer (debugger, profiler). But it doesn't help when you're debugging
the *compiler itself*:

- "Why did the optimiser delete instruction #42?" → Look at `IrToIr` for
  that pass. Was it dead code? Was it folded into another instruction?
- "Which AST node produced this IR?" → Look at `AstToIr`. Was it a loop
  node or a command node?
- "The machine code for this instruction seems wrong — what IR produced it?"
  → Look at `IrToMachineCode` in reverse.

The chain makes the compiler pipeline **transparent and debuggable at every
stage**. The flat composite mapping is just `compose(segment_1, ..., segment_n)`.

#### Serialisation

The source map chain is serialisable to the debug sidecar format (spec 05d)
and also to standard formats:

| Export Format | Consumer |
|---|---|
| Internal `.dbg` sidecar | Our debug adapter (spec 05e) |
| DWARF `.debug_line` | gdb, lldb, perf, valgrind |
| Source Map v3 `.map` | Chrome DevTools, browser tools |
| PDB | WinDbg, Visual Studio (future) |

---

### Package 3: `brainfuck-ir-compiler` — AST → IR

**What it is:** The Brainfuck-specific frontend that translates a Brainfuck
AST (from the parser, spec BF01) into the target-independent IR (from
package 1) and produces the first two segments of the source map chain.

**What it knows:** Brainfuck semantics — tape, cells, pointer, loops, I/O.

**What it does NOT know:** RISC-V, ARM, ELF, optimisation, machine code.

```
brainfuck-ir-compiler/
  src/
    compiler.{ext}         ← BrainfuckIrCompiler: AST → IrProgram
    build_mode.{ext}       ← BuildMode enum + BuildConfig
  tests/
    test_compiler.{ext}    ← unit: each AST node → correct IR
    test_debug.{ext}       ← bounds checks present in debug mode
    test_source_map.{ext}  ← AST→IR mapping segment is correct
    golden/                ← expected IR text files for diff-testing
```

#### Build Modes

Build modes are **pluggable and configurable**, not a hardcoded enum. A
`BuildConfig` object controls what the compiler emits:

```python
@dataclass
class BuildConfig:
    """Controls compiler behaviour. Modes are composable flags, not an enum."""
    insert_bounds_checks: bool = False    # tape pointer range checks
    insert_debug_locs: bool = True        # source location markers
    mask_byte_arithmetic: bool = True     # AND 0xFF after every cell mutation
    tape_size: int = 30_000              # configurable tape length

# Presets for convenience
DEBUG_CONFIG = BuildConfig(
    insert_bounds_checks=True,
    insert_debug_locs=True,
    mask_byte_arithmetic=True,
)

RELEASE_CONFIG = BuildConfig(
    insert_bounds_checks=False,
    insert_debug_locs=False,   # optimiser may want these; emitter strips
    mask_byte_arithmetic=True,  # correctness; backends can elide
)
```

New modes can be added without modifying existing code — just construct a
`BuildConfig` with the desired flags.

#### Compilation Mapping

```
┌────────────────────┬──────────────────────────────────────────────────┐
│ AST Node           │ IR Output                                        │
├────────────────────┼──────────────────────────────────────────────────┤
│ program            │ Prologue + body + epilogue                       │
│                    │                                                    │
│ command(RIGHT)     │ ADD_IMM v_ptr, v_ptr, 1                          │
│                    │                                                    │
│ command(LEFT)      │ ADD_IMM v_ptr, v_ptr, -1                         │
│                    │                                                    │
│ command(INC)       │ LOAD_BYTE  v_t, v_tape, v_ptr                   │
│                    │ ADD_IMM    v_t, v_t, 1                           │
│                    │ AND_IMM    v_t, v_t, 255    (if mask enabled)    │
│                    │ STORE_BYTE v_t, v_tape, v_ptr                   │
│                    │                                                    │
│ command(DEC)       │ LOAD_BYTE  v_t, v_tape, v_ptr                   │
│                    │ ADD_IMM    v_t, v_t, -1                          │
│                    │ AND_IMM    v_t, v_t, 255                         │
│                    │ STORE_BYTE v_t, v_tape, v_ptr                   │
│                    │                                                    │
│ command(OUTPUT)    │ LOAD_BYTE  v_t, v_tape, v_ptr                   │
│                    │ <move v_t to syscall arg register>               │
│                    │ SYSCALL    1                                      │
│                    │                                                    │
│ command(INPUT)     │ SYSCALL    2                                      │
│                    │ STORE_BYTE v_ret, v_tape, v_ptr                 │
│                    │                                                    │
│ loop(body)         │ LABEL      loop_N_start                          │
│                    │ LOAD_BYTE  v_t, v_tape, v_ptr                   │
│                    │ BRANCH_Z   v_t, loop_N_end                       │
│                    │ ...compile body...                                │
│                    │ JUMP       loop_N_start                          │
│                    │ LABEL      loop_N_end                             │
└────────────────────┴──────────────────────────────────────────────────┘
```

#### Debug Mode: Bounds Checking

When `insert_bounds_checks` is true, the compiler inserts checks before
every pointer move:

```
; Before: ADD_IMM v_ptr, v_ptr, 1  (move right)
; Debug build inserts:
CMP_GT      v_check, v_ptr, v_max     ; is ptr >= tape_size?
BRANCH_NZ   v_check, __trap_oob       ; if so, trap

; Before: ADD_IMM v_ptr, v_ptr, -1  (move left)
CMP_LT      v_check, v_ptr, v_zero    ; is ptr < 0?
BRANCH_NZ   v_check, __trap_oob
```

#### Source Map Output

The compiler produces two segments of the source map chain:

1. **SourceToAst** — from parser output (maps source positions to AST node IDs)
2. **AstToIr** — built during compilation (maps each AST node to the IR
   instruction IDs it produced)

For a single `+` command at line 1 col 3:
```
SourceToAst: (file="hello.bf", line=1, col=3, len=1) → ast_node_42
AstToIr:     ast_node_42 → [ir_7, ir_8, ir_9, ir_10]
                            (LOAD_BYTE, ADD_IMM, AND_IMM, STORE_BYTE)
```

---

### Package 4: `compiler-ir-optimizer` — IR → Optimised IR

**What it is:** A pass manager that transforms IR programs. Starts as a
**passthrough** (identity function) and incrementally gains optimisation
passes.

**Why it's separate:** Optimisation is optional. Debug builds skip it (or
run a subset). Release builds run all passes. The optimiser takes an
`IrProgram` in and produces an `IrProgram` out — same type, same contract.
The caller doesn't need to know what happened inside.

```
compiler-ir-optimizer/
  src/
    optimizer.{ext}        ← PassManager: run passes in order
    pass.{ext}             ← Pass interface / trait / protocol
    passes/
      identity.{ext}       ← passthrough (no-op, useful for testing)
      contraction.{ext}    ← fold consecutive same-ops: +++ → ADD_IMM 3
      clear_loop.{ext}     ← [-] → STORE_BYTE 0
      copy_loop.{ext}      ← [->+<] → add and clear
      scan_loop.{ext}      ← [>] → scan for zero cell
      dead_store.{ext}     ← remove stores overwritten before read
      constant_fold.{ext}  ← evaluate arithmetic on known constants
      mask_elision.{ext}   ← remove AND 0xFF before STORE_BYTE (ISA-aware hint)
  tests/
    test_identity.{ext}    ← passthrough produces identical output
    test_contraction.{ext} ← +++ → ADD_IMM 3
    test_clear_loop.{ext}  ← [-] optimisation
    test_source_map.{ext}  ← IrToIr mapping is correct through each pass
```

#### Pass Interface

```python
class OptimizationPass(Protocol):
    @property
    def name(self) -> str: ...

    def run(
        self,
        program: IrProgram,
        source_map: SourceMapChain,
    ) -> tuple[IrProgram, IrToIr]:
        """Transform the program; return new program + mapping segment."""
```

Every pass returns:
1. A new `IrProgram` (the transformed IR)
2. An `IrToIr` mapping segment (old instruction IDs → new instruction IDs)

This is the critical contract: **passes must explain what they did** to the
source map. A contraction pass that folds three `ADD_IMM 1` instructions
(IDs 7, 8, 9) into one `ADD_IMM 3` (ID 100) produces a mapping:

```
IrToIr(pass_name="contraction"):
  7  → [100]
  8  → [100]
  9  → [100]
```

A dead-store elimination pass that removes instruction 15 produces:

```
IrToIr(pass_name="dead_store_elimination"):
  15 → []   (deleted)
```

#### Pass Manager

```python
class PassManager:
    def __init__(self) -> None:
        self._passes: list[OptimizationPass] = []

    def add_pass(self, p: OptimizationPass) -> None:
        self._passes.append(p)

    def run(
        self,
        program: IrProgram,
        source_map: SourceMapChain,
    ) -> tuple[IrProgram, SourceMapChain]:
        """Run all passes in order, accumulating IrToIr segments."""
        for p in self._passes:
            program, ir_to_ir = p.run(program, source_map)
            source_map.ir_to_ir.append(ir_to_ir)
        return program, source_map
```

#### Initial Implementation: Identity Pass Only

```python
class IdentityPass:
    name = "identity"

    def run(self, program, source_map):
        # Passthrough: every instruction maps to itself
        mapping = IrToIr(
            entries=[(instr.id, [instr.id]) for instr in program.instructions],
            deleted=set(),
            pass_name=self.name,
        )
        return program, mapping
```

#### Future Passes

| Pass | Pattern | Transformation | Impact |
|------|---------|----------------|--------|
| **Contraction** | `+++` (N consecutive same-ops) | `ADD_IMM v, v, N` | 10× fewer instructions on real programs |
| **Clear loop** | `[-]` or `[+]` | `STORE_BYTE 0, tape, ptr` | Eliminates entire loop |
| **Copy loop** | `[->+<]` | Add cell[0] to cell[1], clear cell[0] | Major speedup for data movement |
| **Scan loop** | `[>]` or `[<]` | Scan for zero cell | O(1) vs O(n) |
| **Dead store** | Store then store again | Remove first store | Reduces memory traffic |
| **Constant fold** | Arithmetic on known constants | Evaluate at compile time | Free speedup |
| **Mask elision** | `AND 0xFF` before `STORE_BYTE` | Remove AND (byte store truncates) | ISA-aware hint, passes down to backend |

---

### Package 5: `codegen-riscv` — IR → RISC-V Machine Code

**What it is:** A backend that translates IR instructions into RISC-V
(RV32I) machine code bytes. It consumes an `IrProgram` and produces raw
binary + an `IrToMachineCode` source map segment.

**What it knows:** RISC-V instruction encoding, register allocation,
label resolution.

**What it does NOT know:** Brainfuck, ELF, optimisation, other ISAs.

```
codegen-riscv/
  src/
    backend.{ext}          ← RiscVBackend: IrProgram → MachineCodeResult
    register_alloc.{ext}   ← virtual → physical register mapping
    instruction_sel.{ext}  ← IR opcode → RISC-V instruction(s)
    label_resolver.{ext}   ← two-pass label resolution + patching
    encoding.{ext}         ← RISC-V binary encoding (or import from riscv-simulator)
  tests/
    test_each_ir_op.{ext}  ← unit: each IR instruction → correct RISC-V bytes
    test_labels.{ext}      ← forward/backward jumps resolve correctly
    test_source_map.{ext}  ← IrToMachineCode mapping is correct
```

#### Output Type

```python
@dataclass
class MachineCodeResult:
    code: bytes                                # raw machine code
    data_size: int                             # bytes needed for .bss
    entry_offset: int                          # byte offset of entry point
    symbols: dict[str, int]                    # label → byte offset
    ir_to_mc: IrToMachineCode                  # source map segment
```

#### Register Allocation

Brainfuck is simple enough for **fixed allocation** — no graph colouring
needed. BASIC will need a real allocator (variables, loop counters, string
pointers can easily exceed the number of physical registers), which is why
the backend uses a **strategy pattern** — swap in a new allocator without
changing anything else:

```python
class RegisterAllocator(Protocol):
    def allocate(self, program: IrProgram) -> dict[int, int]:
        """Map virtual register indices to physical register numbers."""

class FixedBrainfuckAllocator:
    """Fixed mapping for Brainfuck's small register needs."""
    MAPPING = {
        0: 8,    # v0 (tape base)   → s0 (x8)  — callee-saved
        1: 9,    # v1 (tape ptr)    → s1 (x9)  — callee-saved
        2: 5,    # v2 (temp)        → t0 (x5)  — scratch
        3: 6,    # v3 (temp2)       → t1 (x6)  — scratch
        4: 10,   # v4 (syscall arg) → a0 (x10) — argument/return
        5: 17,   # v5 (syscall num) → a7 (x17) — syscall number
    }
```

#### Instruction Selection

```
┌───────────────────────┬──────────────────────────────────────────────────┐
│ IR                    │ RISC-V                              │ MC bytes │
├───────────────────────┼─────────────────────────────────────┼──────────┤
│ LOAD_IMM  r, small    │ addi  rd, x0, imm                  │ 4        │
│ LOAD_IMM  r, large    │ lui rd, upper20                     │ 8        │
│                       │ addi rd, rd, lower12                │          │
│ LOAD_ADDR r, label    │ lui rd, %hi(label)                  │ 8        │
│                       │ addi rd, rd, %lo(label)             │          │
│ LOAD_BYTE d, b, off   │ add t2, base, off                  │ 8        │
│                       │ lb  dst, 0(t2)                      │          │
│ STORE_BYTE s, b, off  │ add t2, base, off                  │ 8        │
│                       │ sb  src, 0(t2)                      │          │
│ ADD_IMM   d, s, v     │ addi rd, rs, imm                   │ 4        │
│ ADD       d, l, r     │ add  rd, rs1, rs2                   │ 4        │
│ AND_IMM   d, s, v     │ andi rd, rs, imm                   │ 4        │
│ BRANCH_Z  r, lbl      │ beq  r, x0, offset                 │ 4        │
│ BRANCH_NZ r, lbl      │ bne  r, x0, offset                 │ 4        │
│ JUMP      lbl         │ jal  x0, offset                    │ 4        │
│ SYSCALL   n           │ addi a7, x0, n; ecall              │ 8        │
│ HALT                  │ addi a7, x0, 10; ecall             │ 8        │
│ LABEL     name        │ (no code — records symbol offset)  │ 0        │
│ NOP                   │ addi x0, x0, 0                     │ 4        │
│ COMMENT   text        │ (no code)                          │ 0        │
└───────────────────────┴─────────────────────────────────────┴──────────┘
```

#### Source Map: IrToMachineCode

For each IR instruction, the backend records the byte offset and length
of the machine code it produced:

```
IR instruction #7 (LOAD_BYTE) → MC offset 0x14, length 8 bytes
IR instruction #8 (ADD_IMM)   → MC offset 0x1C, length 4 bytes
IR instruction #9 (AND_IMM)   → MC offset 0x20, length 4 bytes
IR instruction #10 (STORE_BYTE)→ MC offset 0x24, length 8 bytes
```

The `IrToMachineCode` segment stores these triples:
`(ir_instruction_id, mc_byte_offset, mc_byte_length)`.

#### Label Resolution

Two-pass assembly, same pattern as the existing ARM assembler (spec 06):

1. **Pass 1:** Walk IR, emit placeholder bytes, record label offsets.
2. **Pass 2:** Patch branch/jump immediates using label offsets.

#### Future Backends

The backend interface is:

```python
class Backend(Protocol):
    def lower(
        self,
        program: IrProgram,
        config: BuildConfig,
    ) -> MachineCodeResult: ...
```

Future packages:
- `codegen-arm` — ARM32 backend
- `codegen-x86-64` — x86-64 backend
- `codegen-wasm` — WebAssembly backend

Each is a separate package implementing the same interface.

---

### Package 6: `executable-packager` — Machine Code → Executable File

**What it is:** Takes raw machine code bytes, data declarations, build
config, and the source map chain, and produces an OS-specific executable
file ready to run.

**What it knows:** Executable file formats (ELF, Mach-O, PE), debug info
formats (DWARF, PDB), section layout, alignment, entry points.

**What it does NOT know:** RISC-V, Brainfuck, IR, optimisation.

```
executable-packager/
  src/
    packager.{ext}         ← ExecutablePackager: top-level orchestrator
    build_profile.{ext}    ← BuildProfile: pluggable, composable config
    format.{ext}           ← OutputFormat enum + interface
    formats/
      elf.{ext}            ← ELF emitter (Linux)
      macho.{ext}          ← Mach-O emitter (macOS — future)
      pe.{ext}             ← PE/COFF emitter (Windows — future)
      raw.{ext}            ← Raw binary (for direct simulator loading)
    debug_info/
      dwarf.{ext}          ← DWARF .debug_line, .debug_info generation
      pdb.{ext}            ← PDB generation (future)
    sections.{ext}         ← Section types: .text, .data, .bss, .debug_*
  tests/
    test_elf.{ext}         ← ELF headers, segments, alignment correct
    test_dwarf.{ext}       ← DWARF line table encodes correctly
    test_debug_vs_release  ← debug sections present/absent as expected
```

#### Build Profiles

Build profiles are **pluggable and composable** — not a fixed enum. A
profile is a collection of flags that tell the packager what to include:

```python
@dataclass
class BuildProfile:
    """Configures what goes into the executable."""
    name: str
    include_debug_sections: bool = False   # .debug_line, .debug_info
    include_symbol_table: bool = False     # .symtab, .strtab
    include_source_map: bool = False       # embed full source map chain
    strip_nops: bool = False               # remove NOP padding
    section_alignment: int = 4096          # page alignment (OS-dependent)

# Preset profiles
DEBUG_PROFILE = BuildProfile(
    name="debug",
    include_debug_sections=True,
    include_symbol_table=True,
    include_source_map=True,
)

RELEASE_PROFILE = BuildProfile(
    name="release",
    include_debug_sections=False,
    include_symbol_table=False,
    include_source_map=False,
    strip_nops=True,
)

# Custom profiles are easy to create
PROFILING_PROFILE = BuildProfile(
    name="profiling",
    include_debug_sections=True,    # need line info for perf
    include_symbol_table=True,       # need function names
    include_source_map=False,        # don't need full chain
    strip_nops=True,                 # still want performance
)
```

#### Packager Interface

```python
class ExecutablePackager:
    def package(
        self,
        machine_code: MachineCodeResult,
        profile: BuildProfile,
        output_format: OutputFormat,
        source_map: SourceMapChain | None = None,
    ) -> bytes:
        """Package machine code into an executable."""

    def package_to_file(
        self,
        machine_code: MachineCodeResult,
        output_path: str,
        profile: BuildProfile,
        output_format: OutputFormat,
        source_map: SourceMapChain | None = None,
    ) -> None:
        """Package and write to disk."""
```

#### ELF Layout

```
┌─────────────────────────────────────────┐
│ ELF Header (52 bytes for 32-bit)        │
│   e_ident:    7f 45 4c 46 (ELF magic)   │
│   e_type:     ET_EXEC                    │
│   e_machine:  EM_RISCV (243)            │
│   e_entry:    0x10000 + entry_offset    │
├─────────────────────────────────────────┤
│ Program Header 1: .text segment         │
│   type: PT_LOAD                         │
│   flags: PF_R | PF_X (read + execute)  │
│   vaddr: 0x10000                        │
│   filesz: code.len()                    │
├─────────────────────────────────────────┤
│ Program Header 2: .bss segment          │
│   type: PT_LOAD                         │
│   flags: PF_R | PF_W (read + write)    │
│   vaddr: 0x10000 + aligned(code.len()) │
│   memsz: data_size (30000 for tape)    │
│   filesz: 0 (zero-initialized)         │
├─────────────────────────────────────────┤
│ .text section (machine code bytes)      │
├─────────────────────────────────────────┤
│ .debug_line section (debug profile)     │  ← only in debug builds
│ .debug_info section (debug profile)     │  ← only in debug builds
├─────────────────────────────────────────┤
│ .symtab section (if symbol table)       │  ← only if profile says so
│ .strtab section (string table)          │  ← only if profile says so
├─────────────────────────────────────────┤
│ Section Headers                         │
└─────────────────────────────────────────┘
```

#### DWARF Generation

The DWARF emitter converts the composed source map chain into standard
DWARF sections:

```python
class DwarfEmitter:
    def emit_debug_line(self, source_map: SourceMapChain) -> bytes:
        """Generate .debug_line section from source map chain.

        Composes all segments (source→AST→IR→optIR→MC) to produce
        the final source_position → mc_offset mapping, then encodes
        it as a DWARF line number program.
        """

    def emit_debug_info(
        self, source_map: SourceMapChain, symbols: dict[str, int]
    ) -> bytes:
        """Generate .debug_info section with compilation unit and
        variable descriptions.
        """
```

---

### Package 7: `riscv-assembler` — Assembly Text → Machine Code

**What it is:** A two-pass assembler that parses RISC-V assembly text and
produces machine code bytes. This is the RISC-V counterpart of the existing
ARM assembler (spec 06).

**Why it's separate from `codegen-riscv`:** The codegen works at the IR
level and uses encoding functions directly. The assembler works at the text
level — it's a tool for humans (and for the simulator's `load_assembly()`
feature). Different consumers, different interfaces.

```
riscv-assembler/
  src/
    assembler.{ext}        ← RiscVAssembler: text → AssemblyResult
    parser.{ext}           ← parse "addi x1, x0, 42" → ParsedInstruction
    encoder.{ext}          ← ParsedInstruction → u32 (or import encode_*)
    symbols.{ext}          ← SymbolTable for two-pass label resolution
    directives.{ext}       ← .data, .text, .bss, .global directives
  tests/
    test_parse.{ext}       ← each mnemonic parses correctly
    test_encode.{ext}      ← each instruction encodes correctly
    test_labels.{ext}      ← forward/backward label resolution
    test_programs.{ext}    ← multi-instruction programs assemble correctly
```

---

## RISC-V Simulator Upgrades

The existing RISC-V simulator (spec 07a) needs upgrades to serve as the
execution target:

### 1. ELF Loader

```python
def load_elf(self, elf_bytes: bytes) -> None:
    """Parse ELF, load segments into memory, set PC to entry point."""
```

### 2. System Call Dispatch

Extend `ecall` to dispatch based on `a7`:

| a7 | Name | Behaviour |
|----|------|-----------|
| 1 | write | Write byte in `a0` to stdout buffer |
| 2 | read | Read byte from stdin buffer into `a0` (-1 for EOF) |
| 10 | exit | Halt with exit code in `a0` |

### 3. Configurable Memory

Accept memory size parameter (default 256 KiB, up from 64 KiB).

### 4. Debug Support

- **Breakpoints:** Halt at a given PC
- **Single-step:** Execute one instruction, then pause
- **Register/memory inspection:** Read state at any point
- **Source mapping:** Given a PC + loaded source map, look up source line

### 5. Assembly Text Input

```python
def load_assembly(self, asm_text: str) -> None:
    """Assemble text, load into memory, set PC."""
    # Uses riscv-assembler package
```

---

## End-to-End Pipeline

The `brainfuck` package (or a CLI tool) orchestrates the full pipeline:

```python
class BrainfuckNativeCompiler:
    def compile(
        self,
        source: str,
        backend: Backend = RiscVBackend(),
        build_config: BuildConfig = DEBUG_CONFIG,
        build_profile: BuildProfile = DEBUG_PROFILE,
        output_format: OutputFormat = OutputFormat.ELF,
        optimizer_passes: list[OptimizationPass] | None = None,
    ) -> bytes:
        # 1. Lex + Parse (existing packages)
        tokens = Brainfuck.Lexer.tokenize(source)
        ast = Brainfuck.Parser.parse(tokens)

        # 2. AST → IR (brainfuck-ir-compiler)
        ir_compiler = BrainfuckIrCompiler(build_config)
        program, source_map = ir_compiler.compile(ast)

        # 3. Optimise IR (compiler-ir-optimizer)
        optimizer = PassManager()
        if optimizer_passes:
            for p in optimizer_passes:
                optimizer.add_pass(p)
        else:
            optimizer.add_pass(IdentityPass())  # passthrough
        program, source_map = optimizer.run(program, source_map)

        # 4. IR → Machine Code (codegen-riscv or other backend)
        mc_result = backend.lower(program, build_config)
        source_map.ir_to_machine_code = mc_result.ir_to_mc

        # 5. Machine Code → Executable (executable-packager)
        packager = ExecutablePackager()
        return packager.package(
            mc_result, build_profile, output_format, source_map
        )
```

---

## Dependency Graph

```
compiler-source-map          ← depends on nothing (pure data types)
    ▲
compiler-ir                  ← depends on compiler-source-map
    ▲
    ├── brainfuck-ir-compiler ← depends on compiler-ir, compiler-source-map,
    │                            brainfuck-parser (spec BF01)
    │
    ├── compiler-ir-optimizer ← depends on compiler-ir, compiler-source-map
    │
    ├── codegen-riscv         ← depends on compiler-ir, compiler-source-map
    │       │
    │       └── (optionally) riscv-simulator (for encode_* helpers)
    │
    └── executable-packager   ← depends on compiler-source-map
            │
            └── (optionally) codegen-riscv (for MachineCodeResult type,
                 but really just needs the generic interface)

riscv-assembler              ← depends on riscv-simulator (encode_* helpers)

riscv-simulator              ← upgraded: ELF loader uses executable-packager types
```

---

## Source Map: Full Worked Example

For Brainfuck source `+.` at line 1 cols 1-2:

### Segment 1: SourceToAst

```
(file="hello.bf", line=1, col=1, len=1) → ast_node_0  (command INC)
(file="hello.bf", line=1, col=2, len=1) → ast_node_1  (command OUTPUT)
```

### Segment 2: AstToIr

```
ast_node_0 → [ir_2, ir_3, ir_4, ir_5]
              LOAD_BYTE, ADD_IMM, AND_IMM, STORE_BYTE

ast_node_1 → [ir_6, ir_7]
              LOAD_BYTE, SYSCALL
```

(ir_0, ir_1 are the prologue — LOAD_ADDR and LOAD_IMM — and map to no
specific AST node.)

### Segment 3: IrToIr (identity pass)

```
ir_0 → [ir_0], ir_1 → [ir_1], ..., ir_7 → [ir_7]
```

### Segment 4: IrToMachineCode

```
ir_0  (LOAD_ADDR)  → MC offset 0x00, length 8
ir_1  (LOAD_IMM)   → MC offset 0x08, length 4
ir_2  (LOAD_BYTE)  → MC offset 0x0C, length 8
ir_3  (ADD_IMM)    → MC offset 0x14, length 4
ir_4  (AND_IMM)    → MC offset 0x18, length 4
ir_5  (STORE_BYTE) → MC offset 0x1C, length 8
ir_6  (LOAD_BYTE)  → MC offset 0x24, length 8
ir_7  (SYSCALL)    → MC offset 0x2C, length 8
```

### Composed: Source → Machine Code

```
"+" at line 1 col 1  →  MC bytes 0x0C..0x23  (20 bytes of RISC-V)
"." at line 1 col 2  →  MC bytes 0x24..0x33  (16 bytes of RISC-V)
```

### Reverse: MC offset 0x14 → ?

```
0x14  is in  ir_3 (ADD_IMM)
ir_3  came from  ir_3 (identity pass)
ir_3  belongs to  ast_node_0
ast_node_0  is at  line 1, col 1 of hello.bf
→ "the ADD_IMM at byte 0x14 is from the '+' on line 1 col 1"
```

---

## Test Strategy

### Per-Package Tests

| Package | Key Test Cases |
|---------|----------------|
| `compiler-ir` | Print→parse roundtrip; instruction ID uniqueness |
| `compiler-source-map` | Compose segments; reverse lookup; empty chain |
| `brainfuck-ir-compiler` | Each AST node → correct IR; debug bounds checks; golden files |
| `compiler-ir-optimizer` | Identity pass preserves IR; contraction folds correctly; source map preserved |
| `codegen-riscv` | Each IR op → correct RISC-V bytes; label resolution; register mapping |
| `executable-packager` | ELF headers correct; debug sections present/absent; alignment |
| `riscv-assembler` | Each mnemonic parses; labels resolve; roundtrip with encoder |

### Integration Tests

| Test | Pipeline |
|------|----------|
| Hello World | BF source → IR → RISC-V → raw bytes → simulator → verify output |
| Hello World (ELF) | BF source → IR → RISC-V → ELF → simulator ELF loader → verify |
| Cat program | `,.[,.]` with simulated input → verify echo |
| Nested loops | Complex control flow → verify correct execution |
| Cell wrapping | 255+1=0, 0-1=255 → verify |
| Debug breakpoint | Compile debug → load in simulator → break at line → inspect tape |
| Source map round-trip | Compile → pick random MC offset → reverse lookup → verify source pos |
| Optimised vs unoptimised | Same program, both modes → same output, different code size |

### End-to-End Golden Tests

For each test Brainfuck program, commit:
1. `.bf` — source
2. `.ir` — expected IR text (from printer)
3. `.opt.ir` — expected optimised IR (when optimiser is implemented)
4. `.riscv.s` — expected RISC-V assembly text (from a disassembler, future)

Tests verify that the compiler produces output matching the golden files.
When the compiler intentionally changes output (e.g., new optimisation),
update the golden files and explain in the commit message.

---

## Implementation Order

```
Phase 1: Core Types
  1.1  compiler-source-map  — SourceMapChain, all segment types
  1.2  compiler-ir          — IrInstruction, IrProgram, printer, parser

Phase 2: Brainfuck Frontend
  2.1  brainfuck-ir-compiler — AST → IR with source map segments 1+2
  2.2  Golden-file tests for IR output

Phase 3: Passthrough Optimiser
  3.1  compiler-ir-optimizer — PassManager + IdentityPass
  3.2  Verify source map chain integrity through identity pass

Phase 4: RISC-V Backend
  4.1  codegen-riscv — instruction selection + register allocation
  4.2  Label resolution (two-pass)
  4.3  Source map segment 4 (IrToMachineCode)
  4.4  Test: BF → IR → RISC-V bytes → run on simulator

Phase 5: Simulator Upgrades
  5.1  Syscall dispatch in riscv-simulator
  5.2  Configurable memory size
  5.3  End-to-end: BF → raw bytes → simulator → verify output

Phase 6: Executable Packaging
  6.1  executable-packager — ELF format, debug/release profiles
  6.2  ELF loader in riscv-simulator
  6.3  End-to-end: BF → ELF → simulator → verify output

Phase 7: Debug Tooling
  7.1  DWARF .debug_line generation from source map chain
  7.2  Breakpoints + single-step in simulator
  7.3  Source-level debugging demo

Phase 8: RISC-V Assembler
  8.1  riscv-assembler — text parser + two-pass assembly
  8.2  Integrate with simulator: load_assembly()

Phase 9: Optimisation Passes (incremental)
  9.1  Contraction pass
  9.2  Clear-loop pass
  9.3  Copy-loop pass
  9.4  Mask elision pass
  9.5  Each pass: verify source map chain integrity
```

---

## Future Frontend: BASIC

The IR is designed to grow. Here's a preview of what BASIC will need and how
it maps to the IR architecture:

| BASIC Feature | IR Requirement | Status |
|---|---|---|
| `LET X = 5` | `STORE_WORD` to named data label | v1 opcodes sufficient |
| `PRINT X` | `LOAD_WORD` + `SYSCALL` (format + write) | v1, but needs format helper |
| `INPUT X` | `SYSCALL` (read) + parse + `STORE_WORD` | v1, but needs parse helper |
| `IF...THEN` | `CMP_*` + `BRANCH_Z` | v1 opcodes sufficient |
| `FOR...NEXT` | `LABEL` + `CMP_*` + `BRANCH_*` + `ADD_IMM` | v1 opcodes sufficient |
| `GOTO N` | `JUMP label_N` | v1 opcodes sufficient |
| `GOSUB/RETURN` | `CALL` / `RET` | v1 opcodes sufficient |
| `A * B` | `MUL dst, lhs, rhs` | **Needs new opcode** |
| `A / B` | `DIV dst, lhs, rhs` | **Needs new opcode** |
| `A$ = "hello"` | String data decl + `MEMCOPY` | **Needs new opcodes** |
| `3.14 * R` | `FMUL` + float registers | **Needs float opcodes + register classes** |
| `DEF FN` | `CALL` with argument passing convention | v1 opcodes, needs ABI convention |

The BASIC frontend (`basic-ir-compiler`) will be a separate package following
the same pattern as `brainfuck-ir-compiler`. It will:
1. Depend on `compiler-ir` and `compiler-source-map`
2. Produce an `IrProgram` + `SourceToAst` + `AstToIr` segments
3. Share the same optimizer, backend, and packager

When BASIC implementation begins, we'll add the needed opcodes to
`compiler-ir` and any new optimization passes to `compiler-ir-optimizer`
(e.g., constant propagation for `LET X = 3 + 4`).

---

## Open Questions

1. **Byte masking in IR.** The IR includes `AND_IMM v, v, 255` for
   correctness. The mask-elision optimiser pass can remove it when the
   backend guarantees byte-width stores. Is this the right split?
   Recommendation: yes.

2. **Tape allocation.** Static .bss (simple, fixed 30 KB) vs. dynamic
   allocation via syscall (flexible). Recommendation: .bss for v1.

3. **I/O convention.** Raw syscalls (platform-specific) vs. helper functions
   (runtime library). Recommendation: raw syscalls for simulator, helper
   functions for real OS targets (future).

4. **Reference language.** Start with one language (Go or Rust), port later?
   Or implement in all 7+ languages from the start? Recommendation: single
   reference implementation first.

5. **IR serialisation.** Should IR be writable to disk as a `.ir` file,
   enabling separate compilation and inspection? Recommendation: yes — the
   text format via printer/parser serves this need.

6. **Source map granularity.** Should the source map track individual
   characters (col+length) or just lines? Recommendation: character-level
   for Brainfuck (where each character is a command), line-level as minimum
   for other languages.

7. **Optimiser pass ordering.** Should passes declare dependencies on each
   other (e.g., "run contraction before copy-loop detection")?
   Recommendation: not for v1 — manual ordering in the pass manager is
   sufficient.

8. **Runtime library.** BASIC needs formatted I/O (`PRINT`, `INPUT`), string
   operations, and potentially garbage collection. Should there be a
   `compiler-runtime` package providing helper functions callable via `CALL`?
   Recommendation: yes, as a future package. Brainfuck doesn't need it.
