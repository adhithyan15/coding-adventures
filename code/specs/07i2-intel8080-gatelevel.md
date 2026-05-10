# 07i2 — Intel 8080 Gate-Level Simulator

## Overview

The gate-level 8080 simulator models the Intel 8080A microprocessor at the
hardware level. Every arithmetic operation routes through actual logic gate functions —
AND, OR, XOR, NOT — chained into half-adders, full-adders, an 8-bit ALU, a 16-bit
incrementer, and an 8-bit register file. The program counter uses a 16-bit ripple-carry
incrementer built from half-adders. The stack pointer is a 16-bit register with its
own incrementer/decrementer. The instruction decoder is a combinational gate tree
that pattern-matches opcode bits into control signals.

This is NOT the same as the behavioral simulator (`07i-intel8080-simulator.md`).
The behavioral simulator executes instructions directly with host-language integers.
This simulator routes everything through gate abstractions built from the bottom up,
showing exactly how the real 8080 computed at the circuit level.

Both simulators implement the same instruction set and produce identical results for
any valid program. The difference is the execution path:

```
Behavioral:  opcode → match statement → host arithmetic → result
Gate-level:  opcode → decoder gates → control signals → ALU gates → result
```

## Layer Position

```
[Logic Gates] → [Arithmetic] → [CPU] → [YOU ARE HERE] → Assembler → ...
     ↑              ↑            ↑            ↑
  AND/OR/NOT    Adders/ALU    Registers   8080 wiring
```

This package composes:
- `logic-gates`: `and_gate`, `or_gate`, `xor_gate`, `not_gate`, `mux_2to1`,
  `d_flip_flop`, `register_8bit`
- `arithmetic`: `half_adder`, `full_adder`, `ripple_carry_adder_8bit`,
  `ripple_carry_adder_16bit`, `alu_8bit`
- `clock`: `Clock`, `ClockEdge`

## Why Gate-Level for the 8080?

The Intel 8080 contained approximately 6,000 transistors — 73% more than the 8008's
3,500. Those extra transistors paid for expanded capability at every level:

| Feature | 8008 | 8080 | Gates added |
|---------|------|------|-------------|
| Data bus width | 8 bit | 8 bit | (same) |
| Address bus | 14 bit | 16 bit | +2 address bits → +2 adder stages |
| Stack | 8-level internal | 16-bit SP in RAM | Stack replaced by SP register + 16-bit adder |
| ALU | 8-bit ripple | 8-bit ripple | (same ALU; more control lines) |
| Decoder | 8-bit decode | 8-bit decode | Richer control signal matrix |
| I/O | 8 in, 24 out | 256 in, 256 out | Port register widened to 8 bits |
| Register file | 7 × 8-bit | 7 × 8-bit + SP | SP is an extra 16-bit register |

By simulating at gate level, we can trace:
- A single ADD instruction through: decode → register read → 8-bit adder → flag logic → register write
- A CALL instruction through: 16-bit PC adder → 16-bit SP decrementer → 2 memory writes → branch
- A DAD HL instruction through: two 8-bit adders chained into 16-bit addition with carry propagation

## Architecture — Block Diagram

```
                    ┌──────────────────────────────────────────────────┐
                    │          Intel 8080A (Gate-Level)                │
                    │                                                  │
    Memory ───────→ │  ┌────────────┐    ┌──────────────────────────┐  │
    (program        │  │ Program    │    │   Instruction             │  │
     + data)        │  │ Counter    │───→│   Decoder                 │  │
                    │  │ (16-bit)   │    │   (combinational gates)   │  │
                    │  │ adder-inc  │    └─────────┬────────────────┘  │
                    │  └─────┬──────┘              │                  │
                    │        │ ↑                   │ control signals  │
                    │        │ └─────────────┐     │                  │
                    │  ┌─────┴──────┐  ┌─────┴─────┴───────┐          │
                    │  │ Stack      │  │    Control Unit    │          │
                    │  │ Pointer    │  │    (FSM / gate     │          │
                    │  │ (16-bit)   │  │     combinational) │          │
                    │  └────────────┘  └──────────┬─────────┘          │
                    │                             │                    │
                    │       ┌─────────────────────┼──────┐             │
                    │       │                     │      │             │
                    │  ┌────┴──────┐  ┌───────────┴┐  ┌──┴──┐         │
                    │  │ Register  │  │   8-bit    │  │ Flag│         │
                    │  │  File     │─→│   ALU      │──│ Reg │         │
                    │  │ 7×8-bit   │  │ (gates!)   │  └─────┘         │
                    │  └───────────┘  └────────────┘                  │
                    │       ↑              ↑                           │
                    └───────┴──────────────┴───────────────────────────┘
                            │              │
                        Memory          Memory
                        reads           writes
```

