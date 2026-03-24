# 07e2 — ARM1 Gate-Level Simulator

## Overview

The gate-level ARM1 simulator models the first ARM processor at the hardware level.
Every arithmetic operation routes through actual logic gate functions — AND, OR, XOR,
NOT — chained into adders, then into a 32-bit ALU. Registers are built from D
flip-flops. The barrel shifter is a 32×32 crossbar of multiplexers. The instruction
decoder is a combinational gate tree mapping to a PLA-like structure, just as Sophie
Wilson and Steve Furber designed the original in 1984.

This is NOT the same as the behavioral simulator (`07e-arm1-simulator.md`). The
behavioral simulator executes instructions directly with host-language integers.
This simulator routes everything through the gate abstractions we built from scratch,
showing exactly how the real ARM1 computed at the circuit level.

Both simulators implement the same ARMv1 instruction set and produce identical results
for any program. The difference is the execution path:

```
Behavioral:  opcode → match statement → host arithmetic → result
Gate-level:  opcode → decoder gates → barrel shifter gates → ALU gates → adder gates → logic gates → result
```

## Layer Position

```
[Logic Gates] → [Arithmetic] → [CPU] → [YOU ARE HERE] → Assembler → ...
     ↑              ↑            ↑           ↑
  AND/OR/NOT    Adders/ALU    Registers   ARM1 wiring
```

This package composes packages from layers below:
- `logic-gates`: AND, OR, XOR, NOT, MUX, MuxN, decoder, D flip-flop, register
- `arithmetic`: half_adder, full_adder, ripple_carry_adder, ALU
- `clock`: Clock, ClockEdge

## Why Gate-Level?

The real ARM1 had approximately 25,000 transistors. That's roughly 10× the Intel
4004's 2,300 transistors — a significant jump, but still small enough to reason
about. By building our simulator from gates up, we can:

1. **Count gates**: How many AND/OR/NOT operations does `ADD R0, R1, R2, LSL #3`
   actually require? (Spoiler: the barrel shifter alone is ~1,000 gates.)
2. **Trace signals**: Follow a bit from register R2 through the barrel shifter,
   through the 32-bit ripple-carry adder, and into register R0.
3. **Understand the barrel shifter**: The ARM1's most distinctive hardware feature
   is a crossbar network of pass transistors. We model it with multiplexer gates.
4. **See where 25,000 transistors go**: The register file alone uses ~3,200
   flip-flops. The barrel shifter uses another ~1,024 muxes. The ALU uses ~640
   gates. It adds up fast.
5. **Compare with the 4004**: The 4004 gate-level simulator uses ~786 gates. The
   ARM1 uses ~12,000+. This 15× increase buys us: 32-bit data path (8× wider),
   16 registers (same count but 8× wider), conditional execution, and the barrel
   shifter.

## Architecture — Block Diagram

```
                    ┌──────────────────────────────────────────────────────┐
                    │              ARM1 (Gate-Level)                       │
                    │                                                      │
   Memory ────────→ │  ┌──────────┐    ┌───────────────────┐              │
   (unified         │  │ Program  │    │   Instruction      │              │
    bus)            │  │ Counter  │───→│   Decoder (PLA)     │              │
                    │  │ (26-bit) │    │   (gate trees)      │              │
                    │  └──────────┘    └─────────┬──────────┘              │
                    │       ↑                    │                         │
                    │       │             control signals                  │
                    │       │                    │                         │
                    │  ┌────┴───────┐    ┌───────┴────────┐               │
                    │  │ Condition  │    │   Control       │               │
                    │  │ Evaluator  │    │   Unit (FSM)    │               │
                    │  │ (4-bit     │    └───────┬────────┘               │
                    │  │  gates)    │            │                         │
                    │  └────────────┘            │                         │
                    │                            │                         │
                    │  ┌─────────────────────────┼───────────────┐        │
                    │  │                         │               │        │
                    │  │  ┌───────────┐   ┌──────┴──────┐  ┌────┴────┐   │
                    │  │  │ Register  │   │   Barrel     │  │  32-bit │   │
                    │  │  │   File    │──→│   Shifter    │──→│  ALU    │   │
                    │  │  │ 25×32-bit │   │ (32×32 mux  │  │(gates!) │   │
                    │  │  │(flip-flops)│  │  crossbar)  │  └────┬────┘   │
                    │  │  └───────────┘   └─────────────┘       │        │
                    │  │       ↑                            ┌────┴────┐   │
                    │  │  ┌────┴────┐                       │  Flags  │   │
                    │  │  │ Address │                       │  N Z C V│   │
                    │  │  │ Register│                       └─────────┘   │
                    │  │  └─────────┘                                     │
                    │  └──────────────────────────────────────────────────┘
                    └──────────────────────────────────────────────────────┘
```

