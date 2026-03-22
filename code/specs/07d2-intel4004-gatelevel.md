# 07d2 — Intel 4004 Gate-Level Simulator

## Overview

The gate-level 4004 simulator models the world's first commercial microprocessor
at the hardware level. Every arithmetic operation routes through actual logic gate
functions — AND, OR, XOR, NOT — chained into adders, then into a 4-bit ALU. Registers
are built from D flip-flops. The program counter is a 12-bit register with an
incrementer built from half-adders. The instruction decoder is a combinational
gate tree.

This is NOT the same as the behavioral simulator (`07d-intel4004-simulator.md`).
The behavioral simulator executes instructions directly with host-language integers.
This simulator routes everything through the gate abstractions we built from scratch,
showing exactly how the real 4004 computed at the circuit level.

Both simulators implement the same 46-instruction set and produce identical results
for any program. The difference is the execution path:

```
Behavioral:  opcode → match statement → host arithmetic → result
Gate-level:  opcode → decoder gates → ALU gates → adder gates → logic gates → result
```

## Layer Position

```
[Logic Gates] → [Arithmetic] → [CPU] → [YOU ARE HERE] → Assembler → ...
     ↑              ↑            ↑           ↑
  AND/OR/NOT    Adders/ALU    Registers   4004 wiring
```

This package composes packages from layers below:
- `logic-gates`: AND, OR, XOR, NOT, MUX, decoder, D flip-flop, register
- `arithmetic`: half_adder, full_adder, ripple_carry_adder, ALU
- `clock`: Clock, ClockEdge

## Why Gate-Level?

The real Intel 4004 had exactly 2,300 transistors. Every single one of those transistors
implemented some combination of NAND/NOR gates, which were wired into adders, registers,
multiplexers, and a decoder. By building our simulator the same way — from gates up —
we can:

1. **Count gates**: How many AND/OR/NOT operations does `ADD R3` actually require?
2. **Trace signals**: Follow a bit from register R3 through the ALU and into the accumulator
3. **Understand timing**: See that a ripple-carry add through 4 full-adders takes 4 gate delays
4. **Appreciate the constraint**: 2,300 transistors is incredibly few — see where they all go

## Architecture — Block Diagram

```
                    ┌─────────────────────────────────────────┐
                    │          Intel 4004 (Gate-Level)         │
                    │                                         │
    ROM ──────────→ │  ┌──────────┐    ┌──────────────────┐   │
    (program        │  │ Program  │    │   Instruction     │   │
     bytes)         │  │ Counter  │───→│   Decoder         │   │
                    │  │ (12-bit) │    │   (gate trees)    │   │
                    │  └──────────┘    └────────┬─────────┘   │
                    │       ↑                   │             │
                    │       │            control signals      │
                    │       │                   │             │
                    │  ┌────┴───┐        ┌──────┴──────┐      │
                    │  │ Stack  │        │  Control    │      │
                    │  │ 3×12b  │        │  Unit (FSM) │      │
                    │  └────────┘        └──────┬──────┘      │
                    │                          │              │
                    │       ┌──────────────────┼──────┐       │
                    │       │                  │      │       │
                    │  ┌────┴────┐   ┌────────┴┐  ┌──┴──┐   │
                    │  │Register │   │  4-bit   │  │Carry│   │
                    │  │  File   │──→│  ALU     │──│Flag │   │
                    │  │16×4-bit │   │(gates!)  │  └─────┘   │
                    │  └─────────┘   └──────────┘            │
                    │       ↑             ↑                   │
                    │  ┌────┴────┐   ┌────┴────┐             │
                    │  │  Accum  │   │   RAM    │             │
                    │  │ (4-bit) │   │  Banks   │             │
                    │  └─────────┘   └─────────┘             │
                    └─────────────────────────────────────────┘
```

## Components

### 1. bits.py — Bit Conversion Helpers

Converts between integer values (0–15) and bit lists (LSB-first, as used by the
arithmetic package).

```python
def int_to_bits(value: int, width: int) -> list[int]:
    """Convert integer to bit list (LSB first). E.g., 5 → [1, 0, 1, 0]."""

def bits_to_int(bits: list[int]) -> int:
    """Convert bit list (LSB first) to integer. E.g., [1, 0, 1, 0] → 5."""
```

