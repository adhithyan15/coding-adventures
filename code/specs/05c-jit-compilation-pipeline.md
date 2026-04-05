# 05c — JIT Compilation Pipeline: Profiler, Architecture Catalog, and Tiered Compiler

## Overview

This document specifies the complete "acceleration pipeline" — the path from
interpreted bytecode to optimized native machine code. It covers three
interconnected subsystems:

1. **A generic bytecode/IR profiler** that hooks into the VM to collect runtime
   feedback (execution counts, types, branches, call sites)
2. **A comprehensive architecture catalog** listing every CPU target we could
   generate native code for, from the Intel 4004 (1971) to AArch64 (2025)
3. **A tiered JIT compiler** inspired by V8 and JavaScriptCore that
   progressively compiles hot code through increasingly aggressive optimization

The critical design principle: **this entire pipeline is language-agnostic.**
Only the grammar files (`.tokens` + `.grammar`) and the AST-to-bytecode
lowering rules change per language. The profiler, JIT tiers, and native
backends are shared infrastructure. JavaScript, Python, Ruby, Lua — or any
future language that compiles to our generic bytecode — all share the same
acceleration path.

```
Source (any language)
  |
  v
Lexer + Parser (grammar-driven)
  |
  v
Bytecode Compiler (AST -> generic bytecode)
  |
  v
+================================================================+
| VM (Tier 0: Interpreter)                                       |
|   |                                                            |
|   +-- Profiler hooks --> ProfileData                           |
|                            |                                   |
|                 +----------+----------+----------+             |
|                 |          |          |           |             |
|              Tier 1     Tier 2     Tier 3                      |
|             Baseline   Optimizing  Full Opt                    |
|                 |          |          |                         |
|                 +-----+----+----+----+                         |
|                       |              |                          |
|                   Native Code Backends                         |
|              RISC-V  ARM  x86-64  WASM  4004  6502  ...       |
+================================================================+
```

### Relationship to Existing Specs

- **05-virtual-machine.md** — The GenericVM that this pipeline extends
- **05b-jit-compiler.md** — The conceptual shell; this document supersedes and
  expands it into an implementation-ready design
- **04a-bytecode-compiler-backends.md** — Establishes the "one frontend,
  multiple backends" pattern that the JIT extends to runtime
- **D02-branch-predictor.md** — The micro-architecture work that the JIT's
  instruction scheduling can leverage

---

## Part 1: Generic Bytecode/IR Profiler

### 1.1 Why Profile?

The VM executes all bytecode uniformly — it spends equal effort dispatching
a `LOAD_CONST` in dead code and a `ADD` inside a hot inner loop. A profiler
identifies what's hot so the JIT knows where to spend compilation effort.

Think of a factory. You don't automate every task — you automate the ones
repeated thousands of times per day. The profiler is the person with a
clipboard counting how often each station is used.

### 1.2 What the Profiler Collects

| Data Category | What It Records | Why the JIT Needs It |
|---------------|----------------|---------------------|
| **Execution counts** | Times each bytecode offset executes | Identify hot functions/loops for tier promotion |
| **Type feedback** | Types seen at each arithmetic/comparison op | Enable type specialization (int+int -> native integer ADD) |
| **Call site info** | Caller/callee pairs with frequency | Guide inlining decisions (inline frequent callees) |
| **Branch direction** | Taken/not-taken history per branch | Code layout optimization (likely path falls through) |
| **Loop structure** | Loop headers, back-edges, trip counts | Identify OSR entry points, guide loop unrolling |

### 1.3 ProfilerHook Protocol

The profiler follows the GenericVM's `OpcodeHandler` plugin pattern. The VM
calls profiler hooks at defined points in the eval loop. When no profiler is
attached, the hooks are null checks — zero overhead.

```python
class ProfilerHook(Protocol):
    """Hooks called by the VM during bytecode execution."""

    def on_instruction(self, pc: int, opcode: int, operand: int | None) -> None:
        """Called before every instruction dispatch."""
        ...

    def on_type_feedback(self, pc: int, operand_types: tuple[type, ...]) -> None:
        """Called before arithmetic/comparison ops with the operand types."""
        ...

    def on_call(self, caller_pc: int, callee_name: str) -> None:
        """Called when a function is invoked."""
        ...

    def on_return(self, callee_name: str) -> None:
        """Called when a function returns."""
        ...

    def on_branch(self, pc: int, taken: bool) -> None:
        """Called at every conditional branch with the actual direction."""
        ...
```