## Components

### 1. bits.{ext} — Bit Conversion Helpers

Converts between integer values and bit lists (LSB-first, as used by the
arithmetic package). Width is 32 for most ARM1 operations.

```python
def int_to_bits(value: int, width: int) -> list[int]:
    """Convert integer to bit list (LSB first). E.g., 5 → [1, 0, 1, 0, 0, ...0]."""

def bits_to_int(bits: list[int]) -> int:
    """Convert bit list (LSB first) to integer. E.g., [1, 0, 1, 0, ...0] → 5."""

def sign_extend(bits: list[int], target_width: int) -> list[int]:
    """Sign-extend a bit list to target width."""
```

### 2. alu.{ext} — 32-Bit ALU (Wraps arithmetic.ALU)

Thin wrapper around `ALU(bit_width=32)` from the arithmetic package. The existing
ALU already routes through gates internally:

```
ALU.execute(ADD, a_bits, b_bits)
  → ripple_carry_adder(a_bits, b_bits)
    → full_adder(a[0], b[0], 0)
      → half_adder(a[0], b[0])  →  XOR(a, b), AND(a, b)
      → half_adder(sum, cin)    →  XOR(sum, cin), AND(sum, cin)
      → OR(carry1, carry2)
    → full_adder(a[1], b[1], carry)
    → ... (30 more full adders)
    → full_adder(a[31], b[31], carry)
```

Every ADD instruction traverses this entire chain of 32 full adders = 160 gate calls.

```python
class ARM1ALU:
    def __init__(self) -> None:
        self._alu = ALU(bit_width=32)

    def execute(self, opcode: int, a: int, b: int, carry_in: bool) -> ALUResult:
        """Execute one of the 16 ALU operations.

        Args:
            opcode: 4-bit ALU opcode (0=AND, 1=EOR, ..., 15=MVN)
            a: First operand (Rn value), 32-bit
            b: Second operand (after barrel shifter), 32-bit
            carry_in: Current carry flag

        Returns:
            ALUResult with (value, n_flag, z_flag, c_flag, v_flag)
        """

    def compute_flags(self, opcode: int, a: int, b: int, result: int,
                      carry_out: bool, shifter_carry: bool) -> ARM1Flags:
        """Compute N, Z, C, V flags from the ALU result.

        For arithmetic ops (ADD, SUB, etc.): C comes from the adder carry-out.
        For logical ops (AND, EOR, etc.): C comes from the barrel shifter.
        V is only meaningful for arithmetic ops.
        """
```

Internally, the 16 operations are implemented using the gate-level ALU and
additional combinational logic:

```
AND:  a_bits[i] = AND(a[i], b[i])     for each of 32 bits
EOR:  a_bits[i] = XOR(a[i], b[i])
SUB:  ripple_carry_adder(a, NOT(b), carry_in=1)   (two's complement)
RSB:  ripple_carry_adder(NOT(a), b, carry_in=1)
ADD:  ripple_carry_adder(a, b, carry_in=0)
ADC:  ripple_carry_adder(a, b, carry_in=C)
SBC:  ripple_carry_adder(a, NOT(b), carry_in=C)
RSC:  ripple_carry_adder(NOT(a), b, carry_in=C)
TST:  AND (flags only)
TEQ:  EOR (flags only)
CMP:  SUB (flags only)
CMN:  ADD (flags only)
ORR:  a_bits[i] = OR(a[i], b[i])
MOV:  result = b  (pass-through)
BIC:  a_bits[i] = AND(a[i], NOT(b[i]))
MVN:  result_bits[i] = NOT(b[i])
```

