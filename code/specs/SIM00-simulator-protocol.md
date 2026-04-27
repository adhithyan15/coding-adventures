# SIM00 — Simulator Protocol

## Overview

The **Simulator Protocol** is a shared contract that every hardware simulator
and bytecode virtual machine in this repository implements. It defines a small,
uniform Python interface — `Simulator[StateT]` for byte-input simulators and
`VirtualMachine[StateT, ProgramT]` for code-object VMs — so that the compiler
pipeline, end-to-end tests, and visualization tools can treat every execution
backend interchangeably.

Without a common protocol, switching from an Intel 4004 backend to an ARM
backend would require learning a different API. With it, the change is one line:

```python
# Before:
result = intel4004_sim.execute(binary)

# After:
result = arm_sim.execute(binary)

# The assertion is identical:
assert result.ok
assert result.final_state.registers[0] == 42
```

This uniformity is the whole point. The protocol is defined using Python's
`typing.Protocol` mechanism — *structural subtyping* (also called duck typing
with types). No inheritance is needed. Any class that has the right methods with
the right signatures **automatically** satisfies the protocol.

## Layer Position

```
Logic Gates → Arithmetic → CPU → ISA Simulator ← [YOU ARE HERE]
                                       ↑
                             Bytecode VM Simulator
                                       ↑
                             CodeObject VM Simulator
```

The protocol sits *above* all simulators as a horizontal cross-cutting concern.
It is not a layer in the compilation pipeline — it is the *interface* that all
simulation layers expose.

---

## Motivation and Design Goals

### Goal 1 — uniform testability

Every simulator in this repo should be testable through one common interface.
A test helper like `assert_executes_correctly(sim, binary)` should work for
any simulator regardless of architecture.

### Goal 2 — compiler integration tests

The Nib compiler's end-to-end tests follow a single pattern:

```
Source → Compile → Assemble → simulate.execute(binary) → assert state
```

The `simulate.execute()` call must be architecture-agnostic. Adding a new
backend (say, RISC-V or WASM) should not require rewriting any test harness
code.

### Goal 3 — backend substitutability

If the Intel 4004 backend is slow for a particular test, you should be able to
swap in a faster VM without changing any test assertions. This is the
*Liskov Substitution Principle* expressed as a protocol contract.

### Goal 4 — frozen state snapshots

State snapshots must be immutable. This prevents a subtle class of bugs:

```python
# BUG without frozen snapshots:
state_before = sim.get_state()   # not a snapshot — just a reference
sim.execute(more_code)           # mutates the "state_before" object
assert state_before.registers[0] == 5   # fails — value has changed!

# With frozen dataclasses this is impossible — FrozenInstanceError is raised
# the moment anything tries to mutate the returned snapshot.
```

### Goal 5 — two variants for two input types

Hardware and bytecode simulators accept `bytes` as input (raw machine code).
CodeObject VMs accept a `CodeObject` (a structured program representation).
Both variants expose the same five methods; only the input type of `load()` and
`execute()` differs.

---

## Protocol Definitions

### Shared types

Both protocol variants share `StepTrace` and `ExecutionResult`.

#### `StepTrace`

A frozen record of a single instruction execution. Every call to `step()`
returns one `StepTrace`.

```python
@dataclass(frozen=True)
class StepTrace:
    pc_before: int    # Program counter BEFORE the instruction was fetched.
                      # Expressed in the architecture's native address space
                      # (e.g., 0–4095 for the Intel 4004's 12-bit ROM).
    pc_after: int     # Program counter AFTER execution. For most instructions
                      # this is pc_before + instruction_size. For jumps/calls
                      # it is the branch target.
    mnemonic: str     # Short human-readable instruction name.
                      # Examples: "NOP", "ADD R2", "JUN 0x100", "LDM 7".
    description: str  # Longer plain-English description of what happened.
                      # Examples: "LDM 7 @ 0x003", "ADD R2 @ 0x010".
```

Think of `mnemonic` as what an assembly listing shows, and `description` as what
a debugger's "step" tooltip would say.

#### `ExecutionResult[StateT]`

The return value of `execute()`. A frozen snapshot of the complete result of
running a program.