The VM attaches a profiler via `vm.set_profiler(profiler)` — a single method
call, matching the simplicity of `vm.register_opcode()`.

### 1.4 Data Structures

```python
@dataclass
class TypeFeedbackRecord:
    """Type history for a single bytecode offset.

    Classification:
    - monomorphic:   only one type combination seen (best for JIT — specialize directly)
    - polymorphic:   2-4 type combinations seen (JIT can use inline cache chain)
    - megamorphic:   5+ type combinations seen (JIT falls back to generic dispatch)
    """
    offset: int
    type_history: dict[tuple[type, ...], int]  # type combo -> count

    @property
    def classification(self) -> str:
        n = len(self.type_history)
        if n == 1: return "monomorphic"
        if n <= 4: return "polymorphic"
        return "megamorphic"


@dataclass
class CallSiteRecord:
    caller_offset: int
    callee_name: str
    count: int


@dataclass
class BranchRecord:
    offset: int
    taken_count: int
    not_taken_count: int

    @property
    def taken_ratio(self) -> float:
        total = self.taken_count + self.not_taken_count
        return self.taken_count / total if total > 0 else 0.5


@dataclass
class LoopInfo:
    header_offset: int       # first instruction of the loop
    back_edge_offset: int    # the branch that jumps back to the header
    estimated_trip_count: int


@dataclass
class ProfileData:
    """Complete profiling data for a code object."""
    execution_counts: dict[int, int]          # offset -> count
    type_feedback: dict[int, TypeFeedbackRecord]
    call_sites: list[CallSiteRecord]
    branches: dict[int, BranchRecord]
    loops: list[LoopInfo]
```

### 1.5 Control Flow Graph Construction

The profiler builds a CFG from flat bytecode to identify loops:

1. **Scan for basic block boundaries**: Any branch target or instruction
   following a branch starts a new basic block
2. **Build directed graph**: Each block points to its successors (fall-through
   and branch targets)
3. **Identify back-edges**: An edge where the target dominates the source
   indicates a loop
4. **Compute loop nesting**: Build the dominator tree to find nested loops

```
Example: while (i < n) { total = total + i; i = i + 1; }

Bytecode:
  0: LOAD_NAME i          ─┐
  1: LOAD_NAME n           │  Block 0 (loop header)
  2: COMPARE LT            │
  3: JUMP_IF_FALSE 10     ─┘─── Block 0 exits
  4: LOAD_NAME total      ─┐
  5: LOAD_NAME i           │
  6: ADD                   │  Block 1 (loop body)
  7: STORE_NAME total      │
  8: LOAD_NAME i           │
  9: JUMP 0               ─┘─── back-edge to Block 0
 10: HALT                 ─── Block 2 (loop exit)

CFG:
  Block 0 (header) ──→ Block 1 (body) ──→ Block 0 (back-edge)
       │
       └──→ Block 2 (exit)
```

### 1.6 Hot Path Detection

Threshold-based promotion policy:

```
                  100 execs          1,000 execs        10,000 execs
  Tier 0 ──────────────> Tier 1 ──────────────> Tier 2 ──────────────> Tier 3
(interpreter)           (baseline)              (optimizing)           (full opt)
      <──────────────          <──────────────         <──────────────
       deoptimize               deoptimize              deoptimize
```

Thresholds are configurable per-deployment. A REPL might use lower thresholds
(code runs briefly). A server might use higher thresholds (amortize compilation
over long-running processes).

### 1.7 Concrete Example

Given this program:

```python
total = 0
for i in range(1000):
    total = total + i
```

After 100 iterations, the profiler reports:

```
Execution counts:
  offset 0 (LOAD_CONST 0):     1    # total = 0 (once)
  offset 4 (LOAD_NAME total): 100    # loop body
  offset 5 (LOAD_NAME i):     100
  offset 6 (ADD):             100
  offset 7 (STORE_NAME total): 100

Type feedback at offset 6 (ADD):
  (int, int) -> 100 times    # MONOMORPHIC — perfect for specialization

Branch at offset 3 (JUMP_IF_FALSE):
  taken: 0, not_taken: 100   # loop body always entered (so far)

Loop detected:
  header=0, back_edge=9, trip_count=100 (and growing)
```

The JIT sees: monomorphic int+int at offset 6, a hot loop with 100+
iterations. This is a prime candidate for Tier 1 promotion.

### 1.8 Public API Summary

