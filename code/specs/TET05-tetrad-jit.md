# TET05 — Tetrad JIT Compiler Specification

> **🪦 RETIRED (2026-04-25)** — the `tetrad-jit` package has been
> deleted.  Its responsibilities now live in:
>
> - **JIT engine** — `jit_core.JITCore` (spec
>   [LANG03](LANG03-jit-core.md)) drives tier promotion, type
>   guards, deopt, and the JIT cache; uses
>   `jit_core.BackendProtocol` for codegen.
> - **Intel 4004 backend** — `tetrad_runtime.Intel4004Backend`
>   implements `BackendProtocol` and bridges `CIRInstr` through to
>   the inlined `tetrad_runtime._intel4004_codegen` (the original
>   bytecode → 4004-asm → binary pipeline, kept verbatim).
> - **End-to-end runner** — `TetradRuntime.run_with_jit(source)` runs
>   the full Tetrad-on-LANG JIT path; functions the codegen does not
>   support transparently fall back to interpretation through
>   jit-core's standard deopt mechanism.
>
> The `_intel4004_codegen` subpackage is internal and will retire
> when a CIR-native 4004 backend lands (planned `intel4004-backend`
> package); ``Intel4004Backend`` becomes a one-line forwarder at
> that point.
>
> The text below is preserved as a historical record of what the
> retired implementation did.  Read it for context; do not start new
> work against `TetradJIT` — there is no `TetradJIT` anymore.

## Overview

The Tetrad JIT compiler is a **profile-guided Intel 4004 native code generator**.
It reads the feedback vectors and metrics collected by the VM (spec TET04),
identifies hot functions, and emits real Intel 4004 machine code that runs
those functions on the `intel4004-simulator` hardware model.

The JIT is the conceptual payoff of the entire Tetrad pipeline:

```
Tetrad source
  → lexer / parser / type-checker (TET00-TET02)
  → bytecode compiler (TET03)
  → TetradVM interpreter (TET04)
  → JIT: bytecode → JIT IR → Intel 4004 binary → simulator (TET05)
```

Targeting the Intel 4004 is architecturally intentional.  Tetrad's VM
constraints (u8 arithmetic, 4-frame call stack, 8 registers) were modelled
after the 4004.  The JIT closes the loop by producing code the 4004's
hardware model can actually execute.

This spec covers:

1. Hot-function detection and triggering
2. The JIT compilation pipeline (bytecode → IR → Intel 4004)
3. Optimization passes (constant folding, dead code, type specialisation)
4. Intel 4004 code generation using the `intel4004-simulator` package
5. The deoptimisation path (unsupported operations fall back to the interpreter)
6. The JIT code cache

---

## Why the Intel 4004?

The Intel 4004 (1971) is a 4-bit accumulator-based CPU with:

- 16 × 4-bit registers (R0–R15), grouped as 8 register pairs (P0–P7)
- A 4-bit accumulator (A) and carry flag (CY)
- A 3-level hardware call stack
- 4096 bytes of ROM program storage
- 640 nibbles of RAM

Tetrad values are u8 (8-bit).  The code generator stores each u8 in a
**register pair**: the high nibble in the even register, the low nibble in
the odd register.  For example, the value `0x2A` (42) in pair P2 means
`R4=2, R5=10` (0x2 and 0xA).

This is the same nibble-pair representation used by the Nib language backend
(spec NIB01).  Tetrad's JIT leverages the same `intel4004-simulator` package
to actually execute the compiled binary.

---

## JIT Architecture

