# TET05 — Tetrad JIT Compiler Specification

## Overview

The Tetrad JIT compiler is a **profile-guided native code generator**. It reads the
feedback vectors and metrics collected by the VM (spec TET04), identifies hot functions
and loops, and emits x86-64 machine code that runs those functions at hardware speed
rather than interpreted speed.

The JIT is the payoff for the careful design of the feedback vector and metrics API.
Every design decision in TET03 (feedback slots) and TET04 (metrics layer) was made with
this consumer in mind.

This spec covers:
1. Hot-function detection and triggering
2. The JIT compilation pipeline (bytecode → IR → x86-64)
3. Optimization passes (constant folding, dead code, type specialization)
4. x86-64 code generation using Python `ctypes`
5. On-stack replacement (OSR) for hot loops
6. The deoptimization path (when speculation fails)

---

## Why a JIT? The Lisp Context

Most Lisp implementations today (SBCL, Racket, Guile, Chez) have AOT compilers or
simple interpreters. SBCL has a native compiler but it is not a tracing/profiling JIT —
it compiles whole functions with type declarations, not hot paths with type feedback.

The gap: **no major Lisp VM exposes a clean metrics API** that an external JIT can
consume to make type-specialization decisions. Tetrad fills this gap by design.

A future Lisp front-end that compiles to Tetrad bytecode will get the JIT almost for
free: the feedback vectors it generates will tell the JIT which slots are
monomorphic integer operations (compile to a single `add` instruction), which are
polymorphic (compile a type dispatch), and which are megamorphic (bail to the
interpreter). This is exactly how V8 makes JavaScript fast.

---

## JIT Architecture

```
VM executes program (interpreted)
    │
    │  metrics accumulate per instruction
    ▼
Hot-function detector (threshold: 100 calls or 500 loop iterations)
    │
    │  hot function identified
    ▼
JIT Compiler
    ├── Step 1: Bytecode → JIT IR (SSA form)
    ├── Step 2: Optimization passes
    │     ├── Constant folding
    │     ├── Dead code elimination
    │     ├── Type specialization (using feedback vectors)
    │     └── Branch layout (hot path first)
    ├── Step 3: Register allocation (linear scan)
    └── Step 4: x86-64 code generation → bytes
    │
    ▼
Compiled function (Python bytes object)
    │
    │  ctypes maps bytes to callable
    ▼
JIT Code Cache (fn_name → callable)
    │
    ▼
VM calls compiled version on next invocation
```

When a compiled function needs a feature the JIT doesn't support (deoptimization
trigger), it falls back to the interpreter for that activation.

---

## Three-Tier Compilation Strategy

Optional type annotations create three distinct compilation tiers. Each tier has a
different trigger for when JIT compilation occurs:

| Tier | `CodeObject.type_status` | JIT trigger | Warmup cost |
|---|---|---|---|
| **FULLY_TYPED** | `FULLY_TYPED` | Compiled **before first call** (from `immediate_jit_queue`) | None |
| **PARTIALLY_TYPED** | `PARTIALLY_TYPED` | Compiled after **10 calls** | 10 interpreted runs |
| **UNTYPED** | `UNTYPED` | Compiled after **100 calls** or 500 loop iterations | 100 interpreted runs |

```python
THRESHOLDS = {
    FunctionTypeStatus.FULLY_TYPED:     0,    # immediate — drain from queue before main
    FunctionTypeStatus.PARTIALLY_TYPED: 10,   # quick warmup for partially annotated code
    FunctionTypeStatus.UNTYPED:         100,  # conservative — avoid compiling cold code
}

LOOP_ITERATION_THRESHOLD = 500   # OSR trigger (all tiers)

def should_jit(vm: TetradVM, fn_name: str, code: CodeObject) -> bool:
    if code.immediate_jit_eligible:
        return True   # already queued before main; this path should not be reached
    threshold = THRESHOLDS[code.type_status]
    call_count = vm.metrics().function_call_counts.get(fn_name, 0)
    if call_count >= threshold:
        return True
    loop_iters = vm.loop_iterations(fn_name)
    if any(count >= LOOP_ITERATION_THRESHOLD for count in loop_iters.values()):
        return True
    return False
```