```python
# Attach profiler to VM
vm.set_profiler(BasicProfiler())

# Execute code (profiler collects data automatically)
vm.execute(code_object)

# Read profiler data
profile = vm.profiler.get_profile()
print(profile.execution_counts)
print(profile.type_feedback[6].classification)  # "monomorphic"
print(profile.loops[0].estimated_trip_count)     # 1000
```

---

## Part 2: Architecture Catalog

Like Godbolt/Compiler Explorer, this catalog lists every CPU architecture
our JIT could potentially target. Each architecture represents a possible
native code backend — a `CodeEmitter` implementation that translates our
SSA IR to machine instructions for that target.

### 2.1 Vintage Era (4-8 bit, 1971-1980)

These are the dawn of microprocessors. Generating JIT code for them is an
exercise in extreme constraint — imagine running a subset of JavaScript on
hardware designed for calculators.

| Architecture | Year | Bits | Type | Registers | ISA Public | We Simulate | Notes |
|---|---|---|---|---|---|---|---|
| **Intel 4004** | 1971 | 4 | Accumulator | 16x4-bit | Yes | Yes (ISA + gate) | First commercial CPU. 46 instructions. 640 bytes RAM. |
| **Intel 8008** | 1972 | 8 | Accumulator | 7x8-bit | Yes | No | First 8-bit CPU. Predecessor to 8080. |
| **Intel 8080** | 1974 | 8 | Accumulator | 7x8-bit | Yes | No | Powered CP/M. Z80 is backward-compatible. |
| **MOS 6502** | 1975 | 8 | Accumulator | 3 (A,X,Y) | Yes | No | Apple II, Commodore 64, NES, Atari 2600. $25 vs $300 for 8080. |
| **Zilog Z80** | 1976 | 8 | Register | 14x8-bit | Yes | No | Game Boy, ZX Spectrum, MSX. 8080-compatible. |

The 6502 is the simplest viable JIT target after the 4004 — only 3 registers
and 56 instructions. It would be a fascinating constraint challenge.

### 2.2 Classic Era (16-32 bit, 1978-1990)

The era that split into CISC vs RISC philosophies. Intel and Motorola went
complex (more instructions, fewer cycles per task); ARM and MIPS went simple
(fewer instructions, one cycle each).

| Architecture | Year | Bits | Type | Registers | ISA Public | We Simulate | Notes |
|---|---|---|---|---|---|---|---|
| **Intel 8086** | 1978 | 16 | CISC | 8x16-bit | Yes | No | Started the x86 dynasty. IBM PC. |
| **Motorola 68000** | 1979 | 16/32 | CISC | 16x32-bit | Yes | No | Original Mac, Amiga, Sega Genesis, Atari ST. |
| **VAX** | 1977 | 32 | CISC | 16x32-bit | Yes | No | DEC minicomputers. "CISC taken to its logical extreme." |
| **Intel i386** | 1985 | 32 | CISC | 8x32-bit | Yes | No | First 32-bit x86. Protected mode. |
| **ARM1** | 1985 | 32 | RISC | 16x32-bit | Yes | Yes (ISA + gate) | Only 25,000 transistors. Clean RISC design. |
| **MIPS I** | 1985 | 32 | RISC | 32x32-bit | Yes | No | SGI workstations. Classic textbook RISC. |
| **SPARC** | 1986 | 32 | RISC | 32x32-bit (windowed) | Yes | No | Sun Microsystems. Register windows. |

The Motorola 68000 is particularly interesting — it was the "developer's
favorite" of the 1980s, with a clean ISA despite being CISC. The Sega Genesis
used it, so JIT-compiling game logic to 68000 would be historically fun.

### 2.3 Modern Era (32-64 bit, 1990-present)

These are what code actually runs on today. They are the priority targets
for a production JIT.

| Architecture | Year | Bits | Type | Registers | ISA Public | We Simulate | Notes |
|---|---|---|---|---|---|---|---|
| **PowerPC** | 1992 | 32/64 | RISC | 32x64-bit | Yes | No | Apple PowerMac (1994-2006), PlayStation 3, Wii. |
| **Alpha** | 1992 | 64 | RISC | 32x64-bit | Yes | No | DEC. Fastest CPU of its era. Influenced x86-64 design. |
| **ARMv7** | ~2003 | 32 | RISC | 16x32-bit | Yes | Yes (ISA) | Pre-2016 smartphones. Thumb-2 for code density. |
| **x86-64 / AMD64** | 2003 | 64 | CISC | 16x64-bit | Yes | No | All modern desktops/servers. Variable-length encoding. |
| **AArch64 / ARMv8** | 2011 | 64 | RISC | 31x64-bit | Yes | No | Modern phones, tablets, Apple Silicon, AWS Graviton. |
| **RISC-V RV32I** | 2014 | 32 | RISC | 32x32-bit | Yes (open) | Yes (ISA) | Open-source ISA. Clean, modular design. |
| **RISC-V RV64I** | 2014 | 64 | RISC | 32x64-bit | Yes (open) | No | 64-bit extension of RV32I. |

