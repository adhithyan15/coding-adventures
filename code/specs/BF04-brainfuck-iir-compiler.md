# BF04 — Brainfuck → InterpreterIR Compiler + Brainfuck VM Wrapper

## Overview

This spec introduces the Python package `brainfuck-iir-compiler`, which contains
two things:

1. **`compile_to_iir`** — a Brainfuck frontend that emits `InterpreterIR`
   (LANG01) instead of the static `CompilerIR` used by the AOT path
   (BF02 / `brainfuck-ir-compiler`).
2. **`BrainfuckVM`** — a thin wrapper around `vm-core` (LANG02) that
   pre-configures the VM for Brainfuck semantics (u8 wraparound, host
   `putchar`/`getchar` builtins) and exposes a single `run(source, ...)`
   call.  The same wrapper transparently turns on `jit-core` (LANG03) when
   constructed with `jit=True`.

The point of this package is to validate that the LANG pipeline really is
**language-agnostic**: with the compiler in place, a Brainfuck program can be
executed by the generic `vm-core` interpreter and — once tier-up heuristics
fire — specialised by the generic `jit-core`.  No Brainfuck-specific runtime,
no Brainfuck-specific JIT.  `BrainfuckVM` is the user-facing seam through
which all of this becomes one method call.

Brainfuck is the perfect first frontend to do this with:

- Eight commands, no scoping, no functions, no types beyond `u8` cells and a
  `u32` data pointer
- A single fully-typed function (no polymorphism) so the JIT path is
  exercised cleanly without dynamic-type complications
- Hot inner loops that benefit visibly from tier-up

## Layer Diagram

```
Brainfuck source
     │
     ▼
brainfuck.parser          (BF01 — produces ASTNode tree)
     │
     ▼
brainfuck-iir-compiler    (THIS SPEC — produces IIRModule)
     │
     ▼
interpreter-ir            (LANG01 — IIRInstr / IIRFunction / IIRModule)
     │
     ▼
vm-core                   (LANG02 — interpreter executes IIRModule)
     │
     └── (future) jit-core (LANG03 — specialises hot frames)
```

This is the Brainfuck mirror image of `tetrad-compiler`'s migration path
described in LANG01 §"Migration path for Tetrad", but for a much simpler
language so the prototype shakes out cleanly.

## Public API

### Compiler

```python
def compile_to_iir(ast: ASTNode, *, module_name: str = "brainfuck") -> IIRModule:
    """Compile a Brainfuck AST into a single-function IIRModule."""

def compile_source(source: str, *, module_name: str = "brainfuck") -> IIRModule:
    """Convenience: lex + parse + compile in one call."""
```

The returned `IIRModule` always contains exactly one `IIRFunction` named
`main`, with `params=[]`, `return_type="void"`, and `type_status=FULLY_TYPED`.

### `BrainfuckVM`

```python
class BrainfuckVM:
    def __init__(
        self,
        *,
        jit: bool = False,
        tape_size: int = 30_000,
        max_steps: int | None = None,
    ) -> None:
        """Construct a Brainfuck-configured VM.

        ``jit=True`` enables tier-up via ``jit-core``: the BF main function
        is FULLY_TYPED, so the JIT can specialise on first call (threshold 0).

        ``tape_size`` caps the writable tape range; addresses outside
        ``[0, tape_size)`` raise ``BrainfuckError``.

        ``max_steps`` (if set) raises ``BrainfuckError`` after that many
        IIR instructions execute, providing a fuel limit for runaway loops.
        """

    def run(
        self,
        source: str,
        *,
        input_bytes: bytes = b"",
    ) -> bytes:
        """Compile ``source``, execute it, return collected stdout bytes.

        Stdin is fed from ``input_bytes``; reading past the end yields 0
        (a common Brainfuck convention).
        """

    def compile(self, source: str) -> IIRModule:
        """Just compile, do not execute.  Useful for inspecting the IR."""

    @property
    def metrics(self) -> VMMetrics:
        """Last-run VM metrics (from vm-core).  Includes JIT hit count."""

    @property
    def vm(self) -> VMCore:
        """Direct access to the underlying VMCore for advanced use."""
```

The wrapper owns:

- a `VMCore(u8_wrap=True, profiler_enabled=True)`
- a `BuiltinRegistry` with `putchar` (appends to a per-run output buffer) and
  `getchar` (pops from a per-run input buffer)
