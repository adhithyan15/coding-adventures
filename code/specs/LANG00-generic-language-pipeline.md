# LANG00 — Generic Language Pipeline Architecture

## Overview

Tetrad showed that a complete language — lexer, parser, type checker, bytecode
compiler, register VM, and Intel 4004 JIT — can be built end-to-end in this
repository.  The pipeline works beautifully.  The problem is that every piece
is coupled to Tetrad: `TetradVM`, `TetradJIT`, `CodeObject`, `Op`.

This spec introduces a **two-IR architecture** that makes the interpreted
half of the stack fully generic.  Any new language (BASIC, Lua, Python subset,
Prolog, …) only needs to supply:

1. A lexer + parser that produces an AST
2. A bytecode compiler that emits `InterpreterIR`
3. A type-checker plug-in (optional, enables earlier JIT compilation)
4. A backend selection (Intel 4004, WASM, JVM, RISC-V, …)

Everything else — the VM, the JIT, the optimization passes, the register
allocator, the code-cache, the debugger, the language server, the REPL,
and the notebook kernel — is reused without modification.

---

## The Two-IR Model

Prior to this generification, the pipeline had one IR per language:

```
Tetrad source → TetradBytecode (CodeObject / Op) → TetradVM → TetradJIT
```

The `tetrad-jit` package invented its own `IRInstr` SSA IR specifically
for the 4004 backend.  That IR was good enough for Tetrad alone; it cannot
be reused because it is defined inside `tetrad_jit.ir`.

The generic pipeline separates concerns into two distinct IRs:

```
                ┌─ Interpreted path ─────────────────────────────────┐
Source code     │                                                    │
  → frontend    │   InterpreterIR          CompilerIR                │
  → AST         │   (dynamic, typed        (static, SSA,             │
  → bytecode ───┤    feedback slots,        typed, backend-          │
    compiler    │    deopt anchors)         portable)                │
                │         │                     ▲                    │
                │         │    JIT-core         │                    │
                │         └─── specializes ─────┘                    │
                │              (with feedback)                        │
                │                                                     │
                │   vm-core executes InterpreterIR                   │
                │   jit-core compiles hot frames → CompilerIR        │
                │   backend-protocol emits native binary             │
                └─────────────────────────────────────────────────────┘

                ┌─ Compiled path (unchanged) ─────────────────────────┐
Source code     │                                                    │
  → frontend    │   CompilerIR → ir-optimizer → backend              │
  → AST         │   (already generic; used by Nib, WASM, JVM, etc.) │
  → compiler ───┘                                                     │
                └─────────────────────────────────────────────────────┘
```

### InterpreterIR  (new — LANG01)

A **dynamic** IR designed for interpreted execution and JIT feedback:

- Operations expressed as simple named instructions with source/dest operands
- Each instruction carries an optional **type observation slot** (filled by the
  VM profiler on first execution, updated on subsequent calls)
- **Deopt anchors** mark the instruction index where control must return to the
  interpreter if a compiled assumption turns out to be wrong
- Designed to be walked by `vm-core`'s interpreter loop without any additional
  lowering step

### CompilerIR  (existing — `compiler-ir` package)

A **static** IR designed for code generation:

- SSA form with explicit phi-nodes at join points
- Every value is typed at the point of IR construction (no unknown types)
- Consumed by `ir-optimizer` and then by backends (WASM, JVM, Intel 4004, …)
- Already used by the compiled-language path (Nib, NIB01)

The JIT-core (LANG03) is the bridge between the two IRs.  It reads
`InterpreterIR` plus the feedback vectors gathered by `vm-core`'s profiler,
specializes the code for the observed types, and emits `CompilerIR` that
is handed to the backend.

---

## Layer Map