### 2.4 Virtual Machine Targets

These aren't physical CPUs — they're bytecode formats for software VMs. But
they're valid JIT targets. In fact, this is how real JITs work: V8 can JIT
JavaScript to WebAssembly; HotSpot JITs Java bytecode to native code.

| Target | Year | Type | Stack/Register | We Simulate | Notes |
|---|---|---|---|---|---|
| **WebAssembly** | 2015 | Stack VM | Stack-based | Yes | Portable, sandboxed. Running in every browser. |
| **JVM Bytecode** | 1995 | Stack VM | Stack-based | Yes | Java, Kotlin, Scala, Clojure. Mature ecosystem. |
| **CLR IL** | 2000 | Stack VM | Stack-based | Yes | C#, F#, VB.NET. Similar to JVM but with value types. |

### 2.5 Microcontrollers and Embedded

Targets where JIT compilation itself is too expensive (too little RAM), but
ahead-of-time compilation is valuable.

| Architecture | Year | Bits | Type | Notes |
|---|---|---|---|---|
| **AVR** | 1996 | 8 | RISC | Arduino. 32 registers, Harvard architecture. |
| **MSP430** | 1993 | 16 | RISC | TI ultra-low-power. 16 registers. |
| **Xtensa** | ~2000 | 32 | RISC (configurable) | ESP32/ESP8266. Customizable ISA. |
| **Cortex-M0** | 2009 | 32 | RISC (Thumb) | Simplest ARM core. IoT, wearables. |

### 2.6 Exotic and Research

| Architecture | Year | Bits | Type | Notes |
|---|---|---|---|---|
| **eBPF** | 2014 | 64 | Register | JIT target inside the Linux kernel. Sandboxed. |
| **LoongArch** | 2021 | 64 | RISC | China's new ISA. MIPS-influenced. |
| **SuperH (SH-2/SH-4)** | 1992 | 32 | RISC | Sega Saturn, Dreamcast. |
| **PA-RISC** | 1986 | 32/64 | RISC | HP workstations. Unique delayed branching. |
| **Itanium (IA-64)** | 2001 | 64 | VLIW/EPIC | Intel/HP. Famous failure. VLIW architecture. |

Itanium is particularly interesting as a cautionary tale — it tried to move
optimization from runtime (JIT) to compile-time (VLIW scheduling), and the
compiler technology never caught up to the hardware's ambitions.

### 2.7 Priority Ranking for JIT Backends

| Priority | Architecture | Rationale |
|---|---|---|
| **1** | RISC-V RV32I | Already simulated, open ISA, clean encoding, educational clarity |
| **2** | ARMv7 | Already simulated, real-world mobile relevance |
| **3** | WebAssembly | Already simulated, portable, growing ecosystem |
| **4** | MOS 6502 | Simple enough for a first vintage backend. Historically fun. |
| **5** | Intel 4004 | Already simulated at gate level. The "var x = 1+2 to transistors" demo. |
| **6** | x86-64 | Not simulated but most common desktop/server architecture |
| **7** | AArch64 | Not simulated but dominant mobile/laptop (Apple Silicon) |
| **8** | JVM | Already simulated. Demonstrates cross-VM compilation. |
| **9** | Motorola 68000 | Retro computing fun. Clean CISC ISA. |
| **10** | Z80 | Game Boy target. Constraint programming challenge. |

### 2.8 CodeEmitter Protocol

Every backend implements this interface:

```python
class CodeEmitter(Protocol):
    """Emits native machine code for a specific architecture."""

    def emit_load_immediate(self, dest: Register, value: int) -> None: ...
    def emit_load_memory(self, dest: Register, base: Register, offset: int) -> None: ...
    def emit_store_memory(self, src: Register, base: Register, offset: int) -> None: ...
    def emit_add(self, dest: Register, src1: Register, src2: Register) -> None: ...
    def emit_sub(self, dest: Register, src1: Register, src2: Register) -> None: ...
    def emit_mul(self, dest: Register, src1: Register, src2: Register) -> None: ...
    def emit_compare(self, src1: Register, src2: Register) -> None: ...
    def emit_branch(self, target: Label) -> None: ...
    def emit_branch_if(self, condition: Condition, target: Label) -> None: ...
    def emit_call(self, target: Label) -> None: ...
    def emit_return(self) -> None: ...
    def emit_label(self, label: Label) -> None: ...
    def finalize(self) -> bytes: ...

    @property
    def register_count(self) -> int: ...
    @property
    def word_size(self) -> int: ...
    @property
    def architecture_name(self) -> str: ...
```