## Gate-Level Component Hierarchy

### 1. Logic Primitives (from `logic-gates`)

```
and_gate(a, b)   → int  (0 or 1)
or_gate(a, b)    → int
xor_gate(a, b)   → int
not_gate(a)      → int
nand_gate(a, b)  → int
nor_gate(a, b)   → int
mux_2to1(a, b, sel) → int   (sel=0 → a, sel=1 → b)
```

### 2. Adder Primitives (from `arithmetic`)

```
half_adder(a, b) → (sum, carry)
full_adder(a, b, cin) → (sum, carry_out)
ripple_carry_adder_8bit(a: int, b: int, cin: int) → (sum: int, cout: int)
ripple_carry_adder_16bit(a: int, b: int, cin: int) → (sum: int, cout: int)
alu_8bit(op: int, a: int, b: int, cin: int) → (result: int, flags: ALUFlags)
```

Where `ALUFlags` is a named tuple: `(carry, zero, sign, parity, aux_carry)`.

### 3. Register Components (gate-level models)

All registers are built from D flip-flops clocked by a shared `Clock`:

```python
class Register8(Component):
    """8-bit register: 8 D flip-flops with common clock and load enable."""
    def __init__(self, clock: Clock): ...
    def load(self, value: int) -> None: ...
    def read(self) -> int: ...

class Register16(Component):
    """16-bit register: 16 D flip-flops with common clock and load enable."""
    def __init__(self, clock: Clock): ...
    def load(self, value: int) -> None: ...
    def read(self) -> int: ...
    def inc(self) -> None:  """Increment via 16-bit half-adder chain."""
    def dec(self) -> None:  """Decrement via 16-bit subtractor."""
```

### 4. Register File

The 8080 register file holds A, B, C, D, E, H, L — seven 8-bit registers plus the
flag register:

```python
class RegisterFile(Component):
    """
    7 × 8-bit registers (A, B, C, D, E, H, L) plus flags.

    Access by 3-bit code (000=B, 001=C, ..., 111=A).
    The M pseudo-register (110) is handled externally by the control unit
    which substitutes a memory read/write for M references.
    """
    def read(self, reg_code: int) -> int: ...
    def write(self, reg_code: int, value: int) -> None: ...
    def read_pair(self, pair_code: int) -> int: ...  # BC, DE, HL, SP
    def write_pair(self, pair_code: int, value: int) -> None: ...
```

### 5. 8-bit ALU (gate-level)

The 8080 ALU performs: ADD, ADC, SUB, SBB, ANA, XRA, ORA, CMP, INR, DCR,
DAA, rotates, complement. The gate-level model routes every bit through the
adder chain and logical gate tree:

```python
class ALU8080(Component):
    """
    8-bit ALU built from gate primitives.

    op codes:
      0x0 = ADD   0x1 = ADC   0x2 = SUB   0x3 = SBB
      0x4 = ANA   0x5 = XRA   0x6 = ORA   0x7 = CMP
      0x8 = INR   0x9 = DCR   0xA = RLC   0xB = RRC
      0xC = RAL   0xD = RAR   0xE = CMA   0xF = DAA
    """
    def execute(
        self,
        op: int,
        a: int,
        b: int,
        carry_in: bool,
        aux_carry_in: bool,
    ) -> tuple[int, bool, bool, bool, bool, bool]:
        """Returns (result, cy, zero, sign, parity, aux_carry)."""
```

The ALU's ADD path at gate level:

```
Bit 0: half_adder(A[0], B[0])            → (S[0], C[0])
Bit 1: full_adder(A[1], B[1], C[0])      → (S[1], C[1])
Bit 2: full_adder(A[2], B[2], C[1])      → (S[2], C[2])
Bit 3: full_adder(A[3], B[3], C[2])      → (S[3], C[3])  ← aux carry = C[3]
Bit 4: full_adder(A[4], B[4], C[3])      → (S[4], C[4])
Bit 5: full_adder(A[5], B[5], C[4])      → (S[5], C[5])
Bit 6: full_adder(A[6], B[6], C[5])      → (S[6], C[6])
Bit 7: full_adder(A[7], B[7], C[6])      → (S[7], C[7])  ← carry out

Flags:
  CY  = C[7]
  Z   = NOR(S[0], S[1], S[2], S[3], S[4], S[5], S[6], S[7])
  S   = S[7]
  P   = XNOR(S[0], S[1], S[2], S[3], S[4], S[5], S[6], S[7])
  AC  = C[3]
```