- optionally, a `JITCore` (when `jit=True`) registered against the VM with
  thresholds tuned for FULLY_TYPED frontends (`threshold_fully_typed=0`)

`metrics.total_jit_hits > 0` after a JIT-enabled run is the observable proof
that the JIT path actually fired.

## Machine Model

The Brainfuck "machine" is mapped onto IIR primitives:

| Brainfuck concept | IIR mapping                                       |
|-------------------|---------------------------------------------------|
| 30 000-cell tape  | `vm.memory` (sparse dict keyed by integer index)  |
| Data pointer      | Local variable `ptr`, `type_hint="u32"`           |
| Cell value        | `u8`, accessed via `load_mem` / `store_mem`       |
| Output stream     | Builtin `putchar` — host-supplied callable        |
| Input stream      | Builtin `getchar` — host-supplied callable        |

The host wires `putchar` / `getchar` into the VM via
`vm.register_builtin("putchar", lambda args: ...)` before calling
`vm.execute(module)`.

`u8` wraparound semantics are obtained by constructing the VM with
`u8_wrap=True`; `+` and `-` then automatically mask to `& 0xFF`.

## Command → IIR Mapping

Each Brainfuck command compiles to a fixed sequence of `IIRInstr`s.

### Pointer movement

```
  >  →  IIRInstr("const", "k",   [1],          type_hint="u32")
        IIRInstr("add",   "ptr", ["ptr", "k"], type_hint="u32")

  <  →  IIRInstr("const", "k",   [1],          type_hint="u32")
        IIRInstr("sub",   "ptr", ["ptr", "k"], type_hint="u32")
```

### Cell mutation

```
  +  →  IIRInstr("load_mem",  "v",  ["ptr"],     type_hint="u8")
        IIRInstr("const",     "k",  [1],         type_hint="u8")
        IIRInstr("add",       "v",  ["v", "k"],  type_hint="u8")
        IIRInstr("store_mem", None, ["ptr", "v"])

  -  → analogous with "sub"
```

### I/O

```
  .  →  IIRInstr("load_mem",     "v",  ["ptr"],         type_hint="u8")
        IIRInstr("call_builtin", None, ["putchar", "v"])

  ,  →  IIRInstr("call_builtin", "v",  ["getchar"],     type_hint="u8")
        IIRInstr("store_mem",    None, ["ptr", "v"])
```

### Loops

A Brainfuck loop `[ body ]` compiles to a label–branch–label sandwich.
Loop labels are `bf_loop_<n>_start` / `bf_loop_<n>_end` where `<n>` is
the depth-first loop index — labels (unlike registers) DO need to be
unique per loop because the IIR uses label names to identify jump
targets.

```
  [   →  IIRInstr("label",        None, ["bf_loop_N_start"])
         IIRInstr("load_mem",     "c",  ["ptr"], type_hint="u8")
         IIRInstr("jmp_if_false", None, ["c", "bf_loop_N_end"])

  ]   →  IIRInstr("load_mem",     "c",  ["ptr"], type_hint="u8")
         IIRInstr("jmp_if_true",  None, ["c", "bf_loop_N_start"])
         IIRInstr("label",        None, ["bf_loop_N_end"])
```

### Program prologue

The compiled function begins with a single instruction that initialises the
data pointer register to zero:

```
        IIRInstr("const", "ptr", [0], type_hint="u32")
```

### Program epilogue

```
        IIRInstr("ret_void", None, [])
```

## Register naming (no SSA in the front-end)