### 3. barrel_shifter.{ext} — The ARM1's Signature Component

The barrel shifter is the ARM1's most distinctive hardware feature. On the real
chip, it was implemented as a 32×32 crossbar network of pass transistors — each
of the 32 output bits could be connected to any of the 32 input bits.

We model this with a tree of multiplexers built from logic gates.

```python
class ARM1BarrelShifter:
    def shift(self, value: int, shift_type: int, shift_amount: int,
              carry_in: bool) -> tuple[int, bool]:
        """Apply a shift operation to a 32-bit value.

        Args:
            value: The 32-bit value to shift (from Rm register)
            shift_type: 0=LSL, 1=LSR, 2=ASR, 3=ROR
            shift_amount: Number of positions to shift (0–31 for immediate,
                         0–255 for register, but only low byte used)
            carry_in: Current carry flag (used for RRX and shift-by-0 cases)

        Returns:
            (shifted_value, carry_out)
        """

    def decode_immediate(self, imm8: int, rotate: int) -> tuple[int, bool]:
        """Decode a rotated immediate value (for I=1 data processing).

        The 8-bit immediate is rotated right by (2 × rotate).

        Args:
            imm8: 8-bit immediate value (bits 7:0)
            rotate: 4-bit rotation amount (bits 11:8)

        Returns:
            (32-bit value, carry_out)
        """
```

**Gate-level implementation of LSL (Logical Shift Left):**

```
For each output bit i (0 to 31):
    output[i] = MuxN(input_bits, select=shift_amount)

Concretely, LSL by N means:
    output[i] = input[i - N]  if i >= N
    output[i] = 0             if i < N

This is implemented as a 5-level multiplexer tree (log2(32) = 5):
    Level 0: shift by 0 or 1   (controlled by shift_amount bit 0)
    Level 1: shift by 0 or 2   (controlled by shift_amount bit 1)
    Level 2: shift by 0 or 4   (controlled by shift_amount bit 2)
    Level 3: shift by 0 or 8   (controlled by shift_amount bit 3)
    Level 4: shift by 0 or 16  (controlled by shift_amount bit 4)

Each level: 32 Mux2 gates = 32 × (2 AND + 1 OR + 1 NOT) = 128 gates
Total for all 5 levels: 640 gates per shift type
```

### 4. registers.{ext} — Register File (Sequential Logic)

25 × 32-bit registers built from D flip-flops via the `register()` function
from `logic_gates.sequential`.

```python
class ARM1RegisterFile:
    def __init__(self) -> None:
        # Base registers: R0–R15 (16 × 32-bit)
        # FIQ banked: R8_fiq–R14_fiq (7 × 32-bit)
        # IRQ banked: R13_irq, R14_irq (2 × 32-bit)
        # SVC banked: R13_svc, R14_svc (2 × 32-bit)
        # Total: 25 × 32 = 800 flip-flops

    def read(self, index: int, mode: int) -> int:
        """Read register, returning banked version for current mode.

        Uses multiplexer gates to select between base and banked registers:
        - mode=FIQ and index in 8..14: select FIQ bank
        - mode=IRQ and index in 13..14: select IRQ bank
        - mode=SVC and index in 13..14: select SVC bank
        - otherwise: select base register
        """

    def write(self, index: int, mode: int, value: int) -> None:
        """Write register, targeting banked version for current mode."""

    def read_pc(self) -> int:
        """Read R15 (PC + flags). Returns full 32-bit value."""

    def write_pc(self, value: int) -> None:
        """Write R15 (updates both PC and flags if in privileged mode)."""
```