```python
@dataclass(frozen=True)
class ExecutionResult(Generic[StateT]):
    halted: bool              # True  → program reached a HALT instruction.
                              # False → max_steps was exceeded; program may
                              #         still be running (or in an infinite loop).
    steps: int                # Total number of instructions executed.
    final_state: StateT       # Frozen snapshot of CPU/VM state at termination.
                              # The concrete type is architecture-specific:
                              # Intel4004State, ARMState, JVMState, etc.
    error: str | None         # None → clean halt, no errors.
                              # str  → describes what went wrong.
    traces: list[StepTrace]   # Full instruction-level execution trace,
                              # one StepTrace per step, in order.

    @property
    def ok(self) -> bool:
        """True iff the program halted cleanly with no error.

        Equivalent to: halted == True and error is None.
        """
        return self.halted and self.error is None
```

#### `ExecutionResult` semantics

| `halted` | `error` | `ok` | Meaning |
|----------|---------|------|---------|
| `True`   | `None`  | `True`  | Clean halt — HALT instruction reached, no errors. |
| `False`  | `str`   | `False` | max_steps exceeded — program may be in an infinite loop. |
| `True`   | `str`   | `False` | Error halt — halted due to an error condition (e.g. invalid opcode, stack overflow). |
| `False`  | `None`  | `False` | (should not occur in practice) |

The `ok` property is the primary success check in end-to-end tests:

```python
result = sim.execute(binary)
assert result.ok, f"Simulation failed: {result.error}"
# Now safe to inspect result.final_state
```

---

### `Simulator[StateT]` — byte-input simulators

For all hardware ISA simulators and stack-based bytecode VMs that accept raw
machine code bytes as input.

```python
class Simulator(Protocol[StateT]):
    def load(self, program: bytes) -> None:
        """Load binary machine code into the simulator's program memory.

        For ROM-based architectures (Intel 4004, ARM), writes to instruction
        memory starting at address 0. For RAM-based architectures, loads into
        the appropriate code segment.
        """
        ...

    def step(self) -> StepTrace:
        """Execute exactly one instruction and return a trace of what happened.

        Raises RuntimeError if the CPU is halted (call reset() first) or if no
        program has been loaded.
        """
        ...

    def execute(self, program: bytes, max_steps: int = 100_000) -> ExecutionResult[StateT]:
        """Load program, run to HALT or max_steps, return full result.

        Sequence:
          1. Reset to power-on state.
          2. Load program bytes into memory.
          3. Execute instructions one at a time.
          4. Stop at HALT or when max_steps is reached.
          5. Return an ExecutionResult with final state and full trace.
        """
        ...

    def get_state(self) -> StateT:
        """Return a frozen snapshot of current CPU/VM state.

        The returned object is immutable. Continued execution does not affect
        the snapshot. In practice, this means returning a fresh frozen
        dataclass with all mutable collections converted to tuples.
        """
        ...

    def reset(self) -> None:
        """Reset all state to power-on defaults.

        After reset():
          - All registers are cleared to 0.
          - PC is at 0.
          - Carry/overflow flags are cleared.
          - RAM/memory is zeroed.
          - Hardware stack is empty.
          - halted is False.

        Any ROM content loaded via load() persists on ROM-based architectures,
        but all execution state is reset.
        """
        ...
```

**StateT** must be a frozen dataclass. See "State Type Requirements" below.

---

### `VirtualMachine[StateT, ProgramT]` — CodeObject-input VMs

For VMs that take a structured `CodeObject` (or similar program representation)
rather than raw bytes. The `ProgramT` type parameter captures the program type —
`CodeObject` for `register-vm`, `lisp-vm`, and `starlark-vm`.

```python
class VirtualMachine(Protocol[StateT, ProgramT]):
    def load(self, program: ProgramT) -> None:
        """Load a structured program (CodeObject) into the VM."""
        ...

    def step(self) -> StepTrace:
        """Execute exactly one instruction and return a trace."""
        ...

    def execute(self, program: ProgramT, max_steps: int = 100_000) -> ExecutionResult[StateT]:
        """Load program, run to completion or max_steps, return full result."""
        ...

    def get_state(self) -> StateT:
        """Return a frozen snapshot of current VM state."""
        ...

    def reset(self) -> None:
        """Reset all VM state to initial defaults."""
        ...
```

The `brainfuck` interpreter is a special case: its "program" is a string of
`+`, `-`, `[`, `]`, `.`, `,`, `<`, `>` characters, which is compiled to a
`CodeObject` internally. It may expose either interface depending on how the
public API is designed.

---

## State Type Requirements

Every simulator must provide an `XState` frozen dataclass. It must satisfy
these minimum requirements:

### Required fields (all architectures)