### Immediate Compilation (FULLY_TYPED)

`execute_with_jit` drains the `immediate_jit_queue` before running any bytecode:

```python
def execute_with_jit(self, code: CodeObject) -> int:
    # Phase 1: compile all FULLY_TYPED functions before the interpreter starts
    for fn_name in self.vm.metrics.immediate_jit_queue:
        fn_code = find_function(code, fn_name)
        self.compile(fn_code)   # emits x86-64; puts in cache

    # Phase 2: run the interpreter; hot PARTIALLY_TYPED/UNTYPED fns compile as they warm up
    return self.vm.execute(code)
```

The JIT can compile typed functions immediately because it does not need feedback data
for them: the type annotations guarantee `u8 × u8 → u8` for every op. The generated
code has no type guards and no deopt paths.

### Optimized Code for Typed Functions

For a FULLY_TYPED function, the JIT skips the type-specialization pass (Pass 3) because
the type checker already guaranteed all operands are `u8`. The emitted x86-64 is clean
integer arithmetic with no type guards:

```
# Typed fn add(a: u8, b: u8) -> u8 { return a + b; }
push rbp
mov  rbp, rsp
mov  rax, rdi      # load a
add  rax, rsi      # a + b
and  rax, 0xFF     # u8 wrap
pop  rbp
ret

# No type checks. No deopt calls. No guards. Pure arithmetic.
```

For PARTIALLY_TYPED and UNTYPED functions, the type-specialization pass (Pass 3) runs
as before, reading feedback slots to determine which ops can be specialized.

---

## JIT IR (Intermediate Representation)

Between bytecode and x86-64, the JIT uses a simple SSA-based IR. Each IR instruction
operates on **virtual variables** (vN) rather than the accumulator and registers. This
makes the optimization passes easier to implement.

### IR Instructions

```python
@dataclass
class IRInstr:
    op: str              # operation name
    dst: str | None      # destination virtual variable (e.g. "v3"), or None for effects
    srcs: list[str|int]  # source virtual variables or integer constants
    ty: str              # type annotation: "u8" | "unknown" (from feedback)
    comment: str = ""    # optional debug comment
```

### IR Instruction Set

| Op | Effect |
|---|---|
| `const` | `dst = srcs[0]` (integer constant) |
| `load_var` | `dst = vars[srcs[0]]` |
| `store_var` | `vars[srcs[0]] = srcs[1]` |
| `add` | `dst = (srcs[0] + srcs[1]) % 256` |
| `sub` | `dst = (srcs[0] - srcs[1]) % 256` |
| `mul` | `dst = (srcs[0] * srcs[1]) % 256` |
| `div` | `dst = srcs[0] / srcs[1]` |
| `mod` | `dst = srcs[0] % srcs[1]` |
| `and` | `dst = srcs[0] & srcs[1]` |
| `or` | `dst = srcs[0] \| srcs[1]` |
| `xor` | `dst = srcs[0] ^ srcs[1]` |
| `not` | `dst = ~srcs[0] & 0xFF` |
| `shl` | `dst = (srcs[0] << srcs[1]) & 0xFF` |
| `shr` | `dst = srcs[0] >> srcs[1]` |
| `cmp_eq` | `dst = 1 if srcs[0] == srcs[1] else 0` |
| `cmp_lt` | `dst = 1 if srcs[0] < srcs[1] else 0` |
| (other cmps) | similar |
| `jmp` | unconditional jump to `srcs[0]` (label name) |
| `jz` | jump to `srcs[1]` if `srcs[0] == 0` |
| `jnz` | jump to `srcs[1]` if `srcs[0] != 0` |
| `label` | marks a branch target |
| `call` | `dst = call srcs[0](srcs[1..])` |
| `ret` | return `srcs[0]` |
| `io_in` | `dst = read_io()` |
| `io_out` | write `srcs[0]` to io |
| `deopt` | bail to interpreter |