### 2. alu.py — 4-Bit ALU (Wraps arithmetic.ALU)

Thin wrapper around `ALU(bit_width=4)` from the arithmetic package. The existing
ALU already routes through gates internally:

```
ALU.execute(ADD, a_bits, b_bits)
  → ripple_carry_adder(a_bits, b_bits)
    → full_adder(a[0], b[0], 0)
      → half_adder(a[0], b[0])  →  XOR(a, b), AND(a, b)
      → half_adder(sum, cin)    →  XOR(sum, cin), AND(sum, cin)
      → OR(carry1, carry2)
    → full_adder(a[1], b[1], carry)
    → full_adder(a[2], b[2], carry)
    → full_adder(a[3], b[3], carry)
```

Every ADD instruction traverses this entire chain of gate calls.

```python
class Intel4004ALU:
    def __init__(self) -> None:
        self._alu = ALU(bit_width=4)

    def add(self, a: int, b: int, carry_in: bool) -> tuple[int, bool]:
        """Add two 4-bit values with carry. Returns (result, carry_out)."""

    def subtract(self, a: int, b: int, borrow_in: bool) -> tuple[int, bool]:
        """Subtract b from a with borrow. Returns (result, borrow_out)."""

    def bitwise_and(self, a: int, b: int) -> int: ...
    def bitwise_or(self, a: int, b: int) -> int: ...
    def bitwise_xor(self, a: int, b: int) -> int: ...
    def bitwise_not(self, a: int) -> int: ...
    def increment(self, a: int) -> tuple[int, bool]: ...
    def decrement(self, a: int) -> tuple[int, bool]: ...
    def rotate_left(self, a: int, carry_in: bool) -> tuple[int, bool]: ...
    def rotate_right(self, a: int, carry_in: bool) -> tuple[int, bool]: ...
```

### 3. registers.py — Register File (Sequential Logic)

16 × 4-bit registers built from D flip-flops via the `register()` function from
`logic_gates.sequential`.

```python
class Intel4004RegisterFile:
    def __init__(self) -> None:
        # 16 registers, each a sequential.register(width=4)
        # Plus 4-bit accumulator
        # Plus 1-bit carry flag (a D flip-flop)

    def read(self, index: int) -> int: ...
    def write(self, index: int, value: int) -> None: ...
    def read_pair(self, pair: int) -> int: ...      # 8-bit pair value
    def write_pair(self, pair: int, value: int) -> None: ...

    @property
    def accumulator(self) -> int: ...
    @accumulator.setter
    def accumulator(self, value: int) -> None: ...

    @property
    def carry(self) -> bool: ...
    @carry.setter
    def carry(self, value: bool) -> None: ...
```

Each register write goes through:
```
value → int_to_bits(value, 4) → register(bits, clock_edge, state, width=4) → state updated
```

The register function internally uses D flip-flops, which use SR latches, which use
NOR gates. So writing a register traverses: register → D_flip_flop × 4 → SR_latch × 8
→ NOR gate × 16.

### 4. decoder.py — Instruction Decoder (Combinational Gates)

The real 4004 used a PLA (Programmable Logic Array) to decode instructions. We model
this with AND/OR/NOT gate trees that pattern-match on opcode bits.

```python
@dataclass
class DecoderOutput:
    """Control signals produced by the instruction decoder."""
    alu_op: str           # "add", "sub", "and", "not", etc.
    reg_select: int       # Which register (0-15)
    pair_select: int      # Which pair (0-7)
    use_accumulator: bool
    write_accumulator: bool
    write_register: bool
    write_carry: bool
    is_jump: bool
    is_call: bool
    is_return: bool
    is_memory_read: bool
    is_memory_write: bool
    is_two_byte: bool
    is_halt: bool
    immediate: int        # Lower nibble (for LDM, BBL)

def decode(instruction_bits: list[int]) -> DecoderOutput:
    """Decode 8 instruction bits into control signals using gate logic.

    The decoder is pure combinational logic — no state, no clock.
    Input: 8 bits (MSB first: bit7, bit6, ..., bit0)
    Output: control signals that drive the rest of the CPU.
    """
```