| Field | Type | Description |
|-------|------|-------------|
| `pc` | `int` | Program counter at the moment the snapshot was taken. |
| `halted` | `bool` | Whether the machine has halted (HALT / `ret` / HLT instruction). |

### Architecture-specific registers / memory

Beyond the required fields, each state type captures the full architecture
state. See the per-simulator table below.

### Immutability rule

All mutable collections — register arrays, memory buffers, stacks — **must be
converted to immutable equivalents** before being placed in a state snapshot:

| Mutable (inside simulator) | Immutable (in state snapshot) |
|---------------------------|-------------------------------|
| `list[int]` | `tuple[int, ...]` |
| `bytearray` | `bytes` |
| `list[list[int]]` | `tuple[tuple[int, ...], ...]` |
| `dict[str, int]` | `types.MappingProxyType[str, int]` |

The frozen dataclass decorator (`@dataclass(frozen=True)`) enforces this at
the top level but does not recursively freeze nested lists — that is the
simulator's responsibility.

---

## Per-Simulator State Type Table

| Simulator | Package | StateT class | Key state fields |
|-----------|---------|--------------|-----------------|
| Intel 4004 | `intel4004-simulator` | `Intel4004State` | `accumulator: int` (4-bit, 0–15), `registers: tuple[int, ...]` (16 × 4-bit), `carry: bool`, `pc: int` (12-bit, 0–4095), `halted: bool`, `ram: tuple[tuple[tuple[int, ...], ...], ...]` (4 banks × 4 regs × 16 nibbles), `stack: tuple[int, int, int]` (3-level 12-bit stack), `current_ram_bank: int` |
| Intel 8008 | `intel8008-simulator` | `Intel8008State` | `registers: tuple[int, ...]` (7 × 8-bit: A B C D E H L), `memory: bytes` (16 KB flat), `stack: tuple[int, ...]` (8-level 14-bit), `flags: Intel8008Flags` (CY Z S P), `pc: int` (14-bit, 0–16383), `halted: bool` |
| ARM (ARMv7) | `arm-simulator` | `ARMState` | `registers: tuple[int, ...]` (16 × 32-bit: R0–R15), `memory: bytes` (configurable flat), `flags_n: bool`, `flags_z: bool`, `flags_c: bool`, `flags_v: bool`, `pc: int` (32-bit), `halted: bool` |
| ARM1 (ARMv1) | `arm1-simulator` | `ARM1State` | `registers: tuple[int, ...]` (16 × 32-bit visible; 25 physical with banked FIQ/IRQ/SVC), `memory: bytes` (26-bit address space), `flags_n: bool`, `flags_z: bool`, `flags_c: bool`, `flags_v: bool`, `pc: int` (24-bit effective), `mode: str` (USR/FIQ/IRQ/SVC), `halted: bool` |
| RISC-V RV32I | `riscv-simulator` | `RiscVState` | `registers: tuple[int, ...]` (32 × 32-bit: x0–x31; x0 hardwired to 0), `memory: bytes` (configurable flat), `csrs: dict[str, int]` (M-mode CSRs: mstatus, mtvec, mepc, mcause, mscratch), `pc: int` (32-bit), `halted: bool` |
| JVM | `jvm-simulator` | `JVMState` | `operand_stack: tuple[int, ...]`, `locals: tuple[int, ...]`, `constant_pool: tuple[int | str, ...]`, `pc: int`, `halted: bool` |
| CLR/CIL | `clr-simulator` | `CLRState` | `eval_stack: tuple[int, ...]`, `locals: tuple[int, ...]`, `pc: int`, `halted: bool` |
| WebAssembly | `wasm-simulator` | `WasmState` | `value_stack: tuple[int, ...]`, `locals: tuple[int, ...]`, `pc: int`, `halted: bool` |
| register-vm | `register-vm` | `RegisterVMState` | `accumulator: int`, `registers: tuple[int, ...]`, `constants: tuple[int, ...]`, `pc: int`, `halted: bool`, `feedback_vector: tuple[int, ...]` |
| lisp-vm | `lisp-vm` | `LispVMState` | `accumulator: int`, `registers: tuple[int, ...]`, `constants: tuple[int, ...]`, `pc: int`, `halted: bool` |
| starlark-vm | `starlark-vm` | `StarlarkVMState` | `value_stack: tuple[int | str | bool | None, ...]`, `locals: tuple[int | str | bool | None, ...]`, `globals_: tuple[tuple[str, int | str | bool | None], ...]`, `pc: int`, `halted: bool` |
| Brainfuck | `brainfuck` | `BrainfuckState` | `tape: tuple[int, ...]` (30,000 cells, 0–255), `data_pointer: int`, `pc: int`, `output: str`, `halted: bool` |