```
VM executes program (interpreted)
    │
    │  metrics accumulate per function call / loop iteration
    ▼
Hot-function detector (threshold: 100 calls for UNTYPED)
    │
    │  hot function identified
    ▼
JIT Compiler
    ├── Step 1: Tetrad bytecode → JIT IR (SSA form)
    ├── Step 2: Optimization passes
    │     ├── Constant folding
    │     └── Dead code elimination
    ├── Step 3: Intel 4004 code generation
    │     ├── Register pair allocation (virtual var → P0–P5)
    │     ├── 8-bit arithmetic via nibble-pair sequences
    │     └── Two-pass assembler (labels → ROM addresses)
    └── Step 4: Cache compiled binary
    │
    ▼
Compiled function (Python bytes, Intel 4004 binary)
    │
    │  per call: load into Intel4004Simulator, set arg registers, run
    ▼
JIT Code Cache (fn_name → compiled bytes)
    │
    ▼
VM calls compiled version on next invocation
```

---

## Three-Tier Compilation Strategy

| Tier              | `type_status`      | JIT trigger      | Warmup cost     |
|-------------------|--------------------|------------------|-----------------|
| **FULLY_TYPED**   | `FULLY_TYPED`      | Before first call | None           |
| **PARTIALLY_TYPED** | `PARTIALLY_TYPED` | After 10 calls  | 10 interpreted  |
| **UNTYPED**       | `UNTYPED`          | After 100 calls  | 100 interpreted |

```python
THRESHOLDS = {
    FunctionTypeStatus.FULLY_TYPED:     0,   # compile immediately
    FunctionTypeStatus.PARTIALLY_TYPED: 10,
    FunctionTypeStatus.UNTYPED:         100,
}
```

---

## JIT IR (Intermediate Representation)

Between bytecode and 4004, the JIT uses a simple SSA-based IR.  Each IR
instruction operates on **virtual variables** (vN) rather than the accumulator
and registers.  This makes optimization passes simple: constant folding only
needs a values dict; DCE only needs a liveness set.

See `ir.py` for the `IRInstr` dataclass and `evaluate_op`.

### Bytecode → IR Translation

The translator (`translate.py`) maintains:

- `acc` — the SSA variable currently in the accumulator
- `regs[r]` — the SSA variable currently in Tetrad register R[r]

Key mappings:

```
LDA_IMM 42        →  v0 = const 42          ty=u8
LDA_ZERO          →  v1 = const 0           ty=u8
LDA_REG r         →  acc = regs[r]          (no IR instruction)
LDA_VAR i         →  v2 = load_var i        ty=u8
STA_REG r         →  regs[r] = acc          (no IR instruction)
STA_VAR i         →  store_var i, acc       ty=u8
ADD r, [slot]     →  v3 = add acc, regs[r]  ty=u8 or unknown
ADD_IMM n         →  v4 = add acc, n        ty=u8
EQ r              →  v5 = cmp_eq acc, regs[r]
JMP offset        →  jmp lbl_N
JZ offset         →  jz acc, lbl_N
RET               →  ret acc
```

Function parameters are pre-loaded into `regs[0..N-1]` via `param` IR
instructions at function entry.  Tetrad's compiled prologue
(`LDA_REG 0; STA_VAR 0`) is then correctly translated using those SSA vars.

---

## Optimization Passes

### Pass 1: Constant Folding

Evaluates operations on known-constant virtual variables at compile time.

```
v0 = const 10
v1 = const 5
v2 = add v0, v1    →    v2 = const 15   (folded via evaluate_op)
v3 = cmp_lt v0, v1 →    v3 = const 0    (10 < 5 = false)
```

Implementation: single forward pass with `values: dict[str, int | None]`.
`evaluate_op(op, a, b)` (from `ir.py`) computes the result.

### Pass 2: Dead Code Elimination

Removes IR instructions whose destination is never used.

```
v3 = add v1, v2    (v3 never appears in any srcs)  →   (removed)
```

Implementation: backward liveness pass.  Collect all `src` variable names;
any instruction whose `dst` is not in the live set (and has no side effects)
is dropped.

---

## Intel 4004 Code Generation

### Register Pair Convention

Each Tetrad u8 virtual variable maps to one 4004 register pair:

| Pair | Registers | Role |
|------|-----------|------|
| P0 (R0:R1) | R0=hi, R1=lo | Argument 0 / return value |
| P1 (R2:R3) | R2=hi, R3=lo | Argument 1 |
| P2 (R4:R5) | — | Local virtual var 0 |
| P3 (R6:R7) | — | Local virtual var 1 |
| P4 (R8:R9) | — | Local virtual var 2 |
| P5 (R10:R11) | — | Local virtual var 3 |
| P6 (R12:R13) | — | RAM address register (reserved) |
| P7 (R14:R15) | — | Scratch / immediate temp (reserved) |

If a function uses more than 2 params or more than 6 distinct virtual
variables, compilation fails and execution falls back to the interpreter.

### Supported and Unsupported Operations

**Supported in v1** (compiled to 4004 binary):

| IR op | 4004 sequence |
|-------|---------------|
| `const v, [imm]` | `FIM Pv, imm` (2 bytes) |
| `param v, [n]` | P0/P1 already set by caller; just records the mapping |
| `load_var v, [i]` | `FIM P6, 2i; SRC P6; RDM; XCH Rhi` × 2 nibbles |
| `store_var _, [i, v]` | `FIM P6, 2i; SRC P6; LD Rhi; WRM` × 2 nibbles |
| `add v, [a, b]` | `CLC; LD Rb_lo; ADD_R Ra_lo; XCH Rv_lo; LD Rb_hi; ADD_R Ra_hi; XCH Rv_hi` |
| `sub v, [a, b]` | `STC; LD Ra_lo; SUB_R Rb_lo; XCH Rv_lo; LD Ra_hi; SUB_R Rb_hi; XCH Rv_hi` |
| `cmp_lt v, [a, b]` | Subtract a−b, CMC, TCC → result in Rv |
| `cmp_le v, [a, b]` | Subtract b−a (no borrow ⟹ a≤b), TCC |
| `cmp_gt v, [a, b]` | Subtract b−a, CMC, TCC |
| `cmp_ge v, [a, b]` | Subtract a−b (no borrow ⟹ a≥b), TCC |
| `cmp_eq v, [a, b]` | Sub hi + JCN; sub lo + JCN; set 1 or 0 |
| `cmp_ne v, [a, b]` | Inverse of cmp_eq |
| `jmp _, [lbl]` | `JUN lbl` |
| `jz _, [v, lbl]` | Check hi nibble + lo nibble nonzero; `JUN lbl` if both zero |
| `jnz _, [v, lbl]` | `JCN 0xC, lbl` on hi; `JCN 0xC, lbl` on lo |
| `label _, [lbl]` | Label marker (zero bytes) |
| `ret _, [v]` | Copy Pv to P0 if needed; `HLT` |

**Unsupported in v1** (trigger deopt — fall back to interpreter):

`mul`, `div`, `mod`, `and`, `or`, `xor`, `not`, `shl`, `shr`,
`logical_not`, `io_in`, `io_out`, `call`, `deopt`.

Operations without direct 4004 equivalents (bitwise register-to-register ops,
division) are left to v2.

### 8-bit Add on the 4004

The 4004 ADD instruction is 4-bit: `A = A + Rr + CY`.  For u8 addition, we
add the low nibbles (carry-free) then the high nibbles (consuming the carry):

```
; u8 add: Pa + Pb → Pv   (Pa = R(2a):R(2a+1), etc.)
CLC                      ; clear carry for low nibble add
LD   R(2a+1)             ; A = lo(a)
ADD  R(2b+1)             ; A = lo(a) + lo(b) + 0;  carry ← lo overflow
XCH  R(2v+1)             ; R(2v+1) = result_lo;  carry preserved

LD   R(2a)               ; A = hi(a)
ADD  R(2b)               ; A = hi(a) + hi(b) + carry; high overflow discarded
XCH  R(2v)               ; R(2v) = result_hi  (u8 wrap automatic)
```