This reuses the existing assembler package's encoding functions. The ARM
`CodeEmitter` calls `encode_data_processing()`; the RISC-V `CodeEmitter` calls
the RISC-V assembler's encoding functions.

---

## Part 3: Tiered JIT Compiler Architecture

### 3.1 Why Multiple Tiers?

An interpreter can start executing code instantly but runs slowly. An
optimizing compiler produces fast code but takes time to compile. Multiple
tiers let us start fast and get faster:

| Tier | Compilation Cost | Execution Speed | Real-World Analogue |
|---|---|---|---|
| **0: Interpreter** | 0 (instant) | 1x (baseline) | CPython, Ruby MRI |
| **1: Baseline** | ~10us/function | ~5-10x faster | V8 Sparkplug |
| **2: Optimizing** | ~1ms/function | ~20-50x faster | V8 Maglev, JSC DFG |
| **3: Full Optimizing** | ~10-100ms/function | ~50-100x faster | V8 TurboFan, JSC FTL |

The insight from V8's history: the gap between Tier 0 and Tier 3 was too
large. Users experienced perceptible jank while TurboFan compiled. Adding
Sparkplug (Tier 1) and Maglev (Tier 2) smoothed the transition. Our design
follows this lesson.

### 3.2 Tier 0: Interpreter (Existing)

This is our GenericVM. It already works. The additions needed:

1. **Profiler hook points** in the eval loop (see Part 1)
2. **Compiled code check** at loop back-edges and function entries:

```python
def execute_instruction(self, pc, instruction):
    # Check if compiled code exists for this PC
    if self.jit and self.jit.has_compiled_code(pc):
        return self.jit.execute_compiled(pc, self.frame)

    # Normal interpretation
    match instruction.opcode:
        case OpCode.LOAD_CONST: ...
        case OpCode.ADD: ...
```

### 3.3 Tier 1: Baseline Compiler

Translates bytecode 1:1 to native code **without optimization**. The goal
is to eliminate interpreter dispatch overhead (the `match` statement) while
compiling almost instantly.

No register allocation — uses a fixed stack-based mapping. Each bytecode
instruction becomes a fixed sequence of native instructions:

```
Bytecode:               RISC-V output (Tier 1):

LOAD_CONST 0  (42)      li   t0, 42          # load immediate
                         addi sp, sp, -4      # push to stack
                         sw   t0, 0(sp)

LOAD_CONST 1  (7)       li   t0, 7
                         addi sp, sp, -4
                         sw   t0, 0(sp)

ADD                      lw   t0, 0(sp)       # pop first
                         lw   t1, 4(sp)       # pop second
                         add  t0, t1, t0      # add
                         addi sp, sp, 4       # adjust stack
                         sw   t0, 0(sp)       # push result
```

This is intentionally naive. Every value goes through the stack in memory.
But it's ~5-10x faster than interpretation because there's no opcode dispatch
loop, no instruction decoding, no VM state bookkeeping.

### 3.4 Tier 2: Optimizing Compiler

Uses profiler data to generate significantly better code. Key optimizations:

**Type specialization**: The profiler says ADD at offset 6 always sees
`(int, int)`. Instead of calling a generic ADD that checks types at runtime,
emit a direct native integer add with a type guard:

```
# Before (generic):               # After (specialized):
call    generic_add                # Guard: check types are int
                                   lw   t0, type_offset(a0)
                                   li   t1, TYPE_INT
                                   bne  t0, t1, deoptimize
                                   # Fast path: native integer add
                                   add  a0, a0, a1
```

**Constant folding**: `1 + 2` becomes `3` at compile time.

**Dead code elimination**: If the profiler says a branch is never taken, don't
compile that path (but insert a deoptimization guard in case it ever is taken).

**Inline caching**: For property lookups, cache the object shape and offset so
subsequent accesses are a single memory load instead of a hash table lookup.

#### SSA Intermediate Representation

Tier 2 converts stack bytecode to SSA (Static Single Assignment) form for
optimization. SSA is the industry standard used by LLVM, V8's Maglev, and
HotSpot's C2:

```
Bytecode:                    SSA IR:

LOAD_CONST 0  (val=42)      v0 = Constant(42)
LOAD_CONST 1  (val=7)       v1 = Constant(7)
ADD                          v2 = Add(v0, v1)       # type: int
STORE_NAME 0  (name="x")    Store("x", v2)
```

The key property of SSA: each value is assigned exactly once. This makes
dataflow analysis trivial — to find where a value comes from, just look at
its definition. Phi nodes handle control flow merges:

```
# if (cond) { x = 1; } else { x = 2; } use(x);

                           v0 = Constant(1)
                           v1 = Constant(2)
    merge point:           v2 = Phi(v0, v1)    # x is v0 or v1
                           Call("use", v2)
```

### 3.5 Tier 3: Full Optimizing Compiler

The most aggressive tier. Additional optimizations beyond Tier 2:

**Inlining**: Replace CALL with the callee's body. The profiler's call site
data guides which functions to inline (high-frequency callees first):

```
# Before inlining:            # After inlining:
v0 = Call("square", v1)       v0 = Mul(v1, v1)    # square(x) = x * x
```

**Escape analysis**: If an object never escapes the current function (never
stored to a global, never passed to a non-inlined call), allocate it on the
stack instead of the heap. This eliminates GC pressure for temporary objects.

**Loop-invariant code motion**: Move computations that don't change across
iterations out of the loop:

```
# Before:                      # After:
for i in range(n):             t = len(array)
    if i < len(array): ...     for i in range(n):
                                   if i < t: ...
```

**Full register allocation**: Graph coloring or linear scan to map SSA values
to physical registers, minimizing memory spills.

**Instruction scheduling**: Reorder instructions to avoid pipeline stalls,
using our existing pipeline model (D04-pipeline.md) to predict latencies.

### 3.6 Deoptimization and On-Stack Replacement

These are the hardest parts of JIT compilation.

#### Deoptimization

When a type guard fails (e.g., ADD assumed int+int but received int+string),
the JIT must bail out:

```
1. Stop executing native code
2. Read the deoptimization map to find the corresponding interpreter state
3. Reconstruct the VM frame: stack, local variables, program counter
4. Resume execution in the interpreter at the correct bytecode offset
```

The **deoptimization map** is built during JIT compilation — a table mapping
native code addresses to interpreter state:

```
Native PC   Bytecode PC   Stack Depth   Live Registers -> Stack Slots
0x1000      offset 4      2             t0 -> slot[0], t1 -> slot[1]
0x1008      offset 5      3             t0 -> slot[0], t1 -> slot[1], t2 -> slot[2]
0x1010      offset 6      3             (same as above, pre-ADD)
```

#### On-Stack Replacement (OSR)

Enter compiled code mid-execution. When a long-running loop becomes hot,
we don't want to wait for the function to return. Instead:

```
1. At a loop back-edge, the interpreter checks: "is compiled code available?"
2. If yes, build the compiled code's expected state from interpreter state
3. Jump into the compiled code at the loop header

   Interpreter                          Compiled Code
   ┌──────────┐                        ┌──────────────┐
   │ frame:   │   ── transfer state →  │ registers:   │
   │  i = 500 │                        │  t0 = 500    │
   │  n = 1000│                        │  t1 = 1000   │
   │  total=X │                        │  t2 = X      │
   └──────────┘                        └──────────────┘
       ↓                                     ↓
   (stop interpreting)              (continue in native code)
```

### 3.7 IR Design: Why SSA?

Three IR styles exist in production JITs:

| IR Style | Used By | Pros | Cons | Our Choice |
|---|---|---|---|---|
| **Linear IR** | LuaJIT | Simple, cache-friendly | Hard to optimize | No |
| **SSA** | LLVM, Maglev, HotSpot C2 | Standard, many algorithms, well-understood | Phi nodes add complexity | **Yes** |
| **Sea-of-Nodes** | V8 TurboFan, Graal | Most flexible, scheduling freedom | Very complex, hard to debug | No (too complex for education) |

SSA is the sweet spot: powerful enough for all standard optimizations
(constant propagation, dead code elimination, register allocation), simple
enough to implement and explain. LLVM uses it, so there's extensive literature.

### 3.8 Multi-Language Applicability

Everything in this spec is language-agnostic. Here's how it applies to
different source languages:

#### JavaScript (via ECMAScript grammars)

- **Grammar**: `es2025.tokens` + `es2025.grammar`
- **Type challenges**: Everything is a Number (IEEE 754 double) until the
  profiler proves otherwise. Objects have hidden shapes.