The decoder uses AND gates to match specific bit patterns and OR gates to combine
cases. For example, to detect an ADD instruction (0x8_):

```
is_add = AND(bit7, NOT(bit6), NOT(bit5), NOT(bit4))
         ↑ bit7=1, bit6=0, bit5=0, bit4=0 → upper nibble = 0x8
```

### 5. pc.py — Program Counter (12-bit Register + Incrementer)

```python
class Intel4004PC:
    def __init__(self) -> None:
        # 12-bit register using sequential.register(width=12)
        # Incrementer: chain of 12 half-adders

    @property
    def value(self) -> int: ...         # Current PC (0-4095)

    def increment(self) -> None: ...    # PC += 1 (using half-adder chain)
    def load(self, address: int) -> None: ...  # PC = address (for jumps)
    def increment_twice(self) -> None: ...     # PC += 2 (for 2-byte instructions)
```

The incrementer is built from 12 chained half-adders (from arithmetic.adders):
```
half_adder(pc_bit0, 1) → sum0, carry0
half_adder(pc_bit1, carry0) → sum1, carry1
...
half_adder(pc_bit11, carry10) → sum11, carry11
```

### 6. stack.py — 3-Level Hardware Stack

```python
class Intel4004Stack:
    def __init__(self) -> None:
        # 3 × 12-bit registers (sequential.register(width=12))
        # 2-bit stack pointer (wraps mod 3)

    def push(self, address: int) -> None:
        """Push return address. Wraps on 4th push (overwrites oldest)."""

    def pop(self) -> int:
        """Pop return address. Wraps on underflow."""

    @property
    def depth(self) -> int: ...  # Current stack depth (0-3)
```

### 7. ram.py — RAM Banks

```python
class Intel4004RAM:
    def __init__(self) -> None:
        # 4 banks × 4 registers × (16 main + 4 status) nibbles
        # Each nibble: sequential.register(width=4)
        # Total: 320 × 4 = 1,280 flip-flops

    def set_bank(self, bank: int) -> None: ...
    def set_address(self, address: int) -> None: ...  # From SRC (8-bit)

    def read_main(self) -> int: ...       # Read current main character
    def write_main(self, value: int) -> None: ...
    def read_status(self, index: int) -> int: ...     # Read status char 0-3
    def write_status(self, index: int, value: int) -> None: ...

    @property
    def output_port(self) -> int: ...     # WMP target
    @output_port.setter
    def output_port(self, value: int) -> None: ...
```

### 8. control.py — Control Unit (FSM)

The control unit is a finite state machine that sequences the fetch-decode-execute
cycle. It drives the clock and generates phase-specific control signals.

```python
class Intel4004ControlUnit:
    def __init__(self, clock: Clock) -> None:
        # FSM states: FETCH, FETCH2, DECODE, EXECUTE
        # Uses clock edges to advance

    def step(self) -> ControlSignals:
        """Advance one instruction. May take multiple clock cycles."""
```

The real 4004 used an 8-phase machine cycle (A1, A2, A3, M1, M2, X1, X2, X3).
Each instruction took 1 or 2 machine cycles (8 or 16 clock phases). We simplify
to a 4-state FSM that captures the essential sequencing.

### 9. cpu.py — Top-Level Wiring

```python
class Intel4004GateLevel:
    def __init__(self) -> None:
        self._alu = Intel4004ALU()
        self._registers = Intel4004RegisterFile()
        self._decoder = decode  # The combinational decoder function
        self._pc = Intel4004PC()
        self._stack = Intel4004Stack()
        self._ram = Intel4004RAM()
        self._control = Intel4004ControlUnit(Clock(frequency_hz=740_000))
        self._rom = bytearray(4096)

    # --- Same public API as behavioral simulator ---
    def load_program(self, rom: bytes, start_address: int = 0) -> None: ...
    def step(self) -> Intel4004Trace: ...
    def run(self, program: bytes, max_steps: int = 10000) -> list[Intel4004Trace]: ...
    def reset(self) -> None: ...

    @property
    def accumulator(self) -> int: ...
    @property
    def registers(self) -> list[int]: ...
    @property
    def carry(self) -> bool: ...
    @property
    def pc(self) -> int: ...

    # --- Gate-level specific ---
    def gate_count(self) -> dict[str, int]:
        """Count gates used by component. Educational comparison with 2,300 transistors."""
        # Returns: {"alu": N, "registers": N, "decoder": N, "pc": N, ...}

    def inspect_internals(self) -> dict:
        """Return internal state for debugging (bit-level register contents, etc.)."""
```

