# 07f2 — Intel 8008 Gate-Level Simulator

## Overview

The gate-level 8008 simulator models the world's first 8-bit microprocessor at the
hardware level. Every arithmetic operation routes through actual logic gate functions —
AND, OR, XOR, NOT — chained into half-adders, full-adders, a ripple-carry adder, and
then an 8-bit ALU. Registers are built from D flip-flops. The 14-bit program counter
uses an incrementer built from a chain of half-adders. The 8-level push-down stack
is built from 8 × 14-bit registers. The instruction decoder is a combinational gate
tree that pattern-matches opcode bits into control signals.

This is NOT the same as the behavioral simulator (`07f-intel8008-simulator.md`).
The behavioral simulator executes instructions directly with host-language integers.
This simulator routes everything through the gate abstractions built from scratch in
`logic-gates` and `arithmetic`, showing exactly how the real 8008 computed at the
circuit level.

Both simulators implement the same instruction set and produce identical results for
any program. The difference is the execution path:

```
Behavioral:  opcode → match statement → host arithmetic → result
Gate-level:  opcode → decoder gates → ALU gates → adder gates → logic gates → result
```

## Layer Position

```
[Logic Gates] → [Arithmetic] → [CPU] → [YOU ARE HERE] → Assembler → ...
     ↑              ↑            ↑            ↑
  AND/OR/NOT    Adders/ALU    Registers   8008 wiring
```

This package composes packages from layers below:
- `logic-gates`: AND, OR, XOR, NOT, MUX, D flip-flop, register
- `arithmetic`: half_adder, full_adder, ripple_carry_adder, ALU
- `clock`: Clock, ClockEdge

## Why Gate-Level for the 8008?

The real Intel 8008 had approximately 3,500 transistors — 52% more than the 4004's
2,300. Those extra transistors paid for:
1. **8-bit datapath** instead of 4-bit (doubles the ALU and register widths)
2. **7 registers** instead of accumulator-only (register file doubled)
3. **14-bit PC** instead of 12-bit (longer address counter)
4. **8-level stack** instead of 3-level (stack doubled)
5. **Richer decoder** — 48 operations vs 46, with 3-byte instruction formats

By simulating at gate level, we can count exactly how each of those transistors
contributes to the expanded capability, and trace a bit through the full 8-bit
ripple-carry adder to see 8 gate delays (vs 4 for the 4004).

## Architecture — Block Diagram

```
                    ┌─────────────────────────────────────────────┐
                    │         Intel 8008 (Gate-Level)              │
                    │                                             │
    Memory ───────→ │  ┌──────────┐    ┌──────────────────────┐   │
    (program        │  │ Program  │    │   Instruction         │   │
     + data)        │  │ Counter  │───→│   Decoder             │   │
                    │  │ (14-bit) │    │   (gate trees)        │   │
                    │  └──────────┘    └────────┬─────────────┘   │
                    │       ↑                   │                 │
                    │       │            control signals          │
                    │       │                   │                 │
                    │  ┌────┴────┐       ┌──────┴──────┐          │
                    │  │ Stack   │       │  Control    │          │
                    │  │ 8×14-bit│       │  Unit (FSM) │          │
                    │  └─────────┘       └──────┬──────┘          │
                    │                           │                 │
                    │       ┌───────────────────┼──────┐          │
                    │       │                   │      │          │
                    │  ┌────┴──────┐  ┌─────────┴┐  ┌──┴──┐      │
                    │  │ Register  │  │  8-bit   │  │Flag │      │
                    │  │  File     │─→│  ALU     │──│Reg  │      │
                    │  │ 7×8-bit   │  │(gates!)  │  └─────┘      │
                    │  └───────────┘  └──────────┘               │
                    │       ↑              ↑                      │
                    │  ┌────┴────┐    ┌────┴────┐                 │
                    │  │ Accum   │    │ 16 KiB  │                 │
                    │  │ (8-bit) │    │ Memory  │                 │
                    │  └─────────┘    └─────────┘                 │
                    │                                             │
                    │  ┌──────────┐   ┌───────────────────┐       │
                    │  │ Input    │   │ Output Ports       │       │
                    │  │ Ports 0–7│   │ 0–23               │       │
                    │  └──────────┘   └───────────────────┘       │
                    └─────────────────────────────────────────────┘
```

## Components

### 1. bits.py — Bit Conversion Helpers