InterpreterIR's textual operand grammar looks SSA-shaped, but
`vm-core`'s frame model is plain mutable registers: `frame.assign(name,
value)` overwrites the same register slot on each call.  Critically, an
SSA renaming would *break* programs with conditional control flow —
names defined only inside a skipped loop body would be undefined when
post-body code tries to read them, and the IIR has no phi-nodes that
the front-end could emit to paper over this.

So this compiler uses a small fixed set of register names:

| Name  | Purpose                                |
|-------|----------------------------------------|
| `ptr` | Data pointer                           |
| `v`   | Cell-value scratch (loaded / stored)   |
| `c`   | Loop-condition scratch                 |
| `k`   | Immediate-1 scratch                    |

Reusing these across instructions is intentional and matches how
`brainfuck-ir-compiler` allocates fixed register roles in the AOT path.

When BF05 wires the JIT, `jit-core`'s specialiser builds its own SSA
form internally; the front-end's job is just to give vm-core a
runnable program.

## Type status

Every `IIRInstr` produced by this compiler carries a concrete `type_hint`
(`u8` or `u32`) — never `"any"`.  The resulting `IIRFunction.type_status`
is therefore `FULLY_TYPED`, which lets `jit-core` (LANG03) tier up
**immediately** on first call rather than waiting for `min_observations`
observations to accumulate.  This is the cleanest possible exercise of the
JIT path.

## Test plan

The package's tests must demonstrate three things:

1. **Compilation correctness** — golden IIR for known Brainfuck snippets
   (`+`, `>+<`, `+[-]`, nested loops).

2. **Execution correctness** — for every reference program below, executing
   the compiled IIR via `vm-core` (with `u8_wrap=True` and host-wired
   `putchar` / `getchar`) produces output identical to running the same
   program through the existing `brainfuck.execute_brainfuck` interpreter.

   Reference programs:
   - `+++.` → `\x03`
   - `++>+++<+.` → cell 0 holds 3, output is `\x03`
   - `++[>+<-]>.` → 2 (the canonical move-and-add)
   - `,.` (echo)
   - The classic `Hello World!` Brainfuck program
   - A short Sierpinski / busy-loop program with deeply nested `[…]`

3. **Type status** — every compiled function has
   `type_status == FunctionTypeStatus.FULLY_TYPED` and every emitted
   `IIRInstr` has `type_hint != "any"`.

Coverage target: **≥ 95%** (CLAUDE.md library target).

## Out of scope (deferred to follow-up specs)

- **JIT wiring (BF05)** — actually attaching `jit-core` to the wrapper and
  threading specialisation through a backend.  `BrainfuckVM(jit=True)` is
  defined in this spec as the user-facing seam, but in BF04 it raises
  `NotImplementedError` pointing to BF05.  The reason for the deferral:
  Brainfuck's `load_mem`/`store_mem`/`call_builtin` instructions don't yet
  have direct lowerings in `intel4004-backend` (which was built for
  Tetrad's instruction set).  BF05 will choose between extending an
  existing backend, writing a minimal "host Python" backend that is a real
  tier-up, or routing through `aot-core` (LANG04).
- **Backend selection** — once the JIT path is online, choosing which
  backend (Intel 4004 simulator, x86-64 host, WASM) executes the specialised
  code is an orthogonal choice deferred to LANG05 backend-protocol work.
- **Peephole optimisation** — collapsing `+++++` into a single `add 5`, or
  `[-]` into a single `store_mem ptr, 0`, is a nice prototype follow-up but
  is intentionally not done here.  The point of BF04 is to thread the
  unoptimised pipeline end-to-end first.
- **Source maps** — the AOT path emits a `SourceMapChain`; the IIR path
  does not yet.  Debug integration (LANG06) will need this eventually but
  not for the prototype.

## Package layout

```
brainfuck-iir-compiler/
  pyproject.toml
  BUILD
  README.md
  CHANGELOG.md
  src/brainfuck_iir_compiler/
    __init__.py        # exports compile_to_iir, compile_source, BrainfuckVM
    compiler.py        # the AST walker → IIRModule
    vm.py              # BrainfuckVM wrapper class
    errors.py          # BrainfuckError
  tests/
    test_compile.py    # IIR shape / SSA / type_status assertions
    test_execute.py    # end-to-end VM execution vs. reference interpreter
    test_jit.py        # jit=True path: total_jit_hits > 0, output unchanged
```

## Why a separate package from `brainfuck-ir-compiler`?

The existing `brainfuck-ir-compiler` targets the **static AOT** path
(`compiler-ir`'s `IrProgram`), which is consumed by ISA backends to produce
ahead-of-time native binaries.  The IIR path is a **different IR with
different semantics** — feedback slots, deopt anchors, dispatch-loop
opcodes — and is consumed by an interpreter, not a backend.

Mixing the two into one package would force every consumer to depend on
both `compiler-ir` and `interpreter-ir`, and would entangle the AOT
source-map chain machinery with the IIR's dispatch-friendly operand
encoding.  A clean separation also matches what LANG00 calls for: each
language frontend supplies *one* `compiler` package per IR target.

A future Tetrad migration will follow the same pattern (`tetrad-compiler`
already targets IIR; `tetrad-ir-compiler` would target the static IR).