```
Runtime core
  LANG00  (this spec)     Generic language pipeline architecture
  LANG01  interpreter-ir  Dynamic bytecode IR with feedback slots
  LANG02  vm-core         Generic interpreter: dispatch loop + profiler
  LANG03  jit-core        Specialization pass: InterpreterIR → CompilerIR
  LANG04  aot-core        AOT path: vm-core extracted as linkable library
  LANG05  backend-protocol  Contract every code-gen backend must satisfy

Tooling (built on top of LANG01–LANG05)
  LANG06  debug-integration   VSCode debugger via DAP; breakpoints, stepping, inspect
  LANG07  lsp-integration     Language Server Protocol; completions, hover, diagnostics
  LANG08  repl-integration    Interactive REPL via PL00 framework
  LANG09  notebook-kernel     Jupyter-compatible kernel; rich cell output

Existing specs, referenced:
  05d-debug-sidecar-format.md  offset → source location mapping (used by LANG06)
  05e-debug-adapter.md         DAP bridge to VSCode (used by LANG06)
  LS00-language-server-framework.md  generic LSP server (used by LANG07)
  LS01-lsp-language-bridge.md  language plug-in for LSP (used by LANG07)
  PL00-repl.md                 generic REPL framework (used by LANG08)
  05b-jit-compiler.md          JIT compilation concepts
  05c-jit-compilation-pipeline.md  end-to-end pipeline design
  IR00-semantic-ir.md          SemanticIR (higher-level; above CompilerIR)
  compiler-ir package          CompilerIR definition
  ir-optimizer package         SSA optimization passes
```

---

## Package Map (post-generification)

```
Language frontend (per language)
  ├── my-lang-lexer        tokeniser
  ├── my-lang-parser       AST builder
  ├── my-lang-type-checker type inference / annotation
  └── my-lang-compiler     AST → InterpreterIR

                 ↓  InterpreterIR

  ├── vm-core              generic interpreter loop + profiler (LANG02)
  └── jit-core             InterpreterIR → CompilerIR specializer (LANG03)

                 ↓  CompilerIR

  ├── ir-optimizer         constant folding, DCE, inlining
  └── backend-protocol     contract for all backends (LANG05)
          ├── intel4004-backend    Nib / Tetrad 4004 target
          ├── wasm-backend         browser / WASI target
          ├── jvm-backend          JVM class-file target
          └── riscv-backend        RISC-V binary target
```

### What each language supplies

| Component | Language-specific? | Generic package |
|-----------|-------------------|-----------------|
| Lexer | Yes | — |
| Parser | Yes | — |
| Type checker | Yes (optional) | `type-checker-protocol` |
| Bytecode compiler | Yes | emits to `interpreter-ir` |
| Debug sidecar emitter | Yes (3 lines) | `debug-sidecar` (05d) |
| LSP bridge | Yes (10–50 lines) | `ls01` + `ls00` |
| REPL plugin | Yes (20–30 lines) | `pl00` |
| Notebook kernel | Yes (5 lines) | `lang09-kernel` |
| Interpreter / VM | **No** | `vm-core` |
| JIT | **No** | `jit-core` |
| Optimization | **No** | `ir-optimizer` |
| Backend | **No** | `backend-protocol` + existing backends |
| Debugger (VSCode DAP) | **No** | `debug-adapter` (05e) |
| Language server | **No** | `ls00` (generic LSP server) |

---

## Tetrad as a language-specific wrapper

After the generification, the Tetrad packages shrink to:

```
tetrad-lexer        (unchanged)
tetrad-parser       (unchanged)
tetrad-type-checker (unchanged)
tetrad-compiler     NEW: emits InterpreterIR instead of CodeObject/Op
```

`tetrad-vm` and `tetrad-jit` become thin **configuration wrappers**:

```python
# tetrad-vm becomes roughly:
from vm_core import VMCore
from tetrad_compiler import compile_program
vm = VMCore(backend="intel4004")

# tetrad-jit becomes:
from jit_core import JITCore
jit = JITCore(vm, threshold_fully_typed=0, threshold_partial=10, threshold_untyped=100)
```

The implementation work for `tetrad-vm` and `tetrad-jit` (the register VM,
the nibble-pair codegen, the liveness-based register recycling, the tiered
promotion) moves into `vm-core` and `jit-core` respectively and becomes
available to every language for free.