Converts between integer values and bit lists (LSB-first, matching the arithmetic
package's convention). This is the same module as in the 4004 gate-level simulator,
extended to support 14-bit values for the PC and stack.

```python
def int_to_bits(value: int, width: int) -> list[int]:
    """Convert integer to bit list (LSB first).
    
    E.g., int_to_bits(5, 8) → [1, 0, 1, 0, 0, 0, 0, 0]
    The arithmetic package's adders use LSB-first ordering.
    """

def bits_to_int(bits: list[int]) -> int:
    """Convert bit list (LSB first) to integer.
    
    E.g., bits_to_int([1, 0, 1, 0, 0, 0, 0, 0]) → 5
    """

def bits_parity(bits: list[int]) -> int:
    """Compute parity of a bit list via XOR reduction using logic gates.
    
    Returns 1 if even parity (even number of 1s), 0 if odd parity.
    Implemented as a chain of XOR gates:
    parity = XOR(bit0, XOR(bit1, XOR(bit2, ...)))
    then invert: P = NOT(xor_chain)
    (P=1 means even parity in 8008 convention)
    """
```

### 2. alu.py — 8-Bit ALU

Thin wrapper around `ALU(bit_width=8)` from the arithmetic package. The existing
ALU routes internally through gates. The 8008 ALU is double-wide compared to the
4004's 4-bit ALU — requiring 8 full-adders in the ripple-carry chain instead of 4.

```
ALU.execute(ADD, a_bits, b_bits):
  → ripple_carry_adder(a_bits, b_bits, cin=0)  [8 full-adders]
    → full_adder(a[0], b[0], 0)
      → half_adder(a[0], b[0])  → XOR(a, b), AND(a, b)
      → half_adder(sum, 0)      → XOR(sum, 0), AND(sum, 0)
      → OR(carry1, carry2)
    → full_adder(a[1], b[1], carry)
    → ...
    → full_adder(a[7], b[7], carry)   ← 4 more gates than 4004

8-bit ripple-carry: 8 × full_adder = 8 × 5 gates = 40 gates
vs 4-bit:           4 × full_adder = 4 × 5 gates = 20 gates
```

The 8008 also needs parity computation on the ALU output, which the 4004 lacked.
Parity is implemented as a 7-gate XOR reduction tree:

```
parity_bit = XOR(XOR(XOR(b0, b1), XOR(b2, b3)), XOR(XOR(b4, b5), XOR(b6, b7)))
```

```python
class Intel8008ALU:
    def __init__(self) -> None:
        self._alu = ALU(bit_width=8)

    def add(self, a: int, b: int, carry_in: bool) -> tuple[int, bool]:
        """Add two 8-bit values with carry. Returns (result, carry_out)."""

    def subtract(self, a: int, b: int, borrow_in: bool) -> tuple[int, bool]:
        """Subtract b from a with borrow. Returns (result, borrow_out).
        
        Implemented as a + ~b + 1 (two's complement via gates):
        1. NOT each bit of b (8 NOT gates)
        2. ripple_carry_adder(a, ~b, cin=1)
        CY=1 after SUB means no borrow was needed (unsigned a >= b).
        CY=0 means a borrow occurred.
        """

    def bitwise_and(self, a: int, b: int) -> int:
        """8-bit AND via 8 AND gates. Also clears carry."""

    def bitwise_or(self, a: int, b: int) -> int:
        """8-bit OR via 8 OR gates. Also clears carry."""

    def bitwise_xor(self, a: int, b: int) -> int:
        """8-bit XOR via 8 XOR gates. Also clears carry."""

    def compare(self, a: int, b: int) -> tuple[bool, bool, bool, bool]:
        """Compare a and b (a - b), return (carry, zero, sign, parity).
        
        Identical to subtract() but discards the numeric result.
        Uses the same full-adder chain internally.
        """

    def increment(self, a: int) -> tuple[int, bool]:
        """a + 1 via half-adder chain. Returns (result, carry_out)."""

    def decrement(self, a: int) -> tuple[int, bool]:
        """a - 1 via ~a + 0 trick: complement, add 0, complement back.
        More precisely: a - 1 = a + 0xFF via ripple_carry_adder.
        Returns (result, borrow_occurred).
        """

    def rotate_left_circular(self, a: int) -> tuple[int, bool]:
        """Rotate A left circular. CY ← A[7]; A[0] ← A[7].
        
        Implemented via bit shift in the gate representation:
        new_bits = [old_bits[7]] + old_bits[0:7]
        (no arithmetic gates needed — just rewiring)
        """

    def rotate_right_circular(self, a: int) -> tuple[int, bool]:
        """Rotate A right circular. CY ← A[0]; A[7] ← A[0]."""

    def rotate_left_carry(self, a: int, carry_in: bool) -> tuple[int, bool]:
        """9-bit rotate left through carry.
        
        [CY | A7 | A6 | ... | A0] rotate left by 1:
        new_A[0] ← old_CY
        new_CY  ← old_A[7]
        """

    def rotate_right_carry(self, a: int, carry_in: bool) -> tuple[int, bool]:
        """9-bit rotate right through carry."""

    def compute_flags(self, result: int, carry: bool) -> Intel8008FlagBits:
        """Compute all 4 flags from an 8-bit result.
        
        zero   = NOR(b7, b6, b5, b4, b3, b2, b1, b0)  [8-input NOR]
        sign   = b7  [direct wire]
        carry  = carry_out  [from adder]
        parity = NOT(XOR(XOR(XOR(b0,b1), XOR(b2,b3)), XOR(XOR(b4,b5), XOR(b6,b7))))
        """
```

### 3. registers.py — Register File (Sequential Logic)

7 × 8-bit registers (A, B, C, D, E, H, L) plus the 4-bit flag register, all built
from D flip-flops via the `register()` function from `logic_gates.sequential`.

The 8008 has twice as many data bits per register as the 4004 (8 vs 4), so the
register file requires twice the flip-flop count.

```python
class Intel8008RegisterFile:
    # Register indices (matching 3-bit hardware encoding):
    # 0=B, 1=C, 2=D, 3=E, 4=H, 5=L, 6=unused, 7=A

    def __init__(self) -> None:
        # 7 × 8-bit registers, each sequential.register(width=8)
        # = 7 × 8 = 56 D flip-flops
        # Plus 4-bit flag register: sequential.register(width=4)
        # = 4 D flip-flops
        # Total: 60 D flip-flops = 60 × 4 NOR gates = 240 NOR gates

    def read(self, reg_index: int) -> int:
        """Read 8-bit value from register 0–7 (6 is undefined, returns 0).
        
        If reg_index == 6 (M), raises ValueError — caller must resolve
        M to a memory address before calling read().
        """

    def write(self, reg_index: int, value: int) -> None:
        """Write 8-bit value to register. reg_index 6 (M) raises ValueError."""

    @property
    def a(self) -> int: ...       # Accumulator = register index 7
    @a.setter
    def a(self, value: int) -> None: ...

    @property
    def h(self) -> int: ...
    @property
    def l(self) -> int: ...

    @property
    def hl_address(self) -> int:
        """14-bit address formed from H and L.
        
        Implemented via bit extraction and OR/AND gates:
        address = (H[5:0] << 8) | L[7:0]
        = (H & 0x3F) << 8 | L
        """

    @property
    def flags(self) -> Intel8008FlagBits: ...
    @flags.setter
    def flags(self, value: Intel8008FlagBits) -> None: ...
```

Each register write traverses:
```
value → int_to_bits(value, 8) → register(bits, clock_edge, state, width=8)
  → 8 × D_flip_flop(bit, clock_edge)
    → 8 × SR_latch(D, NOT(D))
      → 8 × 2 NOR gates = 16 NOR gates per register
```

### 4. decoder.py — Instruction Decoder (Combinational Gates)

The instruction decoder takes the opcode byte (8 bits) and produces a set of control
signals that drive the rest of the CPU for one instruction cycle.

The decoder is pure combinational logic — no state, no clock. It is the "what do
I do with this instruction?" question answered entirely by AND/OR/NOT gate trees.

The 8008's decoder is more complex than the 4004's because:
- Instructions vary by 1, 2, or 3 bytes (the decoder must signal instruction length)
- The T/F sense bit for conditional branches adds a NOT gate
- The 3-bit operation field in ALU instructions creates an 8-way demultiplexer
- The parity computation is new (4004 had no parity flag)

```python
@dataclass
class DecoderOutput:
    """Control signals produced by the 8008 instruction decoder."""
    # --- Datapath signals ---
    alu_op: str             # "add", "adc", "sub", "sbb", "and", "or", "xor",
                            #  "cmp", "inr", "dcr", "rlc", "rrc", "ral", "rar"
    reg_src: int            # Source register (0–7, where 6=M, 7=A)
    reg_dst: int            # Destination register (0–7)
    use_immediate: bool     # True for MVI, ADI, SUI, etc.
    write_memory: bool      # True when M is destination (MOV M,r or MVI M,d)
    read_memory: bool       # True when M is source (MOV r,M or ALU M ops)
    
    # --- Register file signals ---
    write_acc: bool         # True when A is the destination
    write_reg: bool         # True when any register (not A) is written
    
    # --- Flag signals ---
    update_carry: bool      # True for ADD/SUB/ADC/SBB/rotates
    update_zero: bool       # True for most ALU ops
    update_sign: bool       # True for most ALU ops
    update_parity: bool     # True for most ALU ops
    clear_carry: bool       # True for AND/OR/XOR
    
    # --- Control flow ---
    is_jump: bool
    is_call: bool
    is_return: bool
    is_rst: bool            # Restart instruction (1-byte call to fixed address)
    condition_code: int     # 0=CY, 1=Z, 2=S, 3=P (for conditional branches)
    condition_sense: bool   # True = jump-if-true, False = jump-if-false
    jump_target_rst: int    # 0, 8, 16, ... 56 (for RST instructions)
    
    # --- I/O ---
    is_input: bool
    is_output: bool
    port_number: int        # IN port 0–7 or OUT port 0–23
    
    # --- Instruction properties ---
    is_halt: bool
    instruction_bytes: int  # 1, 2, or 3

def decode(opcode_bits: list[int]) -> DecoderOutput:
    """Decode 8 opcode bits into control signals using gate logic.

    Input: 8 bits (MSB first: bit7, bit6, ..., bit0)
    Output: control signals for this instruction cycle.

    The decoder is organized as a hierarchy of AND/OR/NOT gates
    that match the 8008's instruction encoding structure:

    Level 1 — Decode major group (bits 7–6):
      group_00 = AND(NOT(b7), NOT(b6))    → INR/DCR/MVI/Rotates
      group_01 = AND(NOT(b7), b6)         → MOV, HLT, JMP, CAL, JFc/JTc, IN
      group_10 = AND(b7, NOT(b6))         → ALU register ops
      group_11 = AND(b7, b6)              → ALU immediate, RET, RST, OUT

    Level 2 — Decode within each group (bits 5–0):
      For group_01: check if bits 2–0 are 110 (for MOV family)
        is_mov   = AND(group_01, b2, NOT(b1), b0) ... etc.
        is_halt  = AND(group_01, b5, b4, b3, b2, NOT(b1), b0)  ; 0x76
        is_jump  = AND(group_01, b2, NOT(b1), NOT(b0)) ; bits[2:0] = 100
        is_call  = AND(group_01, b2, b1, NOT(b0))      ; bits[2:0] = 110
      For group_10: decode 3-bit ALU op from bits 5–3
        is_add = AND(group_10, NOT(b5), NOT(b4), NOT(b3))
        is_adc = AND(group_10, NOT(b5), NOT(b4), b3)
        ...etc for all 8 ALU ops
    """
```

Example: detecting `ADD B` (opcode = `0x80` = `10 000 000`):

```
b7=1, b6=0 → group_10 = AND(1, NOT(0)) = AND(1,1) = 1    ✓ ALU reg group
b5=0, b4=0, b3=0 → is_add = AND(NOT(0),NOT(0),NOT(0)) = AND(1,1,1) = 1  ✓ ADD
b2=0, b1=0, b0=0 → reg_src = 0 (register B)              ✓ source = B
```

Example: detecting conditional jump `JFZ` (opcode = `0x48` = `01 001 000`):

```
b7=0, b6=1 → group_01 = 1                   ✓ control group
b2=0, b1=0, b0=0 → bits[2:0] = 000          ✓ jump (not call=110, not mov=xxx)
b4=0 → T=0 → condition_sense = False        ✓ jump-if-false
b5=0, b3=1 → CCC = 001 → condition = Zero  ✓ zero flag
→ "Jump if Zero is False" = JFZ             ✓
```

### 5. pc.py — Program Counter (14-Bit Register + Incrementer)

```python
class Intel8008PC:
    def __init__(self) -> None:
        # 14-bit register using sequential.register(width=14)
        # Incrementer: chain of 14 half-adders
        # (2 more than 4004's 12-bit PC)

    @property
    def value(self) -> int: ...            # Current PC (0–16383)

    def increment(self) -> None:
        """PC += 1 using 14 chained half-adders.

        half_adder(pc_bit0, 1) → sum0, carry0
        half_adder(pc_bit1, carry0) → sum1, carry1
        ...
        half_adder(pc_bit13, carry12) → sum13, carry13
        (carry13 is discarded — 14-bit wrap at 0x3FFF → 0x0000)
        """

    def increment_by(self, n: int) -> None:
        """PC += n (for 2-byte and 3-byte instructions).

        Implemented as n sequential increment() calls. This matches how
        the real chip incremented the PC one byte at a time during fetch.
        """

    def load(self, address: int) -> None:
        """PC = address (for jumps, calls, returns).

        Implemented via parallel load into the 14-bit register.
        address must be 0–16383 (14-bit range).
        """
```

### 6. stack.py — 8-Level Push-Down Stack

The 8008's stack differs architecturally from the 4004's: it holds 8 entries
(vs 3), and the current PC is always entry 0 (it is part of the stack hardware,
not a separate register). Push and pop are implemented by rotating the stack
register array.

```python
class Intel8008Stack:
    def __init__(self) -> None:
        # 8 × 14-bit registers, each sequential.register(width=14)
        # = 8 × 14 = 112 D flip-flops
        # = 112 × 4 NOR gates = 448 NOR gates
        # Plus a 3-bit stack pointer (0–7): sequential.register(width=3)

    def current_pc(self) -> int:
        """Return the current program counter (top of stack)."""

    def push(self, return_address: int) -> None:
        """Rotate stack down: old PC saved at entry 1, target at entry 0.

        Stack rotation via sequential register writes:
        entry[7] ← entry[6]
        entry[6] ← entry[5]
        ...
        entry[1] ← entry[0]  (saves return address = current PC + instr_len)
        entry[0] ← return_address (the call target)

        On overflow (8th push), entry[7] is silently overwritten.
        """

    def pop(self) -> int:
        """Rotate stack up: return address is now at entry 0.

        entry[0] ← entry[1]
        entry[1] ← entry[2]
        ...
        entry[6] ← entry[7]
        entry[7] ← 0 (or unchanged — behavior on underflow is undefined)

        Returns the new top of stack (the return address).
        """

    def load_pc(self, address: int) -> None:
        """Write directly to entry[0] (for JMP and RST, which don't push)."""

    @property
    def depth(self) -> int: ...    # 0–7: number of saved return addresses
```

### 7. memory.py — 16 KiB Byte-Addressable Memory

The 8008 has a unified memory space (no separate I/O address space — I/O uses
dedicated IN/OUT instructions). The memory model is simple: 16,384 bytes.

In the gate-level simulator, each byte in memory is modeled as an 8-bit register
built from D flip-flops. However, simulating 16,384 × 8-bit registers (131,072
flip-flops) would be impractically slow. We use the same practical compromise as
the ARM1 gate-level simulator: memory cells are modeled as Python bytearrays for
storage, but the address decoding logic (the multiplexer tree that selects a memory
cell from a 14-bit address) is implemented using gate functions.

```python
class Intel8008Memory:
    def __init__(self) -> None:
        # Physical storage: bytearray(16384)
        # Address decoder: combinational 14-to-16384 demultiplexer
        #   (implemented as a tree of AND/NOT gates on address bits)

    def read(self, address_bits: list[int]) -> list[int]:
        """Read 8 bits from the address specified by a 14-bit bit list.

        The address decoder uses AND gate trees to select the memory cell:
        For address 0x0010 = [0,0,0,0,1,0,0,0, 0,0,0,0,0,0] (LSB first):
          - Gate tree matches addr_bit4=1, all others 0
          - Returns the 8 bits at that cell
        """

    def write(self, address_bits: list[int], data_bits: list[int]) -> None:
        """Write 8 bits to the address specified by a 14-bit bit list."""

    def load_bytes(self, data: bytes, start: int = 0) -> None:
        """Load a byte sequence directly (bypasses gate model for initialization)."""
```

### 8. io.py — Input/Output Ports

The 8008 has 8 input ports (read by IN) and 24 output ports (written by OUT).
Port selection is encoded in the opcode's bit fields and decoded by gate logic.

```python
class Intel8008IO:
    def __init__(self) -> None:
        # 8 input port registers: each sequential.register(width=8)
        # 24 output port registers: each sequential.register(width=8)
        # Port select decoder: AND/NOT gate tree on port bits from opcode

    def read_input(self, port_bits: list[int]) -> list[int]:
        """Read 8 bits from input port selected by port_bits (3-bit).

        Port decoder:
          port_0 = AND(NOT(p2), NOT(p1), NOT(p0))
          port_1 = AND(NOT(p2), NOT(p1), p0)
          ...
          port_7 = AND(p2, p1, p0)
        MUX8: select one of 8 input registers based on decoded port.
        """

    def write_output(self, port_bits: list[int], data_bits: list[int]) -> None:
        """Write 8 bits to output port selected by port_bits (5-bit, ports 0–23)."""

    def set_input(self, port: int, value: int) -> None:
        """Set input port value (called by test harness / external simulation)."""

    def get_output(self, port: int) -> int:
        """Read current output port value."""
```

### 9. control.py — Control Unit (FSM)

The control unit sequences the fetch-decode-execute cycle. For the 8008, this
is more complex than the 4004 because the instruction length (1, 2, or 3 bytes)
must be determined during fetch to know how many additional bytes to read.

```
FSM states:
  FETCH1   → Read opcode byte from PC, increment PC
  DECODE   → Decode opcode, determine instruction length
  FETCH2   → (if 2 or 3 bytes) Read second byte, increment PC
  FETCH3   → (if 3 bytes) Read third byte, increment PC
  EXECUTE  → Apply decoded operation to registers/memory/PC/flags
```

```python
class Intel8008ControlUnit:
    def __init__(self, clock: Clock) -> None:
        # FSM using sequential logic (D flip-flop for state register)

    def step(self) -> ControlSignals:
        """Advance one full instruction (multiple clock cycles internally).

        The real 8008 uses a multi-phase clock cycle (T-states):
          - Short instruction: 5 T-states (T1–T5)
          - Long instruction: up to 11 T-states for 3-byte instructions
        We model this as a 5-state FSM that abstracts the T-state detail.
        """
```

The real 8008 used an 8-phase machine cycle with states T1–T5 plus WAIT, STOPPED,
and HALTED. Each machine cycle took either 1 or 2 cycles of the two-phase external
clock. We simplify to the logical FETCH/DECODE/FETCH2/FETCH3/EXECUTE model that
captures the essential sequencing without the electrical timing.

### 10. cpu.py — Top-Level Wiring

```python
class Intel8008GateLevel:
    def __init__(self) -> None:
        self._alu       = Intel8008ALU()
        self._registers = Intel8008RegisterFile()
        self._decoder   = decode           # Combinational decoder function
        self._pc        = Intel8008PC()    # Bundled into stack for 8008
        self._stack     = Intel8008Stack() # Stack owns the PC
        self._memory    = Intel8008Memory()
        self._io        = Intel8008IO()
        self._control   = Intel8008ControlUnit(Clock(frequency_hz=500_000))

    # --- Same public API as behavioral simulator ---
    def load_program(self, program: bytes, start_address: int = 0) -> None: ...
    def step(self) -> Intel8008Trace: ...
    def run(
        self,
        program: bytes,
        max_steps: int = 100_000,
        start_address: int = 0,
    ) -> list[Intel8008Trace]: ...
    def reset(self) -> None: ...

    @property
    def a(self) -> int: ...
    @property
    def b(self) -> int: ...
    @property
    def c(self) -> int: ...
    @property
    def d(self) -> int: ...
    @property
    def e(self) -> int: ...
    @property
    def h(self) -> int: ...
    @property
    def l(self) -> int: ...
    @property
    def pc(self) -> int: ...
    @property
    def flags(self) -> Intel8008Flags: ...

    # --- I/O (same as behavioral) ---
    def set_input_port(self, port: int, value: int) -> None: ...
    def get_output_port(self, port: int) -> int: ...

    # --- Gate-level specific ---
    def gate_count(self) -> dict[str, int]:
        """Count gate invocations by component.

        Returns a dict with keys: 'alu', 'registers', 'decoder', 'stack',
        'memory_decoder', 'io_decoder', 'pc', 'flags'. Values are gate
        call counts for the most recent instruction execution.
        """

    def gate_count_total(self) -> dict[str, int]:
        """Estimated static gate count (hardware gates, not invocations).

        This models how many AND/OR/NOT/XOR/NOT gates the chip would need
        if built in hardware. Compare with the real 8008's ~3,500 transistors.
        """

    def inspect_internals(self) -> dict:
        """Return raw bit-level internal state for debugging.

        Returns: {
            'a_bits': list[int],       # Accumulator bits (LSB first)
            'b_bits': list[int],       # Register B bits
            ...
            'flags_bits': list[int],   # [CY, Z, S, P]
            'pc_bits': list[int],      # 14-bit PC bits (LSB first)
            'stack_bits': list[list[int]],  # 8 × 14-bit stack entries
        }
        """
```

## Execution Flow (One ADD Instruction)

Here's what happens when the gate-level simulator executes `ADD B` (`0x80`):

```
1. FETCH1
   PC value (14 bits) → memory address decoder → read byte 0x80
   PC.increment() → 14 half-adders chain to compute PC+1

2. DECODE
   0x80 → 8 bits [0,0,0,0,0,0,0,1] (LSB first) = [1,0,0,0,0,0,0,0] (MSB first)
   → decoder gate tree:
     b7=1, b6=0 → group_10 = AND(b7, NOT(b6)) = 1   ✓ ALU register group
     b5=0,b4=0,b3=0 → is_add = AND(NOT(b5),NOT(b4),NOT(b3)) = 1  ✓ ADD
     b2=0,b1=0,b0=0 → reg_src = 0 (register B)      ✓
   → DecoderOutput(alu_op="add", reg_src=0, write_acc=True,
                   update_carry=True, update_zero=True,
                   update_sign=True, update_parity=True,
                   instruction_bytes=1)

3. EXECUTE
   Read B  → register_file.read(0) → e.g., 0x03 = [1, 1, 0, 0, 0, 0, 0, 0]
   Read A  → register_file.a       → e.g., 0x05 = [1, 0, 1, 0, 0, 0, 0, 0]
   Read CY → register_file.flags.carry → False

   ALU.add(0x05, 0x03, False):
     a_bits = [1,0,1,0, 0,0,0,0]   (A=5, LSB first)
     b_bits = [1,1,0,0, 0,0,0,0]   (B=3, LSB first)
     → ripple_carry_adder(a_bits, b_bits, cin=0):
       → full_adder(1, 1, 0) → sum=0, carry=1
       → full_adder(0, 1, 1) → sum=0, carry=1
       → full_adder(1, 0, 1) → sum=0, carry=1
       → full_adder(0, 0, 1) → sum=1, carry=0
       → full_adder(0, 0, 0) → sum=0, carry=0
       → full_adder(0, 0, 0) → sum=0, carry=0
       → full_adder(0, 0, 0) → sum=0, carry=0
       → full_adder(0, 0, 0) → sum=0, carry=0   [8 adders vs 4 for 4004]
     result_bits = [0,0,0,1, 0,0,0,0] → bits_to_int → 8
     carry_out = 0

   Compute flags from result 8 (0x08):
     zero   = NOR8([0,0,0,1,0,0,0,0]) = 0           (not all zero)
     sign   = result_bits[7] = 0                     (bit 7 = 0)
     carry  = carry_out = 0
     parity = XOR tree over 8 bits:
              XOR(XOR(XOR(0,0), XOR(0,1)), XOR(XOR(0,0), XOR(0,0)))
              = XOR(XOR(0,1), XOR(0,0)) = XOR(1,0) = 1
              parity = NOT(1) = 0                    (odd number of 1 bits)

   Write A = 8 → register_file.a = 8
     → 8 D flip-flops updated
   Write flags (CY=0, Z=0, S=0, P=0)
     → 4 D flip-flops updated
```

Total gate operations for one ADD: ~100+ individual gate function calls
(vs ~60 for the 4004's 4-bit ADD, as expected for double-width).

## Gate Count Estimates

| Component | Gates (approx) | Transistors (approx) | Notes |
|-----------|----------------|---------------------|-------|
| ALU (8-bit) | ~200 | ~500 | 8 full-adders + parity tree + NOT/AND/OR arrays |
| Register file (7×8-bit) | ~448 (flip-flops) | ~1,792 | 56 DFFs × 8 NOR gates each |
| Accumulator (8-bit) | ~32 | ~128 | 8 DFFs |
| Flag register (4-bit) | ~16 | ~64 | 4 DFFs |
| Instruction decoder | ~200 | ~500 | 8008 has more complex decoding than 4004 |
| Program counter (14-bit) | ~112 | ~280 | 14 DFFs + 14 half-adders |
| Stack (8×14-bit) | ~448 | ~1,120 | 112 DFFs |
| Control unit | ~50 | ~120 | FSM with more states than 4004 |
| Multiplexers/glue | ~60 | ~150 | Address decoding, bus selection |
| I/O decoders | ~40 | ~100 | Port select logic |
| **Total** | **~1,606** | **~4,754** | |

The real 8008 had approximately 3,500 transistors. Our model runs slightly high
because:
1. We model the full stack as individual flip-flops (the real chip likely used
   a more compact shift-register style implementation for the push-down stack)
2. Our CMOS-equivalent transistor count uses 4 per NOR gate, but the real 8008
   used PMOS with different gate structures

## Dependencies

```
intel8008-gatelevel
├── logic-gates (AND, OR, XOR, NOT, MUX, D flip-flop, register, SR latch)
├── arithmetic (half_adder, full_adder, ripple_carry_adder, ALU)
└── clock (Clock, ClockEdge)
```

## Test Strategy

### Unit Tests (per component)

- **ALU**:
  - add: (0,0), (0xFF, 0x01) → overflow, (0x7F, 0x01) → sign change
  - subtract: borrow semantics (CY=0 when borrow occurs)
  - AND/OR/XOR: verify CY cleared even when operands have CY preloaded
  - rotate: RLC, RRC, RAL, RAR with edge cases (0x80, 0x01, 0xFF)
  - compute_flags: zero detection, sign detection, parity for all 256 values,
    carry from adder

- **Registers**:
  - Read/write all 7 registers (B, C, D, E, H, L, A)
  - hl_address: verify 14-bit address formation for various H:L combinations
  - Flag register: write and read all flag combinations

- **Decoder**:
  - Verify control signals for every 8008 opcode (exhaustive — 256 inputs,
    many are undefined but decoder must not crash)
  - Spot-check: MOV A,B (0x78), ADD M (0x86), JMP (0x7C), CAL (0x7E),
    RET (0x3F), RST 4 (0x25), IN 3 (0x49+?), HLT (0x76)

- **Stack**:
  - Push/pop at depths 1, 2, 7
  - Verify depth counter is accurate
  - Wrap on 8th push: entry[7] overwritten, caller can detect via address mismatch

- **Memory**:
  - Read/write at address 0x0000, 0x0001, 0x1234, 0x3FFE, 0x3FFF
  - Address decoder: correct cell selected from gate logic
  - Verify gate path for address 0x0010 (spot check the AND gate tree)

- **PC**:
  - Increment from 0, from 0x3FFE, from 0x3FFF (wrap to 0x0000)
  - increment_by(1), increment_by(2), increment_by(3)
  - load() to arbitrary address

- **I/O**:
  - Read each input port 0–7
  - Write each output port 0–23
  - Port decoder gate tree spot-checks

### Integration Tests

Same programs as the behavioral simulator:
- x = 1 + 2
- 4 × 5 multiply via loop
- Absolute value subroutine
- Parity check

### Cross-Validation Tests

The ultimate correctness guarantee: run N programs on both behavioral and gate-level
simulators and assert identical final CPU state after every instruction.

```python
def test_cross_validate(program: bytes) -> None:
    behavioral = Intel8008Simulator()
    gatelevel  = Intel8008GateLevel()
    b_traces = behavioral.run(program, max_steps=1000)
    g_traces = gatelevel.run(program, max_steps=1000)
    assert len(b_traces) == len(g_traces)
    for b, g in zip(b_traces, g_traces):
        assert b.address == g.address
        assert b.a_after == g.a_after
        assert b.flags_after == g.flags_after
```

Programs used for cross-validation:
- All single-instruction tests (one per opcode)
- All example programs from 07f
- Hypothesis-generated random programs (property-based testing)

### Gate Count Tests

```python
def test_gate_count_reasonable() -> None:
    counts = Intel8008GateLevel().gate_count_total()
    # ALU should need more gates than 4004's ~80
    assert counts['alu'] > 80
    # Register file scales with 7×8 = 56 DFFs
    assert counts['registers'] >= 448
    # Total should be in the ballpark of real transistor count
    assert 1000 < sum(counts.values()) < 10000
```

### Performance Tests (Informational)

Measure the wall-clock slowdown factor of gate-level vs behavioral for the same
program. The 8008 gate-level simulator will be slower than the 4004 gate-level
simulator (more gates per instruction), but the ratio should be similar (~100–1000×).
Document this in the README as an educational data point.