### Bytecode → IR Translation

The translation is a straight-line pass that creates a new virtual variable for each
`LDA_*` result and each arithmetic result:

```
LDA_IMM 42        →  v0 = const 42          type=u8
STA_REG r0        →  (r0 slot = v0)
LDA_VAR x         →  v1 = load_var "x"      type=u8
ADD r0, slot=0    →  v2 = add v1, v0        type=u8 (from feedback: monomorphic u8)
STA_VAR x         →  store_var "x", v2
```

The accumulator is modelled as an implicit "current value" tracked by the translator,
not as an IR variable. This keeps the IR clean.

---

## Optimization Passes

### Pass 1: Constant Folding

Evaluates operations on known constants at compile time:

```
v0 = const 10
v1 = const 5
v2 = add v0, v1    →    v2 = const 15   (folded)
```

Implementation: forward pass that maintains a `values: dict[str, int | None]` map.
If both sources are known constants, evaluate and replace with `const`.

```python
def constant_fold(ir: list[IRInstr]) -> list[IRInstr]:
    values: dict[str, int | None] = {}
    result = []
    for instr in ir:
        if instr.op == "const":
            values[instr.dst] = instr.srcs[0]
            result.append(instr)
        elif instr.op in ARITHMETIC_OPS:
            a = values.get(instr.srcs[0]) if isinstance(instr.srcs[0], str) else instr.srcs[0]
            b = values.get(instr.srcs[1]) if isinstance(instr.srcs[1], str) else instr.srcs[1]
            if a is not None and b is not None:
                folded = evaluate_op(instr.op, a, b)
                values[instr.dst] = folded
                result.append(IRInstr(op="const", dst=instr.dst, srcs=[folded], ty="u8"))
            else:
                values[instr.dst] = None
                result.append(instr)
        else:
            result.append(instr)
    return result
```

### Pass 2: Dead Code Elimination

Removes IR instructions whose destination is never read:

```
v3 = add v1, v2    (v3 never used)   →   (removed)
```

Implementation: backward liveness pass. Build a set of "live" virtual variables by
scanning uses in reverse. Any `dst` not in the live set is dead.

```python
def dead_code_eliminate(ir: list[IRInstr]) -> list[IRInstr]:
    live: set[str] = set()
    # Collect all uses (backwards)
    for instr in reversed(ir):
        for src in instr.srcs:
            if isinstance(src, str):
                live.add(src)
    # Keep instructions whose dst is live (or has side effects)
    SIDE_EFFECT_OPS = {"store_var", "io_out", "jmp", "jz", "jnz", "call", "ret", "deopt"}
    return [i for i in ir if i.dst in live or i.op in SIDE_EFFECT_OPS or i.op == "label"]
```

### Pass 3: Type Specialization

The most important pass. For each arithmetic op, read the feedback vector to determine
the observed operand types:

```python
def type_specialize(ir: list[IRInstr], feedback: list[SlotState]) -> list[IRInstr]:
    result = []
    slot_idx = 0
    for instr in ir:
        if instr.op in BINARY_OPS_WITH_SLOTS:
            state = feedback[slot_idx]
            slot_idx += 1
            if state.kind == SlotKind.MEGAMORPHIC:
                # Cannot specialize — emit deopt
                result.append(IRInstr(op="deopt", dst=None, srcs=[], ty="unknown",
                                      comment=f"megamorphic at slot {slot_idx-1}"))
            elif state.kind in (SlotKind.MONOMORPHIC, SlotKind.POLYMORPHIC):
                if state.observations == ["u8"]:
                    # Fully monomorphic u8: emit direct integer op, no type check
                    result.append(dataclasses.replace(instr, ty="u8"))
                else:
                    # Polymorphic: emit type guard + fast path
                    result.extend(emit_type_guard(instr, state))
            else:
                # Uninitialized — never reached. Emit deopt.
                result.append(IRInstr(op="deopt", dst=None, srcs=[], ty="unknown"))
        else:
            result.append(instr)
    return result
```