Note: XNOR(all bits) = 1 when an even number of bits are 1 = even parity.
The NOR zero-detect chain is: `nor(nor(nor(S[0],S[1]), nor(S[2],S[3])), nor(nor(S[4],S[5]), nor(S[6],S[7])))`.

### 6. 16-bit Operations (PC incrementer, SP ±2, DAD)

```
PC increment: ripple_carry_adder_16bit(pc, 1, 0) → (new_pc, _)
PC + 2:       ripple_carry_adder_16bit(pc, 2, 0)
PC + 3:       ripple_carry_adder_16bit(pc, 3, 0)
SP decrement: ripple_carry_adder_16bit(sp, 0xFFFE, 0)  (two's complement -2)
SP increment: ripple_carry_adder_16bit(sp, 2, 0)
DAD:          ripple_carry_adder_16bit(hl, rp, 0) → (new_hl, cy_out)
```

### 7. Instruction Decoder

The decoder is a combinational gate tree that maps 8-bit opcodes to control signals.
It is organized as a two-level decode matching the 8080's natural groupings:

```python
class Decoder8080(Component):
    """
    Combinational instruction decoder.

    Given the 8-bit opcode, produces a DecodedInstruction with:
      - op_group: 0–3 (bits 7–6 of opcode)
      - dst: bits 5–3 (destination register code)
      - src: bits 2–0 (source register code)
      - alu_op: 3-bit ALU operation for group-10 instructions
      - is_halt: True for opcode 0x76
      - is_memory_src: True when src=110 (M pseudo-register)
      - is_memory_dst: True when dst=110 (M pseudo-register)
      - extra_bytes: 0, 1, or 2 (additional bytes needed to complete fetch)
    """

    # Gate implementation: use AND/OR/NOT on individual opcode bits
    # Group 01 (MOV): bit7=0, bit6=1
    # Group 10 (ALU reg): bit7=1, bit6=0
    # Group 11 (control): bit7=1, bit6=1
    # Group 00 (misc): bit7=0, bit6=0
```

The gate-level representation of the group decode:

```
group_bit1 = bit7
group_bit0 = bit6
is_group01 = and_gate(not_gate(bit7), bit6)
is_group10 = and_gate(bit7, not_gate(bit6))
is_group11 = and_gate(bit7, bit6)
is_group00 = and_gate(not_gate(bit7), not_gate(bit6))
```

### 8. Control Unit

The control unit is a finite-state machine that orchestrates the fetch-decode-execute
cycle. Gate-level FSMs are represented as state registers (D flip-flops) with
combinational next-state logic:

```
States:
  FETCH_1    — read opcode byte from memory[PC]; increment PC
  FETCH_2    — read second byte (if needed)
  FETCH_3    — read third byte (if needed)
  EXECUTE    — route operands to ALU/register file/memory
  WRITEBACK  — write ALU result to destination
  MEMORY_RD  — wait for memory read (M source operand)
  MEMORY_WR  — wait for memory write (M destination or stack)
  HALT       — stopped
```

```python
class ControlUnit(Component):
    """
    FSM-based control unit built from D flip-flops and combinational logic.

    Orchestrates the pipeline:
      FETCH_1 → [FETCH_2 → FETCH_3] → EXECUTE → [MEMORY_RD/WR] → WRITEBACK
    """
    def tick(self) -> StepTrace | None: ...
```

## Gate-Level Execution Trace

For a single `ADD B` instruction (opcode 0x80):

```
T1 (FETCH):
  memory[PC] → opcode register (0x80)
  PC ← PC + 1  [via 16-bit adder: ripple_carry_adder_16bit(old_pc, 1, 0)]

DECODE:
  bit7=1, bit6=0 → group10 (ALU register)
  bits5-3 = 000 → ADD
  bits2-0 = 000 → register B

EXECUTE:
  a  ← register_file.read(7)   [register A]
  b  ← register_file.read(0)   [register B]
  op ← 0x0 (ADD)
  (result, cy, z, s, p, ac) ← alu_8080.execute(ADD, a, b, carry_in=False, ...)
    internals:
      bit0: half_adder(a[0], b[0]) → (s0, c0)
      bit1: full_adder(a[1], b[1], c0) → (s1, c1)
      ...
      bit7: full_adder(a[7], b[7], c6) → (s7, cy=c7)
      Z = NOR(s0..s7)
      S = s7
      P = XNOR(s0..s7)
      AC = c3

WRITEBACK:
  register_file.write(7, result)   [A ← result]
  flag_register ← {CY=cy, Z=z, S=s, P=p, AC=ac}
```

For `CALL 0x0150` (opcode 0xCD, addr_lo=0x50, addr_hi=0x01):