Each register write goes through:
```
value → int_to_bits(value, 32)
      → register(bits, clock_edge, state, width=32)
      → 32 × D_flip_flop update
      → 32 × SR_latch × 2
      → 128 × NOR gate calls
```

The multiplexer that selects between base and banked registers uses:
```
For index 13:
    output = Mux4(R13_usr, R13_fiq, R13_irq, R13_svc, mode_bits)
           = 4 × (AND + AND + OR) per bit × 32 bits = 384 gates
```

### 5. decoder.{ext} — Instruction Decoder (Combinational Gates)

The real ARM1 used a PLA (Programmable Logic Array) with 42 rows of 36-bit
microinstructions — just 1,512 bits total. We model this with AND/OR/NOT gate
trees.

```python
@dataclass
class DecoderOutput:
    """Control signals produced by the instruction decoder."""

    # Instruction class
    is_data_processing: bool
    is_load_store: bool
    is_block_transfer: bool
    is_branch: bool
    is_swi: bool
    is_coprocessor: bool
    is_undefined: bool

    # Data processing specifics
    alu_opcode: int          # 4-bit ALU operation
    set_flags: bool          # S bit
    immediate_operand: bool  # I bit

    # Barrel shifter control
    shift_type: int          # 00=LSL, 01=LSR, 10=ASR, 11=ROR
    shift_by_register: bool  # Shift amount from Rs (vs immediate)

    # Register selects
    rn: int                  # First operand register (4-bit)
    rd: int                  # Destination register (4-bit)
    rm: int                  # Second operand register (4-bit)
    rs: int                  # Shift amount register (4-bit)

    # Load/store specifics
    is_load: bool            # L bit
    is_byte: bool            # B bit
    is_pre_indexed: bool     # P bit
    is_add_offset: bool      # U bit
    is_writeback: bool       # W bit

    # Block transfer specifics
    register_list: int       # 16-bit bitmap

    # Branch specifics
    is_link: bool            # L bit (for BL)
    branch_offset: int       # 24-bit signed offset

    # Condition
    condition: int           # 4-bit condition code


def decode(instruction_bits: list[int]) -> DecoderOutput:
    """Decode 32 instruction bits into control signals using gate logic.

    The decoder is pure combinational logic — no state, no clock.
    Input: 32 bits (bit31 down to bit0)
    Output: control signals that drive the rest of the CPU.

    The primary classification uses bits 27:25:
        bit27=0, bit26=0          → Data Processing
        bit27=0, bit26=1          → Single Data Transfer
        bit27=1, bit26=0, bit25=0 → Block Data Transfer
        bit27=1, bit26=0, bit25=1 → Branch
        bit27=1, bit26=1          → Coprocessor / SWI
    """
```

Example gate-level classification:

```
is_data_processing = AND(NOT(bit27), NOT(bit26))
is_load_store      = AND(NOT(bit27), bit26)
is_block_transfer  = AND(bit27, NOT(bit26), NOT(bit25))
is_branch          = AND(bit27, NOT(bit26), bit25)
is_coprocessor     = AND(bit27, bit26, NOT(bit25), NOT(bit24))
is_swi             = AND(bit27, bit26, bit25, bit24)
```

### 6. condition_eval.{ext} — Condition Code Evaluator

```python
def evaluate_condition(condition: int, n: bool, z: bool, c: bool, v: bool) -> bool:
    """Evaluate a 4-bit condition code against current flags.

    Uses pure combinational gate logic:
        EQ:  Z
        NE:  NOT(Z)
        CS:  C
        CC:  NOT(C)
        MI:  N
        PL:  NOT(N)
        VS:  V
        VC:  NOT(V)
        HI:  AND(C, NOT(Z))
        LS:  OR(NOT(C), Z)
        GE:  XNOR(N, V)         — same as NOT(XOR(N, V))
        LT:  XOR(N, V)
        GT:  AND(NOT(Z), XNOR(N, V))
        LE:  OR(Z, XOR(N, V))
        AL:  1
        NV:  0

    The result is computed by a 16:1 multiplexer selecting from these
    16 boolean expressions, with the 4-bit condition code as the select.
    """
```