### 8-bit Subtract on the 4004

`SUB Rr: A = A + ~Rr + (1 if CY=0 else 0)`.  With `STC` (CY=1) the
first sub has no borrow-in; carry propagates naturally to the hi nibble:

```
; u8 sub: Pa - Pb → Pv
STC                      ; CY=1 (no borrow-in for lo)
LD   R(2a+1)             ; A = lo(a)
SUB  R(2b+1)             ; A = lo(a) − lo(b);  CY=1 if no borrow
XCH  R(2v+1)             ; save lo result;  carry preserved

LD   R(2a)               ; A = hi(a)
SUB  R(2b)               ; A = hi(a) − hi(b) − borrow; high overflow discarded
XCH  R(2v)               ; R(2v) = result_hi
```

### 8-bit Comparison on the 4004

`cmp_lt Pa, Pb → Pv` (result = 1 if Pa < Pb, else 0):

```
; Compute Pa − Pb; if final borrow (CY=0), Pa < Pb
STC
LD   R(2a+1)
SUB  R(2b+1)             ; lo subtract (lo result discarded)
LD   R(2a)
SUB  R(2b)               ; hi subtract;  CY=1 ⟹ no borrow ⟹ Pa ≥ Pb
CMC                      ; invert: CY=1 ⟹ Pa < Pb
TCC                      ; A = 1 if Pa<Pb, 0 otherwise;  clears carry
XCH  R(2v+1)             ; lo nibble = 0 or 1
LDM  0
XCH  R(2v)               ; hi nibble = 0
```

### Two-pass Assembler

The code generator produces a list of **abstract instructions** (tuples) and
then resolves them to binary bytes in two passes:

**Pass 1**: scan abstract instructions, compute byte offset of each one and
each label, build `label → byte_address` dict.

**Pass 2**: encode each instruction using the resolved label addresses.

Since all compiled functions are small (< 256 bytes), all code fits on
4004 ROM page 0 (addresses 0x000–0x0FF).  `JCN` page-relative addresses are
therefore just the low 8 bits of the absolute ROM address.

### Executing a Compiled Function

```python
def _run_on_4004(binary: bytes, args: list[int]) -> int:
    sim = Intel4004Simulator()
    sim.reset()
    sim.load_program(binary)
    sim._prepare_execution()
    # Load arguments into register pairs
    if len(args) >= 1:
        sim._write_pair(0, args[0] & 0xFF)  # P0 = arg0
    if len(args) >= 2:
        sim._write_pair(1, args[1] & 0xFF)  # P1 = arg1
    # Execute until HLT
    for _ in range(100_000):
        if sim.halted or sim._vm.pc >= len(sim._code.instructions):
            break
        sim.step()
    # Return value is in P0
    return sim._read_pair(0) & 0xFF
```

---

## JIT Code Cache

```python
@dataclass
class JITCacheEntry:
    fn_name: str
    binary: bytes               # Intel 4004 machine code
    param_count: int            # number of u8 arguments
    ir: list[IRInstr]           # post-optimization IR (for dump_ir)
    compilation_time_ns: int    # for benchmarking

class JITCache:
    def get(self, fn_name: str) -> JITCacheEntry | None: ...
    def put(self, entry: JITCacheEntry) -> None: ...
    def stats(self) -> dict[str, dict]: ...
```

---

## JIT Public API

```python
class TetradJIT:

    def __init__(self, vm: TetradVM): ...

    # Compile a function by name (looks up in main code loaded via execute_with_jit).
    # Returns True if compilation to 4004 binary succeeded; False if deopted.
    def compile(self, fn_name: str) -> bool: ...

    # Check if a function is compiled and cached.
    def is_compiled(self, fn_name: str) -> bool: ...

    # Execute a function:
    #   - If cached: run on Intel4004Simulator
    #   - If not cached: run via TetradVM interpreter
    def execute(self, fn_name: str, args: list[int]) -> int: ...

    # Run the whole program.  FULLY_TYPED functions are compiled before the
    # first interpreted instruction.  UNTYPED/PARTIALLY_TYPED functions are
    # compiled once they cross the call-count threshold.
    def execute_with_jit(self, code: CodeObject) -> int: ...

    # Return JIT cache statistics.
    def cache_stats(self) -> dict[str, dict]: ...

    # Return the post-optimization IR for a compiled function.
    def dump_ir(self, fn_name: str) -> str: ...
```