For Tetrad v1, every slot will be monomorphic u8, so this pass simply annotates
every arithmetic op with `ty="u8"` and emits the direct path. The deopt paths are
present but never triggered.

### Pass 4: Branch Layout

Reorders basic blocks so that the hot branch (high `taken_ratio`) comes first in the
generated code. This improves instruction cache locality and branch predictor accuracy.

```python
def branch_layout(ir: list[IRInstr], branch_stats: dict[int, BranchStats]) -> list[IRInstr]:
    # For each JZ/JNZ, if the NOT-taken path is hotter, invert the condition
    # and swap the branch targets.
    result = []
    for instr in ir:
        if instr.op == "jz" and instr in branch_stats:
            stats = branch_stats[instr]
            if stats.taken_ratio < 0.2:   # rarely taken → invert
                result.append(IRInstr(op="jnz", dst=instr.dst, srcs=instr.srcs, ty=instr.ty))
            else:
                result.append(instr)
        else:
            result.append(instr)
    return result
```

---

## Register Allocation

The JIT uses a **linear scan** register allocator over x86-64 registers. For Tetrad's
small functions (typically ≤ 8 virtual variables), linear scan is optimal.

Available x86-64 registers (caller-saved, safe to clobber):
- `rax`, `rcx`, `rdx`, `rsi`, `rdi`, `r8`, `r9`, `r10`, `r11`

The allocator assigns each live range (virtual variable + its uses) to a physical
register. Spills go to the stack frame.

```python
@dataclass
class LiveRange:
    var: str             # virtual variable name
    start: int           # IR instruction index where defined
    end: int             # IR instruction index of last use

def linear_scan_alloc(ir: list[IRInstr]) -> dict[str, str | int]:
    """Returns mapping: virtual var → x86 reg name or stack offset."""
    ranges = compute_live_ranges(ir)
    active: list[LiveRange] = []
    free_regs = ["rax", "rcx", "rdx", "rsi", "rdi", "r8", "r9", "r10"]
    allocation: dict[str, str | int] = {}
    stack_offset = 0

    for lr in sorted(ranges, key=lambda r: r.start):
        # Expire old intervals
        active = [a for a in active if a.end >= lr.start]

        if free_regs:
            reg = free_regs.pop(0)
            allocation[lr.var] = reg
            active.append(lr)
        else:
            # Spill: evict the interval with the furthest end
            spill = max(active, key=lambda a: a.end)
            if spill.end > lr.end:
                allocation[lr.var] = allocation.pop(spill.var)
                stack_offset -= 8
                allocation[spill.var] = stack_offset
                active.remove(spill)
                active.append(lr)
            else:
                stack_offset -= 8
                allocation[lr.var] = stack_offset

    return allocation
```

---

## x86-64 Code Generation

The code generator emits raw x86-64 machine code bytes. The Python `ctypes` module
maps those bytes to a callable function pointer.

### x86-64 Calling Convention (System V AMD64 ABI)

The JIT-compiled function follows the System V AMD64 ABI:
- Arguments: `rdi`, `rsi`, `rdx`, `rcx`, `r8`, `r9`
- Return value: `rax`
- Caller-saved: `rax`, `rcx`, `rdx`, `rsi`, `rdi`, `r8`–`r11`
- Callee-saved: `rbx`, `rbp`, `r12`–`r15`

Each Tetrad function receives its arguments as u8 values in the argument registers.
It returns its result in `rax` (u8, zero-extended to 64 bits).

### Prologue and Epilogue

```
; Prologue
push rbp
mov  rbp, rsp
sub  rsp, N      ; stack space for spilled variables

; ... body ...

; Epilogue
mov  rsp, rbp
pop  rbp
ret
```

### Instruction Encoding Examples

These are the core x86-64 encodings the code generator needs:

```python
def emit_mov_reg_imm(buf: bytearray, dst_reg: str, imm: int):
    """mov dst, imm8   (zero-extends to 64-bit)"""
    rex = 0x48 if dst_reg in 64BIT_REGS else 0x00
    # e.g. mov rax, 42 → 48 C7 C0 2A 00 00 00
    ...

def emit_mov_reg_reg(buf: bytearray, dst: str, src: str):
    """mov dst, src"""
    ...

def emit_add_reg_reg(buf: bytearray, dst: str, src: str):
    """add dst, src; then: and dst, 0xFF (ensure u8 wrap)"""
    ...

def emit_sub_reg_reg(buf: bytearray, dst: str, src: str):
    """sub dst, src; then: and dst, 0xFF"""
    ...

def emit_imul_reg_reg(buf: bytearray, dst: str, src: str):
    """imul dst, src; then: and dst, 0xFF"""
    ...

def emit_jmp_rel32(buf: bytearray, offset: int):
    """jmp rel32"""
    buf += b'\xe9' + struct.pack('<i', offset)

def emit_jz_rel32(buf: bytearray, offset: int):
    """test rax, rax; jz rel32"""
    buf += b'\x48\x85\xc0'   # test rax, rax
    buf += b'\x0f\x84' + struct.pack('<i', offset)

def emit_ret(buf: bytearray):
    buf += b'\xc3'
```

### Full Compilation Example

Compiling `fn add(a, b) { return a + b; }`:

```
IR after translation:
  v0 = load_var "a"    (from register R0 / rdi)
  v1 = load_var "b"    (from register R1 / rsi)
  v2 = add v0, v1      type=u8
  ret v2

After optimization:
  No constants to fold. No dead code. Type is monomorphic u8.

After register allocation:
  v0 → rdi (already there, argument 0)
  v1 → rsi (already there, argument 1)
  v2 → rax

Generated x86-64:
  push rbp
  mov  rbp, rsp
  ; v2 = add v0(rdi), v1(rsi)
  mov  rax, rdi
  add  rax, rsi
  and  rax, 0xFF       ; u8 wrap
  ; ret v2 (rax)
  pop  rbp
  ret
```

Machine code bytes: approximately 14 bytes for this function.

---

## Invoking JIT Code with ctypes

```python
import ctypes
import mmap

def make_executable(buf: bytes) -> ctypes.CFUNCTYPE:
    """Map bytes into executable memory and return a callable."""
    size = len(buf)
    # Allocate page-aligned executable memory
    mem = mmap.mmap(-1, size,
                    prot=mmap.PROT_READ | mmap.PROT_WRITE | mmap.PROT_EXEC)
    mem.write(buf)
    mem.seek(0)
    addr = ctypes.addressof(ctypes.c_char.from_buffer(mem))
    # Create a ctypes function pointer with correct signature
    # All Tetrad functions take and return c_uint8 (u8)
    fn_type = ctypes.CFUNCTYPE(ctypes.c_uint8, *[ctypes.c_uint8] * param_count)
    return fn_type(addr), mem   # keep mem alive
```

The `mem` object must be kept alive as long as the function pointer is in use.
The JIT code cache holds both the callable and the `mmap` object.

---

## JIT Code Cache

```python
@dataclass
class JITCacheEntry:
    fn_name: str
    compiled_at_call_count: int   # for cache invalidation
    native_fn: ctypes.CFUNCTYPE   # callable
    mmap_ref: mmap.mmap           # keep-alive reference
    compilation_time_ns: int       # for benchmarking

class JITCache:
    def __init__(self):
        self._cache: dict[str, JITCacheEntry] = {}

    def get(self, fn_name: str) -> ctypes.CFUNCTYPE | None: ...
    def put(self, entry: JITCacheEntry) -> None: ...
    def invalidate(self, fn_name: str) -> None: ...
    def stats(self) -> dict[str, dict]: ...   # for benchmarking
```

---

## On-Stack Replacement (OSR)

OSR allows the VM to switch a **currently-executing** function from interpreted to
compiled mode mid-execution, typically at a loop back-edge.