### 7. pc.{ext} — Program Counter (26-bit within R15)

```python
class ARM1PC:
    def __init__(self) -> None:
        # The PC is bits 25:2 of R15 (24 bits, representing a 26-bit
        # word-aligned address). We store it as a 26-bit register
        # and extract/insert from R15 as needed.
        # Incrementer: chain of 26 half-adders (but only bits 2-25
        # change; bits 0-1 are always 0).

    @property
    def value(self) -> int:
        """Current PC (26-bit address)."""

    def increment(self) -> None:
        """PC += 4 (advance to next instruction).
        Implemented as adding 1 to the 24-bit field at bits 25:2,
        using 24 chained half-adders.
        """

    def load(self, address: int) -> None:
        """PC = address (for branches). Only bits 25:2 are used."""
```

### 8. memory.{ext} — Memory Interface

```python
class ARM1Memory:
    def __init__(self, size: int = 64 * 1024 * 1024) -> None:
        # Byte-addressable memory (not gate-level — memory cells would
        # require millions of flip-flops). We use a host-language array
        # for memory, but all ADDRESS COMPUTATION routes through gates.

    def read_word(self, address_bits: list[int]) -> list[int]:
        """Read 32-bit word. Address is 26 bits, must be word-aligned.
        Returns 32 bits (LSB first).

        The address bus routes through AND gates to mask the bottom 2 bits,
        then through a decoder to select the memory location.
        """

    def write_word(self, address_bits: list[int], data_bits: list[int]) -> None:
        """Write 32-bit word to memory."""

    def read_byte(self, address_bits: list[int]) -> list[int]:
        """Read single byte. Returns 8 bits (LSB first).
        Uses a 4:1 MUX on address bits 1:0 to select which byte of
        the word to return.
        """

    def write_byte(self, address_bits: list[int], data_bits: list[int]) -> None:
        """Write single byte."""
```

Note: We do NOT model memory cells with flip-flops (that would require
64M × 8 = 512M flip-flops). Instead, memory storage uses host arrays but
all address computation and byte selection uses gate logic. This is the same
pragmatic choice made in the 4004 gate-level simulator.

### 9. control.{ext} — Control Unit (FSM)

```python
class ARM1ControlUnit:
    def __init__(self, clock: Clock) -> None:
        # The ARM1's 3-stage pipeline: Fetch, Decode, Execute
        # In the real chip, the PLA output 42 rows of 36-bit
        # control words. We model the sequencing with a simple FSM.

    def step(self) -> ControlPhase:
        """Advance one pipeline stage.

        The ARM1's pipeline means:
        - Cycle N:   Fetch instruction at PC
        - Cycle N+1: Decode fetched instruction
        - Cycle N+2: Execute decoded instruction

        Multi-cycle instructions (LDR, LDM, B) stall the pipeline.
        """
```

### 10. cpu.{ext} — Top-Level Wiring

```python
class ARM1GateLevel:
    def __init__(self) -> None:
        self._alu = ARM1ALU()
        self._barrel_shifter = ARM1BarrelShifter()
        self._registers = ARM1RegisterFile()
        self._decoder = decode  # Combinational decoder function
        self._condition_eval = evaluate_condition
        self._pc = ARM1PC()
        self._memory = ARM1Memory()
        self._control = ARM1ControlUnit(Clock(frequency_hz=6_000_000))

    # --- Same public API as behavioral simulator ---
    def load_program(self, machine_code: bytes, start_address: int = 0) -> None: ...
    def step(self) -> ARM1Trace: ...
    def run(self, max_steps: int = 100_000) -> list[ARM1Trace]: ...
    def reset(self) -> None: ...

    def read_register(self, index: int) -> int: ...
    def write_register(self, index: int, value: int) -> None: ...

    @property
    def pc(self) -> int: ...
    @property
    def flags(self) -> ARM1Flags: ...
    @property
    def mode(self) -> ProcessorMode: ...

    # --- Gate-level specific ---
    def gate_count(self) -> dict[str, int]:
        """Count gates used by component.

        Educational comparison with 25,000 transistors.
        Returns: {"alu": N, "barrel_shifter": N, "registers": N,
                  "decoder": N, "pc": N, "condition_eval": N, ...}
        """

    def inspect_internals(self) -> dict:
        """Return internal state for debugging (bit-level register contents,
        pipeline state, decoder outputs, barrel shifter intermediates).
        """
```

