# ir-optimizer

A pass-based IR-to-IR optimizer for the AOT compiler pipeline. Sits between
the language compiler frontend (which produces an `IrProgram`) and the backend
code generator (which emits machine code). Transforms an `IrProgram` into a
semantically equivalent but more efficient `IrProgram` by running a pipeline
of optimization passes.

## Where It Fits

```
Source Code
    ↓
Frontend (e.g., nib-compiler)
    ↓ IrProgram
ir-optimizer        ← this package
    ↓ IrProgram (optimized)
Backend (e.g., ir-to-intel-4004-compiler)
    ↓
Machine Code / ROM
```

The optimizer is the eighth component in the Nib compiler pipeline (PR 8 of 11).

## Why Optimization Matters Especially for the Intel 4004

The Intel 4004 — the CPU at the heart of the Busicom 141-PF calculator —
has exactly **4,096 bytes of ROM**. The entire calculator program, including
all arithmetic logic, keyboard handling, and display output, had to fit in
that space.

Every instruction we eliminate is a byte saved. For a resource-constrained
target like the 4004, even a few eliminated instructions can make the
difference between fitting in ROM and not fitting. This is why compiler
optimization matters even for simple programs.

## What Is an IR Optimizer?

An IR optimizer is a program that reads an intermediate representation (IR),
transforms it into a better IR, and passes the result downstream. "Better"
typically means fewer instructions, lower register pressure, or elimination
of redundant work.

The optimizer does not change what the program *does* — only how efficiently
it does it. This semantic equivalence is the core invariant every pass must
maintain.

## The `IrPass` Protocol

Every optimization is implemented as a *pass* — an object with two members:

```python
from typing import Protocol
from compiler_ir import IrProgram

class IrPass(Protocol):
    @property
    def name(self) -> str:
        """Human-readable pass name, e.g. 'DeadCodeEliminator'."""
        ...

    def run(self, program: IrProgram) -> IrProgram:
        """Return an optimized program. Never mutate the input."""
        ...
```

This is a Python **structural protocol** (PEP 544). Any class that has a
`name` property and a `run()` method automatically satisfies `IrPass` —
no explicit inheritance required. This is the same idea as Go's `io.Reader`
interface or Rust's traits.

### Why Protocols?

Protocols enable **composition without coupling**. The optimizer never imports
pass modules by name — it just calls `pass.name` and `pass.run()`. Any object
that responds to those two calls works. You can wrap a lambda, a third-party
class, or a test double without any inheritance ceremony.

### Purity Requirement

Every pass must be **pure**: it returns a new `IrProgram` and never mutates
the input. Purity gives us three guarantees:

1. **Composability** — chaining passes is safe; no pass can corrupt another's
   input.
2. **Testability** — you can assert the input program is unchanged after a
   pass runs.
3. **Reproducibility** — running the same pass on the same input always gives
   the same output.

## The Three Built-in Passes

### Pass 1: `DeadCodeEliminator`

Removes instructions that can never be reached.

**How it works:** After an unconditional branch (`JUMP`, `RET`, `HALT`),
the CPU never executes the next sequential instruction — it jumped away.
Instructions between the branch and the next label are **dead code**.

```
Before:                          After:
  JUMP  loop_end                   JUMP  loop_end
  ADD_IMM v1, v1, 1   ← dead
  LOAD_BYTE v2, v0, v1 ← dead
loop_end:                        loop_end:
  HALT                              HALT
```

Note: `BRANCH_Z` and `BRANCH_NZ` are *conditional* — the fall-through path
is still reachable, so we never mark their successors as dead.

### Pass 2: `ConstantFolder`

Folds constant expressions known at compile time into a single instruction.

**Pattern 1 — LOAD_IMM + ADD_IMM:**

```
Before:                          After:
  LOAD_IMM v1, 5                   LOAD_IMM v1, 8
  ADD_IMM  v1, v1, 3
```

**Pattern 2 — LOAD_IMM + AND_IMM:**

```
Before:                          After:
  LOAD_IMM v1, 17                  LOAD_IMM v1, 1
  AND_IMM  v1, v1, 15              ;  (17 & 15 = 1)
```

**How it works:** A single-pass scan with a `pending_load` dict that maps
register index to its known compile-time value. When a `LOAD_IMM` is seen,
its value is recorded. When an `ADD_IMM` or `AND_IMM` follows on the same
register, the two instructions are folded into one `LOAD_IMM`.

