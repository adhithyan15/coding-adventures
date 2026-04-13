# simulator-protocol

Generic `Simulator[StateT]` protocol — the shared interface all architecture simulators in this repo implement.

## What is a Simulator?

A *simulator* faithfully mimics a real CPU or microcontroller.  It accepts a
block of machine-code bytes and executes them as if it were the real chip.  At
any point you can inspect the full CPU state — registers, flags, memory, stack
— to verify that the hardware would have done exactly what you expect.

Think of it like a film projection booth.  The projector (simulator) doesn't
know which film reel (program bytes) you hand it — it just plays back whatever
you load.

## Why a Generic Protocol?

This repo has (or will have) many simulators:

| Simulator | Architecture | Year |
|---|---|---|
| `Intel4004Simulator` | 4-bit accumulator | 1971 |
| `Intel8008Simulator` | 8-bit | 1972 |
| `ARM1Simulator` | 32-bit RISC | 1985 |
| `RiscVSimulator` | open-standard RISC | 2015 |

All of them load bytes, execute instructions, and expose CPU state.  Without a
shared interface, every caller would need `isinstance` checks.  With
`Simulator[StateT]`, the compiler pipeline, end-to-end tests, and visualization
tools all speak the same language.

Python's `typing.Protocol` gives *structural subtyping*: a class satisfies the
protocol if it has the right methods with the right signatures, with no
explicit inheritance needed.  This is also called "duck typing with types."

## The `Simulator[StateT]` Generics Design

```python
class Simulator(Protocol[StateT]):
    def load(self, program: bytes) -> None: ...
    def step(self) -> StepTrace: ...
    def execute(self, program: bytes, max_steps: int = 100_000) -> ExecutionResult[StateT]: ...
    def get_state(self) -> StateT: ...
    def reset(self) -> None: ...
```

`StateT` is the architecture-specific state type.  For the Intel 4004 it is
`Intel4004State`; for ARM it would be `ARMState`.  Using a type parameter
lets mypy catch mismatches at check time:

```python
# OK
sim: Simulator[Intel4004State] = Intel4004Simulator()

# Type error — wrong state type
sim: Simulator[Intel4004State] = ARMSimulator()  # mypy complains
```

## `ExecutionResult.ok` Shorthand

`ExecutionResult.ok` is `True` only when the program halted cleanly **and**
no error was recorded:

```python
result = sim.execute(binary)

if result.ok:
    print(f"Finished in {result.steps} steps")
    print(f"Accumulator = {result.final_state.accumulator}")
else:
    print(f"Error: {result.error}")
```

## The End-to-End Testing Loop

The whole point of a generic simulator protocol is to make end-to-end testing
uniform across architectures:

```python
from simulator_protocol import Simulator

def test_addition(sim: Simulator, binary: bytes) -> None:
    result = sim.execute(binary)
    assert result.ok, f"Program failed: {result.error}"
    assert result.final_state.accumulator == 3  # 1 + 2 = 3
```

This same test function works for any simulator that implements the protocol —
Intel 4004, Intel 8008, or a future RISC-V simulator.

The full pipeline is:

```
Nib source code
    → Compiler  (source → typed AST → IR)
    → Assembler (IR → machine bytes)
    → sim.execute(bytes)
    → assert result.final_state.<field> == expected
```

## `StepTrace` for Debugging and Visualization

Every executed instruction produces a `StepTrace`:

```python
@dataclass(frozen=True)
class StepTrace:
    pc_before: int    # address where instruction was fetched
    pc_after: int     # address after execution (next instruction or jump target)
    mnemonic: str     # e.g. "ADD R2", "JUN 0x100"
    description: str  # e.g. "ADD R2 @ 0x010"
```

The `ExecutionResult.traces` list holds one entry per instruction:

```python
result = sim.execute(binary)
for trace in result.traces:
    print(f"  {trace.pc_before:03X}: {trace.mnemonic}")
```

This is invaluable for:

- **Debugging**: see exactly which instruction went wrong
- **Visualization**: feed the trace into a step-through debugger UI or timing diagram
- **Education**: show students the instruction-by-instruction execution of a real 1971 CPU

## Usage

```python
from simulator_protocol import Simulator, ExecutionResult, StepTrace

# Type-annotate a simulator variable — any architecture works
sim: Simulator[Intel4004State] = Intel4004Simulator()

# Run a program
result: ExecutionResult[Intel4004State] = sim.execute(program_bytes)

# Check success
if result.ok:
    print(f"Accumulator: {result.final_state.accumulator}")
else:
    print(f"Failed after {result.steps} steps: {result.error}")
```

## Installation

```bash
pip install coding-adventures-simulator-protocol
```

## Development

```bash
pip install -e ".[dev]"
pytest
```
