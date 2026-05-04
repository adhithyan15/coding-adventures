# intel8080-gatelevel

Gate-level simulator for the Intel 8080A microprocessor. Every arithmetic
operation routes through real logic gate functions — AND, OR, XOR, NOT —
chained into half-adders, full-adders, an 8-bit ALU, and a 16-bit
ripple-carry incrementer.

## Why Gate-Level?

The [behavioral simulator](../intel8080-simulator/) tells you *what* the 8080
computes. This simulator shows *how* it computes — at the transistor level.

The Intel 8080 contains ~6,000 transistors. An 8-bit ripple-carry adder uses
~32 transistors. This simulator traces every one of them.

## Architecture

```
opcode → Decoder8080 (gate tree) → control signals
                                          ↓
RegisterFile ──────────────────→ ALU8080 (gate chain) → RegisterFile
(7 × Register8 + Register16 SP)              ↓
                                       FlagRegister
```

## Usage

```python
from intel8080_gatelevel import Intel8080GateLevelSimulator

sim = Intel8080GateLevelSimulator()
result = sim.execute(bytes([
    0x3E, 0x0A,  # MVI A, 10
    0x06, 0x05,  # MVI B, 5
    0x80,        # ADD B
    0x76,        # HLT
]))
print(result.final_state.a)  # 15
```

## Gate-Level ADD Trace

For `ADD B` (opcode 0x80):

```
bit0: half_adder(A[0], B[0]) → (S[0], C[0])
bit1: full_adder(A[1], B[1], C[0]) → (S[1], C[1])
...
bit7: full_adder(A[7], B[7], C[6]) → (S[7], CY=C[7])
AC = C[3]
Z  = NOR-tree(S[0]..S[7])
S  = S[7]
P  = XNOR-tree(S[0]..S[7])
```

## Equivalence

The gate-level and behavioral simulators produce bit-for-bit identical results
for every valid 8080 program. Verified by `tests/test_equivalence.py`.

## Dependencies

- `logic-gates`: AND, OR, XOR, NOT, NOR, XNOR, register
- `arithmetic`: half_adder, full_adder, ripple_carry_adder
- `simulator-protocol`: SIM00 Simulator[T] interface
- `intel8080-simulator`: Intel8080State dataclass

## Layer Position

```
logic-gates → arithmetic → [YOU ARE HERE] ← intel8080-simulator
                                ↓
                       Intel8080GateLevelSimulator
                       (same Simulator[Intel8080State] interface)
```

## Historical Context

The Intel 8080 (1974) powered the Altair 8800, the first mass-market personal
computer kit. It ran at 2 MHz with a 16-bit address bus (64 KiB RAM). The
ripple-carry adder's propagation delay was partly responsible for the 2 MHz
limit — later chips used carry-lookahead adders to go faster.

See spec: `code/specs/07i2-intel8080-gatelevel.md`
