# intel8086-gatelevel

A gate-level Intel 8086 simulator in which every data-path operation routes
through logic-gate primitives — AND, OR, XOR, NOT, and ripple-carry adder —
rather than using Python's integer arithmetic directly.

## Position in the stack

```
logic-gates          (Layer 00)
arithmetic           (Layer 01)  — ripple_carry_adder, full_adder
simulator-protocol   (SIM00)    — Simulator[T] / StepTrace / ExecutionResult
intel-8086-simulator (Layer 07n or 07m) — behavioral reference, X86State
intel8086-gatelevel  (Layer 07m2)       — THIS PACKAGE
```

## What it is

The Intel 8086 (1978) is a 16-bit processor with:

- Four general-purpose 16-bit registers (AX, BX, CX, DX), each split into
  a high byte (AH/BH/CH/DH) and a low byte (AL/BL/CL/DL).
- Four pointer/index registers: SP, BP, SI, DI.
- Four segment registers: CS, DS, SS, ES.
- A 16-bit instruction pointer (IP) and FLAGS register.
- A 20-bit physical address bus: physical = (segment × 16) + offset.

Every arithmetic or logic operation in this simulator passes through
`ripple_carry_adder`, `full_adder`, `and_gate`, `or_gate`, `xor_gate`, and
`not_gate` from the `logic-gates` / `arithmetic` packages.  Python integer
arithmetic (`+`, `-`, `&`, `|`, `^`) is never used on data-path values.

## Modules

| Module | Role |
|---|---|
| `bits.py` | `int_to_bits` / `bits_to_int`; `add_8bit`, `add_16bit`, `add_20bit`; `invert_8bit`, `invert_16bit`; `compute_parity`, `compute_zero` |
| `alu.py` | `ALUResult8086`; `add16`, `sub16`, `and16`, `or16`, `xor16`, `inc16`, `dec16`, `neg16`, `not16` and 8-bit equivalents; shifts, rotates, MUL/DIV, BCD |
| `register_file.py` | `RegisterFile8086` — 16-element LSB-first bit lists for each register; 8-bit byte halves; FLAGS pack/unpack; physical-address computation |
| `decoder.py` | `DecodedInstr`; `decode_instruction` — converts raw bytes to a structured description of one instruction |
| `simulator.py` | `Intel8086GateLevelSimulator` — implements `Simulator[X86State]` |

## Usage

```python
from intel8086_gatelevel import Intel8086GateLevelSimulator

sim = Intel8086GateLevelSimulator()

# Compile a tiny program: MOV AX, 42 / HLT
program = bytes([0xB8, 0x2A, 0x00, 0xF4])

result = sim.execute(program, max_steps=1000)
print(result.final_state.ax)   # 42
print(result.halted)           # True
```

### Step-by-step execution

```python
sim = Intel8086GateLevelSimulator()
sim.load(bytes([0xB8, 0x07, 0x00,   # MOV AX, 7
                0x05, 0x03, 0x00,   # ADD AX, 3
                0xF4]))             # HLT

while not sim._halted:
    trace = sim.step()
    print(trace.pc, trace.mnemonic)

state = sim.get_state()
print(state.ax)   # 10
```

### Cross-validation against the behavioral simulator

```python
from intel_8086_simulator import Intel8086Simulator
from intel8086_gatelevel import Intel8086GateLevelSimulator

program = bytes([0xB8, 0x05, 0x00, 0x05, 0x03, 0x00, 0xF4])

beh = Intel8086Simulator()
gate = Intel8086GateLevelSimulator()

bs = beh.execute(program).final_state
gs = gate.execute(program).final_state

assert gs.ax == bs.ax
assert gs.cf == bs.cf
```

## Gate-level commitment

The data path guarantee is enforced through:

1. **`bits.py`** — every add uses `ripple_carry_adder`; every invert uses
   a loop of `not_gate` calls.
2. **`alu.py`** — every ALU function calls into `bits.py`; no `+`/`-`/`&`
   on data values.
3. **`register_file.py`** — registers are `list[int]` bit arrays;
   `physical_address` uses `add_20bit`.
4. **`simulator.py`** — IP increments via `add_16bit`; effective address
   computations via `add_16bit`; SP push/pop via `add_16bit`.

Exceptions (host arithmetic only for control logic, not data):
- Array indexing and loop bounds are Python integers (host control flow).
- MUL/DIV use Python integer arithmetic internally because a full gate-level
  multiplier/divider is outside this scope.
- BCD adjustments mirror the behavioral simulator and use Python arithmetic
  for the correction values.

## Building and testing

```bash
cd code/packages/python/intel8086-gatelevel
./BUILD    # or: bash BUILD
```

Or manually:

```bash
uv venv --quiet --clear
uv pip install -e ../logic-gates -e ../arithmetic -e ../simulator-protocol \
               -e ../intel-8086-simulator -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v
```

## Test suite

| File | What it covers |
|---|---|
| `test_bits.py` | `add_8bit`, `add_16bit`, carry propagation, parity, zero |
| `test_alu.py` | Every ALU op; overflow, carry, sign, zero flags; BCD |
| `test_register_file.py` | 16-bit read/write; byte halves; FLAGS; physical address |
| `test_decoder.py` | All instruction classes: MOV, ALU, shifts, jumps, string, BCD, I/O |
| `test_programs.py` | End-to-end programs: loops, subroutines, string ops, segments |
| `test_equivalence.py` | 40+ programs run on both simulators; states must match |
| `test_simulator_coverage.py` | All JCC conditions, LOOP variants, REP string ops, addressing modes |

Coverage target: >85 %.