---

## Python Package

The JIT lives in `code/packages/python/tetrad-jit/`.

Dependencies:

- `coding-adventures-tetrad-vm` — the Tetrad VM to wrap
- `coding-adventures-intel4004-simulator` — to execute compiled binaries

Module layout:

```
tetrad_jit/
  ir.py          — IRInstr dataclass + ARITHMETIC_OPS + evaluate_op
  translate.py   — Tetrad bytecode → JIT IR
  passes.py      — constant folding + dead code elimination
  codegen_4004.py — IR → Intel 4004 binary (code gen + two-pass assembler)
  cache.py       — JITCache + JITCacheEntry
  __init__.py    — TetradJIT public class
```

---

## Test Strategy

### IR translation tests

- `LDA_IMM 42; RET` → `[const v0 42, ret v0]`
- Param pre-load: function with 2 params → `[param v0 0, param v1 1, ...]`
- Jump target labels: `JZ +2` at instruction 5 → `lbl_0` on instruction 8

### Optimization pass tests

- Constant folding: `const 10 + const 5` → `const 15`
- Constant fold cmp: `const 3 < const 5` → `const 1`
- DCE: unused `v3 = add v1, v2` → removed
- DCE: used result → kept

### Code generation tests

- `const 42` → 2-byte FIM; simulator reads P0 = 42
- `add P0, P1 → P0`: 3 + 4 = 7
- `sub P0, P1 → P0`: 10 − 3 = 7
- `sub P0, P1 → P0` with borrow: 3 − 10 = 249 (u8 wrap)
- `cmp_lt P0, P1`: 3 < 10 = 1; 10 < 3 = 0
- `cmp_eq P0, P1`: same value = 1; different = 0

### End-to-end JIT tests

Compile Tetrad source, JIT the function, verify results match interpreter:

- `fn add(a: u8, b: u8) -> u8 { return a + b; }` → add(200, 100) = 44
- `fn const42() -> u8 { return 42; }` → 42
- `fn double(n: u8) -> u8 { return n + n; }` → double(5) = 10
- Deopt case: `fn mul(a, b) { return a * b; }` → compile returns False; falls
  back to interpreter

### Hot-function detection tests

- FULLY_TYPED function compiled before first call in `execute_with_jit`
- UNTYPED function not compiled after 99 calls; compiled after 100th

### Coverage target

90%+ line coverage.  Platform-specific paths (simulator step loop) may be
excluded from the coverage denominator.

---

## Divergences from Previous Draft

The original TET05 draft specified x86-64 native code generation using Python
`ctypes` and `mmap`.  This approach was replaced because:

1. Tetrad's design inspiration is the Intel 4004 — the JIT should target the
   same hardware model.
2. x86-64 code generation is not portable (ARM64 host would need separate
   codegen or emulation).
3. The `intel4004-simulator` package already exists in this repo and provides
   a complete execution target.
4. Educational value: seeing Tetrad bytecode become real 4004 instructions
   demonstrates what a JIT actually does at the instruction level.

The JIT IR, optimization passes, hot-function detection thresholds, and
three-tier compilation strategy are unchanged from the draft.

---

## Version History

| Version | Date       | Description |
|---------|------------|-------------|
| 0.2.0   | 2026-04-21 | Retarget codegen from x86-64 to Intel 4004 |
| 0.1.0   | 2026-04-20 | Initial specification (x86-64) |