## Execution Flow (One ADD Instruction)

Here's what happens when the gate-level simulator executes
`ADDS R2, R0, R1, LSL #3` (add R0 + R1 shifted left 3, store in R2, update flags):

```
1. FETCH
   PC value (26 bits) → memory address → read 32-bit instruction word
   PC.increment() → 24 half-adders chain to compute PC+4

2. CONDITION CHECK
   Extract condition bits (31:28) → 4 bits
   condition_eval(cond, N, Z, C, V):
     → Mux16 selects one of 16 boolean expressions
     → ~20 gate calls
   If condition fails → skip to next instruction (no execute phase)

3. DECODE
   32-bit instruction → decoder gate tree:
     bits 27:26 = 00 → AND gates detect "Data Processing"
     bits 24:21 = 0100 → AND gates detect "ADD"
     bit 25 = 0 → register operand (not immediate)
     bit 20 = 1 → S bit set (update flags)
     bits 19:16 = 0000 → Rn = R0
     bits 15:12 = 0010 → Rd = R2
     bits 6:5 = 00 → LSL shift type
     bits 11:7 = 00011 → shift amount = 3
     bits 3:0 = 0001 → Rm = R1
   → DecoderOutput with all control signals
   Total: ~200 gate calls for full decode

4. REGISTER READ
   Read R0 → register_file.read(0, mode)
     → multiplexer selects register 0 from 25 physical registers
     → 32 bits output (one MuxN per bit)
   Read R1 → register_file.read(1, mode)
     → same process
   Total: ~300 gate calls (multiplexer trees for register selection)

5. BARREL SHIFT
   barrel_shifter.shift(R1_value, LSL, 3, carry_in):
     → 5-level multiplexer tree, 32 bits wide
     → Level 0 (shift by 1): 32 × Mux2, controlled by shift_amount[0]=1
       → each bit shifted: bit[i] = Mux2(bit[i], bit[i-1], select=1)
     → Level 1 (shift by 2): 32 × Mux2, controlled by shift_amount[1]=1
     → Level 2 (shift by 4): 32 × Mux2, controlled by shift_amount[2]=0
       → pass through (no shift)
     → Level 3 (shift by 8): 32 × Mux2, controlled by shift_amount[3]=0
     → Level 4 (shift by 16): 32 × Mux2, controlled by shift_amount[4]=0
   Total: 5 × 32 × ~4 = ~640 gate calls

6. ALU EXECUTE
   alu.execute(ADD, R0_bits, shifted_R1_bits, carry_in=0):
     → ripple_carry_adder(a_bits, b_bits, 0):
       → full_adder(a[0], b[0], 0)
         → half_adder(a[0], b[0]) → XOR, AND
         → half_adder(sum, 0) → XOR, AND
         → OR(carry1, carry2)
       → full_adder(a[1], b[1], carry)
       → ... (30 more full adders)
       → full_adder(a[31], b[31], carry)
     → 32 full adders × 5 gates each = 160 gate calls
   Flags:
     N = result[31]
     Z = NOR(all 32 result bits) → 31 NOR gates
     C = carry_out from adder
     V = XOR(carry_in_to_bit31, carry_out) → 1 XOR gate
   Total: ~200 gate calls

7. REGISTER WRITE
   Write R2 = result → register_file.write(2, mode, result)
     → int_to_bits(result, 32)
     → register(bits, clock_edge, state, width=32)
     → 32 × D_flip_flop update → 64 × SR_latch → 128 × NOR gate
   Write flags to R15:
     → 4 × D_flip_flop update → 8 × SR_latch → 16 × NOR gate
   Total: ~150 gate calls

GRAND TOTAL for one ADDS R2, R0, R1, LSL #3: ~1,500+ gate calls
```