> **Note on unimplemented state types:** Several simulators in this repo (ARM,
> ARM1, RISC-V, JVM, CLR, WASM, and the CodeObject VMs) do not yet expose
> `get_state() -> XState` or `execute() -> ExecutionResult[XState]` conforming
> to this protocol. Adding that conformance is the implementation work this spec
> drives. See "Implementation Guide" below.

---

## `TypeVar` and Generic Variance

`StateT` is declared as a covariant type variable:

```python
StateT = TypeVar("StateT", covariant=True)
```

Covariance means that a `Simulator[Intel4004State]` is a valid
`Simulator[SimulatorStateBase]` if `Intel4004State` is a subtype of
`SimulatorStateBase`. In practice you will rarely think about this — it just
means mypy will not complain when you use a concrete simulator where a generic
one is expected.

`ProgramT` in `VirtualMachine` is invariant — the program type must match
exactly.

---

## Implementation Guide

How to add `Simulator[XState]` conformance to an existing simulator that does
not yet implement the protocol.

### Step 1 — create the `XState` frozen dataclass

Create a new file (or add to an existing `state.py`):

```python
from __future__ import annotations
from dataclasses import dataclass

@dataclass(frozen=True)
class ARMState:
    """Immutable snapshot of ARM CPU state at a point in time."""
    registers: tuple[int, ...]   # R0–R15, 32-bit each
    memory: bytes                # full memory contents (e.g., 65536 bytes)
    flags_n: bool                # Negative flag
    flags_z: bool                # Zero flag
    flags_c: bool                # Carry flag
    flags_v: bool                # Overflow flag
    pc: int                      # Program counter (address of next instruction)
    halted: bool                 # True if HLT was executed
```

Rules:
- `@dataclass(frozen=True)` is mandatory.
- All lists/bytearrays in the live simulator must become tuples/bytes in the snapshot.
- Include **all** state needed to fully reproduce what the CPU is doing at that moment.

### Step 2 — add `get_state() -> XState`

In the simulator class:

```python
def get_state(self) -> ARMState:
    return ARMState(
        registers=tuple(self.cpu.registers.to_list()),
        memory=bytes(self.cpu.memory.data),
        flags_n=self.cpu.flags.n,
        flags_z=self.cpu.flags.z,
        flags_c=self.cpu.flags.c,
        flags_v=self.cpu.flags.v,
        pc=self.cpu.pc,
        halted=self._halted,
    )
```

### Step 3 — add `execute() -> ExecutionResult[XState]`

```python
from simulator_protocol import ExecutionResult, StepTrace

def execute(
    self, program: bytes, max_steps: int = 100_000
) -> ExecutionResult[ARMState]:
    self.reset()
    self.load(program)
    traces: list[StepTrace] = []
    error: str | None = None
    for _ in range(max_steps):
        if self._halted:
            break
        trace = self.step()
        traces.append(trace)
    else:
        # max_steps exhausted without halting
        error = f"max_steps ({max_steps}) exceeded"
    return ExecutionResult(
        halted=self._halted,
        steps=len(traces),
        final_state=self.get_state(),
        error=error,
        traces=traces,
    )
```

Note: `step()` must return a `StepTrace` (from `simulator_protocol`), not the
simulator's own internal trace type (e.g., `Intel8008Trace`, `WasmStepTrace`,
`CLRTrace`). Simulators that currently return their own trace types should
either replace those types with `StepTrace` or adapt them in `step()`.

### Step 4 — add `simulator-protocol` to dependencies

In `pyproject.toml`:

```toml
[project]
dependencies = [
    "coding-adventures-simulator-protocol",
    # ... existing deps ...
]
```

In `BUILD`:

```sh
cd ../simulator-protocol && pip install -e .
cd ../arm-simulator && pip install -e .
```

### Step 5 — add conformance tests

In `tests/test_protocol_conformance.py`:

```python
from simulator_protocol import Simulator, ExecutionResult
from arm_simulator import ARMSimulator, ARMState

def test_arm_simulator_satisfies_protocol() -> None:
    """Verify ARMSimulator structurally satisfies Simulator[ARMState]."""
    sim: Simulator[ARMState] = ARMSimulator()
    # Execute a minimal program: MOV R0, #42; HLT
    binary = assemble_arm([encode_mov_imm(0, 42), encode_hlt()])
    result: ExecutionResult[ARMState] = sim.execute(binary)
    assert result.ok, f"Expected clean halt, got: {result.error}"
    assert result.final_state.registers[0] == 42
    assert result.steps > 0
    assert len(result.traces) == result.steps
```

---

## End-to-End Testing Pattern

The primary use case for this protocol is compiler integration testing. The
pattern is always:

```
[Source code] → [Compiler] → [Assembler] → [Simulator.execute()] → [Assert state]
```

### Full example with the Nib compiler

```python
from nib_compiler import compile_nib
from nib_assembler import assemble_for_4004
from intel4004_simulator import Intel4004Simulator

def test_increment_stored_value() -> None:
    """Nib: static x: u8 = 0; fn main() { x = x +% 1; }
    After execution x == 1 in Intel 4004 RAM at bank 0, register 0, char 0.
    """
    source = "static x: u8 = 0; fn main() { x = x +% 1; }"

    ir = compile_nib(source)
    binary = assemble_for_4004(ir)

    sim = Intel4004Simulator()
    result = sim.execute(binary)

    assert result.ok, f"Simulation failed: {result.error}"
    assert result.steps < 100_000, "Too many steps — possible infinite loop"
    assert result.final_state.ram[0][0][0] == 1   # x = 1
```

### Same pattern on ARM

```python
from nib_assembler import assemble_for_arm
from arm_simulator import ARMSimulator

def test_increment_arm() -> None:
    ir = compile_nib("static x: u8 = 0; fn main() { x = x +% 1; }")
    binary = assemble_for_arm(ir)

    arm_sim = ARMSimulator()
    arm_result = arm_sim.execute(binary)

    assert arm_result.ok
    assert arm_result.final_state.registers[0] == 1   # x = 1 in R0
```

Notice that the **test structure is identical** — only the simulator and the
assertion on `final_state` differ. This is the protocol's core value.

### Tracing a failed test

When a test fails, the trace list reveals exactly which instruction went wrong:

```python
result = sim.execute(binary)
if not result.ok:
    for trace in result.traces[-10:]:   # last 10 instructions
        print(f"  pc={trace.pc_before:#06x}  {trace.mnemonic:<20}  {trace.description}")
    raise AssertionError(f"Simulation error: {result.error}")
```

Example output:

```
  pc=0x003a  LDM 5                LDM 5 @ 0x003A — loaded 5 into accumulator
  pc=0x003b  ADD R2               ADD R2 @ 0x003B — A=5+R2=3 -> A=8, carry=0
  pc=0x003c  XCH R0               XCH R0 @ 0x003C — swapped A=8 with R0=0
  pc=0x003d  HLT                  HLT @ 0x003D — execution stopped
```

---

## Historical Context: Why Simulation Matters

### Jensen's Device and the Design Cycle Problem

Before microprocessors, verifying hardware required building it. The time from
"design on paper" to "working chip" was measured in months or years. Every bug
discovered after fabrication meant starting the physical manufacturing process
again.

Simulation breaks this cycle. A behavioral simulator lets you run millions of
test programs against a proposed ISA before a single transistor is etched.

### The Intel 4004 and Simulation-Driven Design

The Intel 4004 (1971) was the world's first commercial microprocessor. Federico
Faggin, Ted Hoff, and Stanley Mazor designed the 4004 for Busicom's calculator.
The chip was simulated in software before the die masks were made. Bugs found in
simulation cost nothing. Bugs found after tapeout cost the price of a new
photomask set — and months of calendar time.

This simulation-before-fabrication discipline became standard. Every chip in
this repo's simulator suite was behaviorally modeled before, during, or
alongside its physical design.

### The ARM1 and the 808-Line Simulator

The ARM1 (1985) is famous for working correctly on its very first power-on.
Sophie Wilson and Steve Furber simulated the entire chip in 808 lines of BBC
BASIC before sending it for fabrication at VLSI Technology. Steve Furber recalls:

> "The simulator let us find bugs in the microarchitecture that would have
> been catastrophically expensive to find in silicon. By the time the first
> chips arrived from the fab, we had high confidence they would work."

Sophie Wilson typed `PRINT PI` at the BBC Micro prompt and the ARM1 returned the
correct answer. The physical chip had **zero errata** on its first spin.

### Simulation Enables Correctness Proofs