```
T1 (FETCH_1):  opcode ← 0xCD; PC ← PC + 1
T2 (FETCH_2):  addr_lo ← memory[PC]; PC ← PC + 1
T3 (FETCH_3):  addr_hi ← memory[PC]; PC ← PC + 1
               target ← (addr_hi << 8) | addr_lo = 0x0150
               return_addr ← PC   (already incremented past 3 bytes)

EXECUTE:
  SP ← SP - 2  [via 16-bit adder: sp + 0xFFFE]
  memory[SP+1] ← return_addr_high
  memory[SP]   ← return_addr_low
  PC ← target  (0x0150)
```

## Python Package Layout

```
code/packages/python/intel8080-gatelevel/
├── pyproject.toml
├── BUILD
├── README.md
├── CHANGELOG.md
└── src/
    └── intel8080_gatelevel/
        ├── __init__.py       # Public API
        ├── alu.py            # ALU8080 — 8-bit ALU from gate primitives
        ├── decoder.py        # Decoder8080 — combinational opcode decoder
        ├── register_file.py  # RegisterFile, Register8, Register16
        ├── control.py        # ControlUnit FSM
        └── simulator.py      # Intel8080GateLevelSimulator
tests/
    ├── test_alu.py           # Gate-level ALU operations
    ├── test_decoder.py       # Decoder gate output for each opcode group
    ├── test_register_file.py # Register read/write via flip-flops
    ├── test_control.py       # FSM state transitions
    ├── test_parity.py        # Parity gate chain correctness
    ├── test_adder16.py       # 16-bit PC incrementer
    ├── test_equivalence.py   # Gate-level == behavioral for all instructions
    └── test_programs.py      # End-to-end program comparison
```

## SIM00 Protocol Conformance

`Intel8080GateLevelSimulator` also satisfies `Simulator[Intel8080State]`, reusing
the same `Intel8080State` frozen dataclass from `intel8080-simulator`. This means
both simulators can be used interchangeably as `Simulator[Intel8080State]` and
their outputs should be bit-for-bit identical for any valid program.

## Equivalence Guarantee

For every valid instruction in the 8080 ISA, the gate-level and behavioral simulators
must produce identical output state. This is verified by `test_equivalence.py`, which:

1. Generates a set of test programs covering every opcode group and register combination
2. Runs each program on both `Intel8080Simulator` and `Intel8080GateLevelSimulator`
3. Asserts that `final_state` is identical between the two simulators

## Key Educational Points

### Why Gate Count Matters

The 8080 has 6,000 transistors. An 8-bit ripple-carry adder takes 4 transistors per
full adder stage × 8 stages = 32 full adders = ~128 transistors. That's 2.1% of the
chip just for the adder. The 16-bit operations (PC increment, SP adjust, DAD) add
another ~256 transistors. This is why the 8080 was not cheap.

### Propagation Delay

A ripple-carry adder has propagation delay proportional to word width. For 8-bit
addition: 8 full-adder delays in the worst case (carry rippling from bit 0 to bit 7).
For 16-bit (DAD): 16 stages. This is why later processors used carry-lookahead adders.
The 8080 ran at 2–5 MHz partly because of this ripple delay.

### Flag Logic as Gates

The zero-detect circuit is a single NOR tree:

```
stage0: NOR(s0, s1) → z01;  NOR(s2, s3) → z23;  NOR(s4, s5) → z45;  NOR(s6, s7) → z67
stage1: AND(z01, z23) → z0123;  AND(z45, z67) → z4567
stage2: AND(z0123, z4567) → zero
```

Three gate delays for an 8-input NOR tree. The parity tree is similar but XOR-based.

### The M Pseudo-Register

M is not in the register file — it is implemented by the control unit as a two-cycle
operation: a memory read cycle to fetch the value, then the ALU operation, then
(for write instructions) a memory write cycle. This is why MOV M,M is illegal
(opcode 0x76 is re-used as HLT) — the address is read twice and the value is written
to the same address it came from, which is a no-op, and the designers chose HLT as
more useful.

## Differences from Behavioral Simulator

The gate-level simulator is slower (every operation goes through Python function calls
for each gate), but is architecturally accurate:

| Aspect | Behavioral | Gate-Level |
|--------|-----------|------------|
| Arithmetic | Python `+` operator | 8-bit ripple-carry adder gates |
| Flags | Computed inline | Computed via gate trees |
| PC increment | `pc += length` | 16-bit adder chain |
| Register read | Dict lookup | `Register8.read()` via flip-flop |
| Decoder | `match opcode` | Combinational gate tree |
| Speed | ~1M steps/sec | ~50K steps/sec (Python overhead) |
| Educational value | Shows what happens | Shows how it happens |