- **Key optimizations**: Hidden classes for object shapes, inline caches for
  property access, speculative integer arithmetic (most JS "numbers" are
  actually small integers)

#### Python

- **Grammar**: future `python.tokens` + `python.grammar`
- **Type challenges**: Dynamically typed, but profiler reveals most code is
  monomorphic in practice
- **Key optimizations**: Specialize `__add__`, `__getattr__` based on observed
  types. Guard on object class, then direct slot access.

#### Ruby

- **Grammar**: future `ruby.tokens` + `ruby.grammar`
- **Type challenges**: Similar to Python, plus blocks/procs require closure
  optimization
- **Key optimizations**: Inline method caches, block inlining, method
  lookup caching

#### Lua

- **Grammar**: existing `lua.tokens` + `lua.grammar` (if created)
- **Type challenges**: Tables are the only data structure (like JS objects)
- **Key optimizations**: Table shape specialization, number-vs-integer
  specialization
- **Alternative**: LuaJIT uses trace-based JIT rather than method-based.
  Our Tier 2/3 could offer a trace-based mode as an alternative.

#### The Key Insight

The profiler doesn't know what language it's profiling. It sees bytecode
offsets and types. The JIT doesn't know what language it's compiling. It sees
SSA IR and type guards. Only the grammar files and the bytecode compiler know
the source language. **Swap the grammar, get a different language. The
acceleration pipeline stays the same.**

### 3.9 End-to-End Walkthrough

Follow `sum_to(1000)` through all four tiers:

```python
def sum_to(n):
    total = 0
    i = 0
    while i < n:
        total = total + i
        i = i + 1
    return total
```

**Bytecode** (from bytecode compiler):

```
 0: LOAD_CONST 0     # 0 (for total)
 1: STORE_NAME 0     # total
 2: LOAD_CONST 0     # 0 (for i)
 3: STORE_NAME 1     # i
 4: LOAD_NAME 1      # i           ← loop header
 5: LOAD_NAME 2      # n
 6: COMPARE LT
 7: JUMP_IF_FALSE 14
 8: LOAD_NAME 0      # total
 9: LOAD_NAME 1      # i
10: ADD
11: STORE_NAME 0     # total
12: LOAD_NAME 1      # i
13: LOAD_CONST 1     # 1
14: ADD
15: STORE_NAME 1     # i
16: JUMP 4                         ← back-edge
17: LOAD_NAME 0      # total
18: RETURN
```

**Profiler data** (after 100 calls to sum_to):

```
offset 10 (ADD): 100,000 executions, type=(int,int) MONOMORPHIC
offset 14 (ADD): 100,000 executions, type=(int,int) MONOMORPHIC
Loop: header=4, back_edge=16, avg_trip_count=1000
Branch at 7: taken=100, not_taken=100,000
```

**Tier 1** (RISC-V, naive 1:1 translation — every value through stack):

```asm
# ~50 instructions, ~5x faster than interpreter
# (no dispatch overhead, but every value goes through memory)
sum_to_tier1:
    li   t0, 0
    sw   t0, 0(s0)        # total = 0
    sw   t0, 4(s0)        # i = 0
.loop:
    lw   t0, 4(s0)        # load i
    lw   t1, 8(s0)        # load n
    bge  t0, t1, .exit    # if i >= n, exit
    lw   t2, 0(s0)        # load total
    add  t2, t2, t0       # total + i
    sw   t2, 0(s0)        # store total
    addi t0, t0, 1        # i + 1
    sw   t0, 4(s0)        # store i
    j    .loop
.exit:
    lw   a0, 0(s0)        # return total
    ret
```

**Tier 2** (RISC-V, type-specialized, values in registers):

```asm
# ~15 instructions, ~30x faster than interpreter
# Type guards at entry, then pure register operations
sum_to_tier2:
    # Type guard: verify n is int (deoptimize if not)
    lw   t0, type_offset(a0)
    li   t1, TYPE_INT
    bne  t0, t1, .deopt
    # Fast path: everything in registers
    li   t2, 0             # total = 0
    li   t3, 0             # i = 0
    lw   t4, value_offset(a0) # n (unboxed int)
.loop:
    bge  t3, t4, .exit
    add  t2, t2, t3        # total += i
    addi t3, t3, 1         # i++
    j    .loop
.exit:
    mv   a0, t2            # return total
    ret
.deopt:
    j    deoptimize_stub   # fall back to interpreter
```

**Tier 3** (RISC-V, loop strength-reduced, minimal instructions):