The pending value is cleared whenever any non-constant instruction writes to
that register (e.g., `LOAD_BYTE`, `ADD`, `SUB`), because we can no longer
know the register's value statically.

### Pass 3: `PeepholeOptimizer`

Local instruction-level optimizations using a sliding window of two instructions.
Iterates until a fixed point (up to 10 times).

**Pattern 1 — Merge consecutive ADD_IMM on the same register:**

```
Before:                          After:
  ADD_IMM v1, v1, 3                ADD_IMM v1, v1, 5
  ADD_IMM v1, v1, 2
```

Very common in Brainfuck: `+++` compiles to three `ADD_IMM v, v, 1`
instructions, which merge to a single `ADD_IMM v, v, 3`.

**Pattern 2 — Remove no-op AND_IMM 255:**

```
Before:                          After:
  ADD_IMM v1, v1, 1                ADD_IMM v1, v1, 1
  AND_IMM v1, v1, 255  ← no-op
```

Only removed when the preceding instruction (`ADD_IMM` or `LOAD_IMM`) has a
value in [0, 255], guaranteeing the AND is truly a no-op.

**Pattern 3 — LOAD_IMM 0 + ADD_IMM k → LOAD_IMM k:**

```
Before:                          After:
  LOAD_IMM v1, 0                   LOAD_IMM v1, 7
  ADD_IMM  v1, v1, 7
```

Loading zero and then adding `k` is the same as loading `k` directly.

## The `IrOptimizer` Pipeline

```python
from ir_optimizer import IrOptimizer

# Standard three-pass pipeline:
optimizer = IrOptimizer.default_passes()
result = optimizer.optimize(program)

# Custom pipeline:
from ir_optimizer.passes import DeadCodeEliminator, ConstantFolder
optimizer = IrOptimizer([DeadCodeEliminator(), ConstantFolder()])
result = optimizer.optimize(program)

# No-op (useful for testing):
result = IrOptimizer.no_op().optimize(program)
```

### `OptimizationResult`

`optimize()` returns an `OptimizationResult`:

```python
@dataclass
class OptimizationResult:
    program: IrProgram            # the optimized program
    passes_run: list[str]         # names of passes that ran, in order
    instructions_before: int      # instruction count before optimization
    instructions_after: int       # instruction count after optimization

    @property
    def instructions_eliminated(self) -> int: ...  # before - after
```

```python
result = IrOptimizer.default_passes().optimize(program)
print(f"Eliminated {result.instructions_eliminated} instructions")
print(f"Passes: {result.passes_run}")
# Eliminated 5 instructions
# Passes: ['DeadCodeEliminator', 'ConstantFolder', 'PeepholeOptimizer']
```

### The Default Pass Order

The standard pipeline runs passes in this order:

1. **DeadCodeEliminator first** — removes dead instructions before folding, so
   the folder never wastes time on code that will be thrown away.
2. **ConstantFolder second** — merges loads and arithmetic, which may create new
   patterns for the peephole pass to catch.
3. **PeepholeOptimizer last** — cleans up the instruction stream. Iterates to
   fixed point so cascading improvements are caught.

## Convenience Function

```python
from ir_optimizer import optimize

# Simplest form — default pipeline:
result = optimize(program)

# Custom passes:
result = optimize(program, passes=[DeadCodeEliminator()])
```

## How to Add a New Pass

1. Create a file in `src/ir_optimizer/passes/my_pass.py`
2. Implement a class with `name` property and `run()` method:

```python
from __future__ import annotations
from compiler_ir import IrProgram

class MyPass:
    @property
    def name(self) -> str:
        return "MyPass"

    def run(self, program: IrProgram) -> IrProgram:
        # ... transform instructions ...
        return IrProgram(
            instructions=new_instrs,
            data=program.data,
            entry_label=program.entry_label,
            version=program.version,
        )
```

3. Export it from `src/ir_optimizer/passes/__init__.py`
4. Add it to the `default_passes()` pipeline if appropriate
5. Write tests in `tests/test_passes/test_my_pass.py`

## Installation

```bash
pip install coding-adventures-ir-optimizer
```

## Development

```bash
# Install with dev dependencies
pip install -e ".[dev]"

# Run tests with coverage
pytest

# Lint
ruff check src/ tests/
```

## Dependencies

- `coding-adventures-compiler-ir` — provides `IrProgram`, `IrInstruction`,
  `IrOp`, and all IR types