---

## Historical context: why these tiers exist

When we built Tetrad we discovered that hardware constraints force the language
runtime tier to match hardware capability:

| Hardware | Year | RAM | Stack | Where runtime lives |
|----------|------|-----|-------|---------------------|
| Intel 4004 | 1971 | 160 bytes | 3-level | Cannot host interpreter |
| Intel 8008 | 1972 | up to 16 KB | 8-level hardware stack, no user SP | Cannot implement software call frames |
| Intel 8080 | 1974 | 64 KB | proper SP, PUSH/POP | First chip where an interpreter is comfortable |
| Motorola 6800 | 1974 | 64 KB | proper SP | Same tier as 8080 |
| MOS 6502 | 1975 | 64 KB | 256-byte hw stack + SP | BASIC on Apple II |

The Tetrad JIT targets the Intel 4004 **as the execution target**, not the
hosting platform.  The interpreter (and JIT compiler itself) run on the host
machine (modern hardware); only the compiled output is fed to the 4004
simulator.  This distinction is fundamental to the architecture:

- **Interpreter host** = the machine running `vm-core` (your laptop)
- **Compiled target** = the ISA simulator (Intel 4004, WASM, JVM, …)

BASIC became popular on the IBM PC not because the 8088 needed BASIC but
because the 8088 *could finally host* the BASIC interpreter comfortably.
Tetrad's VM has the same relationship: `vm-core` runs on modern hardware;
the 4004 backend is where the *compiled output* executes.

---

## Roadmap

The LANG spec series is the specification layer.  Implementation proceeds in
dependency order:

```
Runtime core (implement first):
  LANG01 interpreter-ir   → defines the shared bytecode format
  LANG02 vm-core          → consumes interpreter-ir; hosts the profiler
  LANG03 jit-core         → consumes vm-core feedback; emits compiler-ir
  LANG04 aot-core         → headless vm-core path (no interpreter overhead)
  LANG05 backend-protocol → codifies what intel4004-backend etc. must implement
  LANG19 codegen-core     → unified optimize+compile pipeline for ALL backends

Tooling layer (implement after LANG01–LANG05):
  LANG06 debug-integration  → vm-core debug hooks; DAP bridge via 05d/05e
  LANG07 lsp-integration    → LS01 bridge; incremental re-parse; background infer
  LANG08 repl-integration   → PL00 LanguagePlugin; incremental IIRModule state
  LANG09 notebook-kernel    → JMP over ZeroMQ; cell execution; rich output
```

`codegen-core` (LANG19) sits between the specialisation passes (JIT/AOT)
and the hardware backends.  It provides a generic `CodegenPipeline[IR]`
that works for both the interpreted-language path (`list[CIRInstr]`) and
the compiled-language path (`IrProgram`), eliminating the duplicate
optimize-then-compile code that previously lived separately in `jit-core`,
`aot-core`, and each compiled-language compiler.

After LANG01–LANG09 are implemented, each new language is:
- **4 packages** for the runtime (lexer, parser, type-checker, compiler)
- **~100 lines** of glue code for the full tooling suite (debugger, LSP, REPL, notebook)

A language author who has never written a compiler gets, for free:
VSCode syntax highlighting, red squiggles, Go to Definition, rename, an
interactive REPL, Jupyter notebook support, and a VSCode debugger with
breakpoints and variable inspection.

---

## Non-goals

- **Garbage collection** — out of scope for LANG00–LANG09.  Languages that
  need GC will be addressed in a future spec series (GC00+).
- **Concurrency** — single-threaded execution only.
- **AOT compilation of the VM itself** — `vm-core` runs interpreted on the
  host.  Compiling the VM to native is a future `aot-core` concern (LANG04).
- **Custom notebook frontends** — LANG09 targets standard Jupyter/VS Code
  frontends.  A custom notebook UI is out of scope.
- **Multi-file project compilation** — the REPL and notebook kernel operate
  on single-session state.  Multi-file project management (imports, modules)
  is a language-specific concern outside this spec series.