## Execution Flow (One ADD Instruction)

Here's what happens when the gate-level simulator executes `ADD R3` (0x83):

```
1. FETCH
   PC value (12 bits) → ROM address → read byte 0x83
   PC.increment() → 12 half-adders chain to compute PC+1

2. DECODE
   0x83 → 8 bits [1,0,0,0,0,0,1,1] (MSB first)
   → decoder gate tree:
     bit7=1, bit6=0, bit5=0, bit4=0 → AND gates detect "0x8_ = ADD"
     bit3=0, bit2=0, bit1=1, bit0=1 → register select = 3
   → DecoderOutput(alu_op="add", reg_select=3, write_accumulator=True, ...)

3. EXECUTE
   Read R3 → register_file.read(3) → multiplexer gates select register 3
   Read A  → register_file.accumulator
   Read carry → register_file.carry

   ALU.add(A, R3, carry):
     a_bits = int_to_bits(A, 4)     → e.g., [1, 0, 1, 0] for A=5
     b_bits = int_to_bits(R3, 4)    → e.g., [1, 1, 0, 0] for R3=3
     → ripple_carry_adder(a_bits, b_bits):
       → full_adder(1, 1, 0):  XOR→XOR→AND→AND→OR → sum=0, carry=1
       → full_adder(0, 1, 1):  XOR→XOR→AND→AND→OR → sum=0, carry=1
       → full_adder(1, 0, 1):  XOR→XOR→AND→AND→OR → sum=0, carry=1
       → full_adder(0, 0, 1):  XOR→XOR→AND→AND→OR → sum=1, carry=0
     result_bits = [0, 0, 0, 1] → bits_to_int → 8
     carry_out = 0

   Write A = 8 → register_file.accumulator = 8
     → 4 D flip-flops updated via register() calls
   Write carry = False → register_file.carry = False
     → 1 D flip-flop updated
```

Total gate operations for one ADD: ~60+ individual gate function calls.

## Gate Count Estimates

| Component | Gates (approx) | Transistors (approx) |
|-----------|----------------|---------------------|
| ALU (4-bit) | ~80 | ~200 |
| Register file (16×4) | ~256 (flip-flops) | ~1,024 |
| Accumulator (4-bit) | ~16 | ~64 |
| Carry flag (1-bit) | ~4 | ~16 |
| Instruction decoder | ~120 | ~300 |
| Program counter (12-bit) | ~96 | ~240 |
| Stack (3×12-bit) | ~144 | ~360 |
| Control unit | ~30 | ~80 |
| Multiplexers | ~40 | ~100 |
| **Total** | **~786** | **~2,384** |

Close to the real 4004's 2,300 transistors!

## Dependencies

```
intel4004-gatelevel
├── logic-gates (AND, OR, XOR, NOT, MUX, D flip-flop, register)
├── arithmetic (half_adder, full_adder, ripple_carry_adder, ALU)
└── clock (Clock, ClockEdge)
```

## Test Strategy

### Unit Tests (per component)
- **ALU**: add, subtract, increment, decrement, rotate, bitwise ops with all edge cases
- **Registers**: read/write all 16 registers, pairs, accumulator, carry
- **Decoder**: verify control signals for every opcode
- **PC**: increment, load, 12-bit wrap-around
- **Stack**: push/pop 1–3 levels, wrap on 4th
- **RAM**: read/write all banks/registers/characters

### Integration Tests
- Same programs as behavioral simulator (x=1+2, multiply, subroutine, BCD)
- Verify identical results

### Cross-Validation Tests
- Run N programs on both behavioral and gate-level simulators
- Assert identical final state (accumulator, registers, carry, RAM, PC) after each
- This is the ultimate correctness guarantee

### Gate Count Tests
- Verify gate_count() reports reasonable numbers
- Compare with estimated counts above

### Performance Tests (informational)
- Measure wall-clock time for gate-level vs behavioral on same program
- Document the slowdown factor (expected: 100–1000x)