A behavioral simulator is also a formal model. You can prove that a sequence of
instructions always produces a specific final state, regardless of intermediate
values — something impossible to verify exhaustively in hardware alone.

The `ExecutionResult.traces` field in this protocol exists specifically for this
purpose: it is the complete execution log, amenable to property-based testing,
mutation testing, and formal verification.

### The Value of ISA Diversity

This repo has simulators for ISAs spanning 1971 (4004) to 2017 (WASM):

| Year | ISA | Transistors | Data width |
|------|-----|-------------|------------|
| 1971 | Intel 4004 | 2,300 | 4-bit |
| 1972 | Intel 8008 | 3,500 | 8-bit |
| 1985 | ARM1 | 25,000 | 32-bit |
| ~2000 | ARMv7 | ~100M+ | 32-bit |
| 2010 | RISC-V RV32I | N/A (open) | 32-bit |
| 2017 | WebAssembly | N/A (virtual) | 32-bit |

Running the same compiler-generated code through simulators at different layers
of this stack reveals how much the underlying architecture matters — and how
much the abstraction of a common protocol hides.

---

## Relationship to Other Protocols in This Repo

### TypeChecker protocol

The `TypeChecker` protocol (defined in the type checker package) follows the
same structural-subtyping pattern:

```python
class TypeChecker(Protocol):
    def check(self, ast: AST) -> list[TypeError]: ...
```

Both protocols are consumed by the compiler pipeline without knowing the
concrete implementation. The pipeline composes them:

```
Source → [Lexer] → [Parser] → [TypeChecker] → [Compiler] → [Assembler] → [Simulator]
```

Each step in the pipeline depends only on the protocol, not the implementation.
This makes it possible to swap out any step — use a different type checker,
compile to a different ISA, run on a different simulator — without touching the
other steps.

### GenericVM

The `GenericVM` class in the `virtual-machine` package is the *implementation
base* that the Intel 4004 simulator (and others) use internally. It provides
the dispatch loop, tracing infrastructure, and step/run scaffolding. The
`Simulator[StateT]` protocol is the *interface* — you can have a simulator that
uses `GenericVM` internally and still exposes the protocol, or one that is
hand-written (like the Intel 8008) and still exposes the same protocol.

### Paint VM / P2D Pipeline

The `paint-vm` package (P2D spec series) is an example of a `VirtualMachine`
variant that takes structured `PaintInstruction` objects rather than raw bytes.
It would satisfy `VirtualMachine[PaintVMState, list[PaintInstruction]]`.

---

## Package Layout

The `simulator-protocol` package lives at:

```
code/packages/python/simulator-protocol/
├── src/
│   └── simulator_protocol/
│       ├── __init__.py        — public exports: Simulator, VirtualMachine,
│       │                        ExecutionResult, StepTrace
│       ├── protocol.py        — all protocol and dataclass definitions
│       └── py.typed           — PEP 561 marker
├── tests/
│   └── test_protocol.py       — unit tests for StepTrace and ExecutionResult
├── pyproject.toml
├── BUILD
├── README.md
└── CHANGELOG.md
```

The package name on PyPI is `coding-adventures-simulator-protocol`.

Other packages depend on it like this:

```toml
# in arm-simulator/pyproject.toml
[project]
dependencies = [
    "coding-adventures-simulator-protocol",
    "coding-adventures-cpu-simulator",
]
```

---

## Summary

| Concept | Type | Description |
|---------|------|-------------|
| `StepTrace` | frozen dataclass | Record of one instruction execution (pc_before, pc_after, mnemonic, description) |
| `ExecutionResult[StateT]` | frozen generic dataclass | Full result of running a program (halted, steps, final_state, error, traces) |
| `Simulator[StateT]` | Protocol | Interface for byte-input simulators (hardware ISAs, stack VMs) |
| `VirtualMachine[StateT, ProgramT]` | Protocol | Interface for code-object VMs (register-vm, lisp-vm, starlark-vm, brainfuck) |
| `XState` | frozen dataclass (per arch) | Architecture-specific state snapshot (registers, memory, flags, pc, halted) |

The five methods every simulator exposes: `load`, `step`, `execute`, `get_state`, `reset`.

The three result states you will encounter in tests:
- `result.ok` → clean halt, assertions on `result.final_state` are meaningful.
- `not result.ok and not result.halted` → max_steps exceeded, check `result.error` and inspect `result.traces`.
- `not result.ok and result.halted` → error halt (invalid opcode etc.), check `result.error`.
