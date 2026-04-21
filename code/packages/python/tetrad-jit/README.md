# tetrad-jit

**Tetrad JIT Compiler** — profile-guided Intel 4004 native code generator
for the Tetrad programming language (spec TET05).

## What it does

`tetrad-jit` watches a running `TetradVM` and compiles hot functions to
**Intel 4004 machine code**.  Those functions then run on `Intel4004Simulator`
instead of the interpreter, demonstrating the full JIT pipeline:

```
Tetrad source
  → TetradVM (interpreted)
  → JIT: bytecode → SSA IR → optimization passes → 4004 binary
  → Intel4004Simulator (executes the binary)
```

## Why the Intel 4004?

Tetrad's VM constraints (u8 arithmetic, 4-frame call stack, 8 registers) were
modelled on the Intel 4004 (1971) — the world's first commercial
microprocessor.  The JIT closes the loop by emitting actual 4004 machine code.

The 4004 is a 4-bit machine.  Tetrad's 8-bit (u8) values are stored as
**register pairs**: the high nibble in R(2p) and the low nibble in R(2p+1).
Multi-nibble arithmetic is implemented as two 4-bit operations with carry
propagation.

## Quick start

```python
from tetrad_compiler import compile_program
from tetrad_vm import TetradVM
from tetrad_jit import TetradJIT

source = """
fn add(a: u8, b: u8) -> u8 { return a + b; }
fn main() -> u8 { return add(10, 20); }
"""

code = compile_program(source)
vm   = TetradVM()
jit  = TetradJIT(vm)

# execute_with_jit compiles FULLY_TYPED functions immediately,
# then runs the interpreter.  UNTYPED functions compile once hot.
result = jit.execute_with_jit(code)

# Manually compile and call a function:
jit.compile("add")
assert jit.is_compiled("add")
assert jit.execute("add", [200, 100]) == 44   # u8 wrap: (200+100)%256 = 44
```

## Pipeline stages

| Module | Role |
|---|---|
| `ir.py` | `IRInstr` SSA dataclass; `evaluate_op` constant evaluator |
| `translate.py` | Tetrad bytecode → JIT IR |
| `passes.py` | Constant folding + dead code elimination |
| `codegen_4004.py` | IR → Intel 4004 binary; two-pass assembler; `run_on_4004` |
| `cache.py` | `JITCache` + `JITCacheEntry` |
| `__init__.py` | `TetradJIT` public API |

## Register pair convention

| Pair | Registers | Role |
|------|-----------|------|
| P0 (R0:R1) | R0=hi, R1=lo | arg 0 / return value |
| P1 (R2:R3) | R2=hi, R3=lo | arg 1 |
| P2–P5 | R4–R11 | local virtual variables |
| P6 (R12:R13) | — | RAM address register |
| P7 (R14:R15) | — | scratch / immediate temp |

## Deoptimisation

Operations without a direct 4004 encoding are not supported in v1:
`mul`, `div`, `mod`, `and`, `or`, `xor`, `not`, `shl`, `shr`, `logical_not`,
`io_in`, `io_out`, `call`.

When the JIT encounters any of these, `compile()` returns `False` and the
function continues to run under the interpreter.

### Register pressure

The 4004 has 8 register pairs (P0–P7); P6 and P7 are reserved for RAM
addressing and scratch temporaries, leaving P0–P5 (6 pairs) for virtual
variables.  The code generator uses **liveness-based register recycling**:
a pre-scan finds each variable's last use, and dead pairs are reused before
allocating fresh ones.  Functions with ≤6 simultaneously-live variables compile
successfully even when the total SSA variable count exceeds 6 — for example,
`if`-branching functions whose two branches each need a small number of
variables (the live sets never overlap).

## Compilation tiers

| Tier | trigger |
|---|---|
| `FULLY_TYPED` | compiled before first call (`execute_with_jit`) |
| `PARTIALLY_TYPED` | compiled after 10 interpreter calls |
| `UNTYPED` | compiled after 100 interpreter calls |

## Dependencies

- `coding-adventures-tetrad-vm` — the interpreter to wrap
- `coding-adventures-intel4004-simulator` — hardware model for compiled code