## Gate Count Estimates

| Component | Gates (approx) | Transistors (approx) |
|-----------|----------------|---------------------|
| ALU (32-bit, 16 ops) | ~640 | ~1,600 |
| Barrel shifter (32×32) | ~3,200 | ~6,400 |
| Register file (25×32) | ~3,200 (flip-flops) | ~6,400 |
| Register read MUXes | ~1,600 | ~3,200 |
| Instruction decoder | ~400 | ~800 |
| Condition evaluator | ~80 | ~160 |
| Program counter (26-bit) | ~200 | ~400 |
| Address logic | ~200 | ~400 |
| Control unit (FSM) | ~100 | ~200 |
| Multiplexers (data paths) | ~600 | ~1,200 |
| **Total** | **~10,220** | **~20,760** |

Close to the real ARM1's ~25,000 transistors! The difference is accounted for
by I/O pads, clock distribution, and other analog circuitry we don't model.

## Dependencies

```
arm1-gatelevel
├── logic-gates (AND, OR, XOR, NOT, MUX, MuxN, D flip-flop, register, decoder)
├── arithmetic (half_adder, full_adder, ripple_carry_adder, ALU)
└── clock (Clock, ClockEdge)
```

## Implementation Structure

```
arm1-gatelevel/
├── bits.{ext}              Bit conversion helpers (int ↔ bit lists)
├── alu.{ext}               32-bit ALU wrapping arithmetic.ALU
├── barrel_shifter.{ext}    32×32 crossbar barrel shifter
├── registers.{ext}         25×32-bit register file with banking
├── decoder.{ext}           Instruction decoder (combinational gates)
├── condition_eval.{ext}    Condition code evaluator (16 conditions)
├── pc.{ext}                26-bit program counter with incrementer
├── memory.{ext}            Memory interface (address logic is gate-level)
├── control.{ext}           Control unit FSM
├── cpu.{ext}               Top-level wiring
└── types.{ext}             Shared types (ARM1Flags, ProcessorMode, etc.)
```

## Test Strategy

### Unit Tests (per component)

- **ALU**: All 16 operations with edge cases (overflow, underflow, carry chain,
  max values, zero). Verify flag computation for arithmetic vs logical ops.
- **Barrel Shifter**: All 4 shift types × various amounts (0, 1, 15, 16, 31, 32).
  Verify carry output. Test RRX. Test immediate decoding for all 16 rotations.
- **Registers**: Read/write all 25 physical registers. Mode switching: write R13 in
  USR, verify R13_svc is independent. FIQ banking of R8–R12.
- **Decoder**: Verify control signals for representative instructions from each class.
- **Condition Evaluator**: All 16 conditions with all flag combinations.
- **PC**: Increment, load, 26-bit wrap-around.

### Integration Tests

- Same programs as behavioral simulator (x=1+2, abs, multiply, loop, subroutine)
- Verify identical results

### Cross-Validation Tests

- Run N programs on both behavioral and gate-level simulators
- Assert identical final state (all registers, all flags, memory contents) after each step
- This is the ultimate correctness guarantee

### Gate Count Tests

- Verify `gate_count()` reports reasonable numbers
- Compare with estimated counts above

### Performance Tests (informational)

- Measure wall-clock time for gate-level vs behavioral on same program
- Document the slowdown factor (expected: 500–2000x, higher than 4004 due to
  32-bit data path and barrel shifter complexity)