In v1, OSR is implemented in a simplified form:
1. The VM checks loop iteration counts on every `JMP_LOOP`.
2. If a loop exceeds `LOOP_ITERATION_THRESHOLD`, the JIT compiles the entire function.
3. The **current** activation finishes in the interpreter. New calls use the compiled
   version.

Full OSR (patching the live stack frame) is deferred to v2. The simplified form still
achieves the goal for functions called many times with long loops.

---

## Deoptimization

When a JIT-compiled function encounters a condition it was not compiled to handle
(e.g., a type assumption that turns out to be wrong, or a division by zero), it
**deoptimizes** back to the interpreter.

In v1, the deoptimization mechanism is:

1. The JIT inserts a `deopt` IR instruction wherever a type assumption might fail.
2. The code generator emits a call to a `deopt_handler` C function for each `deopt`.
3. The `deopt_handler` restores the interpreter state (registers, IP, call stack) and
   resumes execution in the interpreted path.

For Tetrad v1, deoptimization never fires in practice (all values are u8, no assumptions
fail). The mechanism is present so the Lisp front-end can rely on it.

---

## JIT Public API

```python
class TetradJIT:

    def __init__(self, vm: TetradVM): ...

    # Compile a function by name, using the VM's current feedback vectors.
    # Returns True if compilation succeeded.
    def compile(self, fn_name: str) -> bool: ...

    # Check if a function is already in the cache.
    def is_compiled(self, fn_name: str) -> bool: ...

    # Execute a function: use compiled version if available, else interpret.
    def execute(self, fn_name: str, args: list[int]) -> int: ...

    # Run the VM, auto-compiling functions as they get hot.
    # This is the main entry point for production use.
    def execute_with_jit(self, code: CodeObject) -> int: ...

    # Return JIT cache statistics.
    def cache_stats(self) -> dict[str, dict]: ...

    # Dump the IR for a compiled function (for debugging).
    def dump_ir(self, fn_name: str) -> str: ...

    # Dump the x86-64 disassembly for a compiled function.
    # Requires the `capstone` Python library for disassembly.
    def dump_asm(self, fn_name: str) -> str: ...
```

---

## Python Package

The JIT lives in `code/packages/python/tetrad-jit/`.

Depends on `coding-adventures-tetrad-vm`.

Optional dependency: `capstone` (for `dump_asm`). The package works without it; only
the disassembly feature is unavailable.

---

## Test Strategy

### IR generation tests

- Verify bytecode → IR translation for each opcode family
- `LDA_IMM 42; STA_REG 0; LDA_VAR x; ADD r0, slot=0` → correct IR with virtual vars

### Optimization pass tests

- Constant folding: `const 10 + const 5` → `const 15`
- Dead code: variable computed but never read → instruction removed
- Type specialization: monomorphic slot → `ty="u8"` annotation; no deopt inserted
- Branch layout: 99% taken branch stays in-line; rarely taken branch inverted

### Register allocation tests

- 2-variable function: both allocated to registers, no spills
- 9-variable function: one variable spills to stack

### Code generation tests

- `add(a, b) { return a + b; }` → x86-64 bytes, callable, produces correct result
- u8 wraparound: `add(200, 100)` → result is 44 (wraps at 256)
- Zero division via `deopt`: `div(10, 0)` invokes deopt handler without crashing

### Hot-function detection tests

- Execute a function 99 times → not compiled
- Execute 101 times → compiled (appears in cache)
- Loop with >500 iterations → triggers OSR threshold

### End-to-end JIT tests

- Execute all five TET00 example programs under `execute_with_jit`
- Verify output matches interpreter output
- Verify JIT-compiled functions appear in cache after enough iterations

### Performance smoke test

- `multiply(255, 200)` under JIT vs. interpreter: verify JIT is faster
  (quantitative threshold: JIT ≥ 5× faster for tight loops)

### Coverage target

90%+ line coverage (lower than 95% due to platform-specific mmap/ctypes paths).

---

## Version History

| Version | Date | Description |
|---|---|---|
| 0.1.0 | 2026-04-20 | Initial specification |