```asm
# ~8 instructions, ~80x faster than interpreter
# Compiler recognizes sum_to(n) = n*(n-1)/2 via strength reduction
# (or at minimum, fully unrolled + SIMD-friendly)
sum_to_tier3:
    lw   t0, value_offset(a0) # n
    addi t1, t0, -1        # n - 1
    mul  t2, t0, t1        # n * (n-1)
    srli a0, t2, 1         # / 2
    ret
```

---

## Part 4: Package Map

| Package | Purpose | Dependencies |
|---|---|---|
| `profiler` | Generic bytecode profiler (Part 1) | `virtual-machine` |
| `jit-compiler` | Tier dispatch, promotion, deopt framework | `profiler`, `virtual-machine` |
| `jit-baseline` | Tier 1 baseline compiler | `jit-compiler`, `assembler` |
| `jit-optimizer` | Tier 2 SSA IR + optimizations | `jit-compiler`, `profiler` |
| `jit-full-optimizer` | Tier 3 aggressive optimizations | `jit-optimizer` |
| `codegen-riscv` | RISC-V code emitter | `riscv-simulator` (for testing) |
| `codegen-arm` | ARM code emitter | `arm-simulator` (for testing) |
| `codegen-wasm` | WASM code emitter | `wasm-simulator` (for testing) |
| `codegen-6502` | MOS 6502 code emitter | (new simulator needed) |
| `codegen-4004` | Intel 4004 code emitter | `intel4004-simulator` (for testing) |
| `codegen-x86-64` | x86-64 code emitter | (new simulator needed) |

---

## Part 5: Implementation Roadmap

| Phase | Milestone | Key Deliverable |
|---|---|---|
| **1** | Profiler | `profiler` package with execution counts, type feedback, CFG |
| **2** | CFG + Loops | Loop detection, back-edge identification, trip count estimation |
| **3** | Tier 1 Baseline | RISC-V code emitter + 1:1 bytecode translation |
| **4** | Deoptimization | Deopt maps, state reconstruction, interpreter fallback |
| **5** | Tier 2 SSA IR | SSA construction from bytecode, basic optimizations |
| **6** | Tier 2 RegAlloc | Linear scan register allocation |
| **7** | OSR | On-stack replacement for long-running loops |
| **8** | Tier 3 Inlining | Function inlining guided by call site profiling |
| **9** | Tier 3 Escape | Escape analysis and stack allocation |
| **10** | More Backends | ARM, WASM, x86-64, 6502, 4004 code emitters |
| **11** | JS Subset | End-to-end: ES1 JavaScript -> bytecode -> JIT -> RISC-V |
| **12** | Python Subset | Same pipeline, different grammar, same JIT |
| **13** | Ruby Subset | Same pipeline, different grammar, same JIT |

---

## Part 6: Test Strategy

| Component | Test Type | What It Verifies |
|---|---|---|
| Profiler | Unit | Execution counter increments, type feedback correctness |
| Profiler | Unit | CFG construction from known bytecode patterns |
| Profiler | Integration | Profile a Starlark program end-to-end |
| Profiler | Performance | Overhead < 10% of interpretation time |
| Tier 1 | Correctness | Compiled code produces same output as interpreter |
| Tier 1 | Unit | Each bytecode instruction translates to correct native code |
| Tier 2 | Correctness | Optimized code produces same output as unoptimized |
| Tier 2 | Unit | SSA construction is correct (phi placement, use-def chains) |
| Tier 3 | Correctness | Aggressively optimized code still correct |
| Tier 3 | Unit | Inlining preserves semantics, escape analysis is sound |
| Deopt | Correctness | Type guard failure falls back to interpreter correctly |
| OSR | Correctness | Mid-loop entry produces correct results |
| End-to-end | Integration | Same program through interpreter and all JIT tiers gives identical output |

---

## Part 7: Future Extensions

- **Trace-based JIT**: Alternative to method-based, inspired by LuaJIT.
  Record a single execution trace through a loop, compile just that trace.
- **Concurrent compilation**: Compile Tier 2/3 in a background thread while
  the interpreter continues executing.
- **Code cache persistence**: Save compiled native code to disk (like Python's
  `.pyc` but for machine code). Avoid recompilation on restart.
- **Profile-guided deoptimization**: Detect oscillating types (int -> string
  -> int) and stop promoting functions that keep deoptimizing.
- **SIMD/Vector support**: Add vector instructions to the SSA IR and code
  emitters for data-parallel workloads.
- **Garbage collection integration**: Safe points in compiled code where the
  GC can scan the stack and relocate objects.
