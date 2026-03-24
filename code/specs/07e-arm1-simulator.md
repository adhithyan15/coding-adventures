# 07e — ARM1 Simulator (ARMv1 Full Instruction Set)

## Overview

The ARM1 simulator implements the complete instruction set of the first ARM processor,
designed by Sophie Wilson and Steve Furber at Acorn Computers and first powered on
April 26, 1985. The ARM1 was a 32-bit RISC processor with just 25,000 transistors —
an order of magnitude simpler than the Intel 386 (275,000 transistors) released the
same year.

The ARM1 famously worked correctly on its very first power-on. Sophie Wilson typed
`PRINT PI` at the BBC Micro prompt and got the correct answer. The entire processor
had been simulated beforehand in 808 lines of BBC BASIC — making it one of the most
celebrated first-silicon successes in chip history.

This is a **behavioral simulator** — it executes ARMv1 machine code directly using
host-language arithmetic. For a gate-level simulation that routes every operation
through actual logic gates, see `07e2-arm1-gatelevel.md`.

## Layer Position

```
Logic Gates → Arithmetic → CPU → [YOU ARE HERE] → Assembler → ...
```

This is an alternative Layer 4 alongside RISC-V (07a), ARM/ARMv7 (07b), WASM (07c),
and Intel 4004 (07d). It sits between the generic CPU simulator and language-specific
tools.

## Why the ARM1?

- **Historical** — the chip that launched the ARM architecture, now in 250+ billion devices
- **Elegant simplicity** — 25,000 transistors, ~45 operations, 3-stage pipeline
- **Unique features** — conditional execution on every instruction, inline barrel shifter
- **RISC philosophy** — load/store architecture with no microcode
- **Contrast with 4004** — shows the jump from 4-bit accumulator (1971) to 32-bit
  register-register RISC (1985) in just 14 years
- **Accidental low power** — 0.1W vs 2W for the 386; simplicity-driven efficiency
  that later made ARM dominant in mobile

## Architecture

| Feature | Value |
|---------|-------|
| Data width | 32 bits |
| Instruction width | 32 bits (fixed-length) |
| ISA version | ARMv1 |
| Registers | 16 × 32-bit visible (R0–R15), 25 physical (banked modes) |
| Flags | N, Z, C, V (packed into R15 with PC) |
| Address space | 26-bit (64 MiB) |
| Program counter | 24 bits (bits 25:2 of R15, bottom 2 always 0) |
| Pipeline | 3-stage: Fetch → Decode → Execute |
| Multiply | **None** — added in ARMv2 (ARM2) |
| Cache | None |
| Memory | Von Neumann (unified instruction/data bus) |
| Clock | 6 MHz (original hardware) |
| Transistors | ~25,000 |
| Process | 3-micron CMOS (VLSI Technology) |
| Power | ~0.1 W |

### Register File

The ARM1 has 16 registers visible at any time, but 25 physical registers total
due to banked registers in different processor modes.

```
Register    Purpose                     Notes
────────    ───────                     ─────
R0–R12      General purpose             Fully orthogonal
R13         Stack Pointer (by convention) Architecturally general-purpose
R14         Link Register (LR)          BL stores return address here
R15         Program Counter + Status    See R15 layout below
```

#### R15 — The Combined PC/PSR Register

In ARMv1, R15 is unique: it packs the program counter AND the processor status
register into a single 32-bit register. This is a defining characteristic of the
original ARM design (later architectures split them into separate registers).

```
 31  30  29  28  27  26  25                          2   1   0
┌───┬───┬───┬───┬───┬───┬──────────────────────────────┬───┬───┐
│ N │ Z │ C │ V │ I │ F │   24-bit Program Counter     │M1 │M0 │
└───┴───┴───┴───┴───┴───┴──────────────────────────────┴───┴───┘

N  = Negative flag        (bit 31)
Z  = Zero flag            (bit 30)
C  = Carry flag           (bit 29)
V  = Overflow flag        (bit 28)
I  = IRQ disable          (bit 27)
F  = FIQ disable          (bit 26)
PC = Program counter      (bits 25:2) — 24 bits, word-aligned
M1 = Mode bit 1           (bit 1)
M0 = Mode bit 0           (bit 0)
```

Because instructions are 32 bits (4 bytes) and word-aligned, bits 1:0 of the
address are always 0. So those bits are repurposed for the processor mode.

The 3-stage pipeline means the PC is always **8 bytes ahead** of the currently
executing instruction: when an instruction at address A is executing, the PC
(as read from R15) contains A + 8.

#### Processor Modes

The ARM1 has 4 processor modes. Each mode has its own banked copies of R13 and
R14. FIQ additionally banks R8–R12 for fast interrupt handling without needing
to save registers.

```
M1  M0  Mode          Banked Registers
──  ──  ────          ────────────────
0   0   User (USR)    (base set)
0   1   FIQ           R8_fiq–R12_fiq, R13_fiq, R14_fiq  (7 banked)
1   0   IRQ           R13_irq, R14_irq                   (2 banked)
1   1   Supervisor    R13_svc, R14_svc                   (2 banked)
        (SVC)

Total physical registers: 16 base + 7 FIQ + 2 IRQ + 2 SVC = 25 banked + 2 shared = 25
```

### Condition Codes

Every ARM instruction has a 4-bit condition field in bits 31:28. This is ARM's
signature feature — any instruction can be made conditional, not just branches.

```
Code  Suffix  Meaning                      Flags Tested
────  ──────  ───────                      ────────────
0000  EQ      Equal                        Z=1
0001  NE      Not equal                    Z=0
0010  CS/HS   Carry set / unsigned ≥       C=1
0011  CC/LO   Carry clear / unsigned <     C=0
0100  MI      Minus (negative)             N=1
0101  PL      Plus (positive or zero)      N=0
0110  VS      Overflow set                 V=1
0111  VC      Overflow clear               V=0
1000  HI      Unsigned higher              C=1 and Z=0
1001  LS      Unsigned lower or same       C=0 or Z=1
1010  GE      Signed ≥                     N=V
1011  LT      Signed <                     N≠V
1100  GT      Signed >                     Z=0 and N=V
1101  LE      Signed ≤                     Z=1 or N≠V
1110  AL      Always (unconditional)       —
1111  NV      Never (reserved)             —
```

### Instruction Encoding

ARMv1 instructions are identified by bit patterns in bits 27:25 and other
discriminator bits. There are 6 instruction classes:

```
Bits 27:26  Bit 25  Class
──────────  ──────  ─────
00          —       Data Processing / PSR Transfer
01          —       Single Data Transfer (LDR/STR)
10          0       Block Data Transfer (LDM/STM)
10          1       Branch (B/BL)
11          0       Coprocessor Data Transfer
11          1,0     Coprocessor ops / SWI
```

## Complete Instruction Set

### 1. Data Processing Instructions

16 ALU operations selected by bits 24:21. All share this encoding:

```
 31  28 27 26 25 24  21 20 19  16 15  12 11           0
┌─────┬─────┬──┬──────┬──┬──────┬──────┬─────────────┐
│Cond │ 00  │I │Opcode│S │  Rn  │  Rd  │  Operand2   │
└─────┴─────┴──┴──────┴──┴──────┴──────┴─────────────┘

I (bit 25):  0 = Operand2 is a shifted register
             1 = Operand2 is a rotated immediate
S (bit 20):  1 = update condition flags (N, Z, C, V)
Rn:          first operand register
Rd:          destination register
```

| Opcode | Mnemonic | Operation | Category |
|--------|----------|-----------|----------|
| 0000 | AND | Rd = Rn AND Op2 | Logical |
| 0001 | EOR | Rd = Rn XOR Op2 | Logical |
| 0010 | SUB | Rd = Rn − Op2 | Arithmetic |
| 0011 | RSB | Rd = Op2 − Rn | Arithmetic |
| 0100 | ADD | Rd = Rn + Op2 | Arithmetic |
| 0101 | ADC | Rd = Rn + Op2 + C | Arithmetic |
| 0110 | SBC | Rd = Rn − Op2 − NOT(C) | Arithmetic |
| 0111 | RSC | Rd = Op2 − Rn − NOT(C) | Arithmetic |
| 1000 | TST | Rn AND Op2 → flags only | Test |
| 1001 | TEQ | Rn XOR Op2 → flags only | Test |
| 1010 | CMP | Rn − Op2 → flags only | Test |
| 1011 | CMN | Rn + Op2 → flags only | Test |
| 1100 | ORR | Rd = Rn OR Op2 | Logical |
| 1101 | MOV | Rd = Op2 | Move |
| 1110 | BIC | Rd = Rn AND NOT(Op2) | Logical |
| 1111 | MVN | Rd = NOT(Op2) | Move |

**Notes:**
- TST, TEQ, CMP, CMN are "test" instructions: they compute a result and set flags
  but do not write to Rd. The S bit is always implicitly set for these.
- For MOV and MVN, Rn is ignored (only Op2 matters).
- When S=1 and Rd=R15, the flags portion of R15 is updated from the ALU result's
  flag bits. This is used for returning from exceptions (restoring the saved PSR).

#### The Barrel Shifter (Operand2)

The second operand in data processing instructions passes through the barrel
shifter before reaching the ALU. This is one of ARM's most distinctive features —
a shift/rotate operation for free on every data processing instruction.

**Form 1: Immediate (I=1)**

```
 11    8  7       0
┌───────┬─────────┐
│Rotate │  Imm8   │
└───────┴─────────┘

value = Imm8 rotated right by (2 × Rotate)
```

The 8-bit immediate is rotated right by an even number of positions (0, 2, 4, ..., 30).
This allows encoding constants like 0xFF, 0xFF00, 0xFF000000, and many others that
would not fit in a plain 8-bit field.

**Form 2: Register (I=0)**

```
 11     7  6  5  4  3    0
┌───────┬───┬────┬──┬─────┐
│Shift  │ 0 │Type│0 │ Rm  │  Shift by immediate
│Amount │   │    │  │     │
└───────┴───┴────┴──┴─────┘

 11   8  7  6  5  4  3    0
┌──────┬──┬───┬────┬──┬─────┐
│  Rs  │0 │ 0 │Type│1 │ Rm  │  Shift by register
└──────┴──┴───┴────┴──┴─────┘
```

Shift types (bits 6:5):
```
Type  Mnemonic  Operation
────  ────────  ─────────
00    LSL       Logical Shift Left
01    LSR       Logical Shift Right
10    ASR       Arithmetic Shift Right (sign-extending)
11    ROR       Rotate Right
```

Special cases:
- LSL #0 = no shift (value unchanged)
- LSR #0 encodes LSR #32 (result is 0, carry = bit 31 of Rm)
- ASR #0 encodes ASR #32 (result is all 0s or all 1s depending on sign bit)
- ROR #0 encodes RRX (Rotate Right Extended): 33-bit rotate through carry flag

The barrel shifter also produces a carry output that updates the C flag when S=1,
for logical operations (AND, EOR, TST, TEQ, ORR, MOV, BIC, MVN). Arithmetic
operations (ADD, SUB, etc.) get their carry from the adder, not the shifter.

### 2. Single Data Transfer (LDR/STR)

```
 31  28 27 26 25 24 23 22 21 20 19  16 15  12 11           0
┌─────┬─────┬──┬──┬──┬──┬──┬──┬──────┬──────┬─────────────┐
│Cond │ 01  │I │P │U │B │W │L │  Rn  │  Rd  │   Offset    │
└─────┴─────┴──┴──┴──┴──┴──┴──┴──────┴──────┴─────────────┘

I:  0 = offset is 12-bit immediate, 1 = offset is shifted register
P:  1 = pre-indexed (add offset before transfer)
    0 = post-indexed (add offset after transfer)
U:  1 = add offset to base, 0 = subtract offset from base
B:  1 = byte transfer (zero-extended), 0 = word transfer
W:  1 = write back the calculated address to Rn
L:  1 = Load (LDR), 0 = Store (STR)
```

**Addressing modes:**
```
[Rn, #offset]      Pre-indexed, immediate offset (P=1, I=0)
[Rn, Rm]           Pre-indexed, register offset (P=1, I=1)
[Rn, Rm, shift]    Pre-indexed, shifted register offset (P=1, I=1)
[Rn, #offset]!     Pre-indexed with writeback (P=1, W=1)
[Rn], #offset      Post-indexed (P=0, always writes back)
```

**Word alignment:** LDR of an unaligned address rotates the loaded word so the
addressed byte is in the least significant position. This is a quirk of the ARM1.

### 3. Block Data Transfer (LDM/STM)

```
 31  28 27 26 25 24 23 22 21 20 19  16 15                 0
┌─────┬─────┬──┬──┬──┬──┬──┬──┬──────┬──────────────────────┐
│Cond │ 10  │0 │P │U │S │W │L │  Rn  │   Register List      │
└─────┴─────┴──┴──┴──┴──┴──┴──┴──────┴──────────────────────┘

P:  Pre/post indexing
U:  Up/down (increment/decrement)
S:  1 = load PSR or force user-mode registers
W:  1 = write back final address to Rn
L:  1 = Load (LDM), 0 = Store (STM)
Register List: 16-bit bitmap, bit N = 1 means transfer register N
```

The four stacking modes:

```
P  U  Name        ARM Mnemonic    Stack usage
─  ─  ────        ────────────    ───────────
0  1  Increment After   IA/FD    Full Descending (standard)
1  1  Increment Before  IB/ED    Empty Descending
0  0  Decrement After   DA/EA    Full Ascending
1  0  Decrement Before  DB/FA    Empty Ascending
```

Registers are always transferred in order from lowest to highest number,
regardless of the direction. The lowest-numbered register goes to the
lowest memory address.

### 4. Branch Instructions (B/BL)

```
 31  28 27 26 25 24 23                                     0
┌─────┬─────┬──┬──┬────────────────────────────────────────┐
│Cond │ 10  │1 │L │         24-bit signed offset            │
└─────┴─────┴──┴──┴────────────────────────────────────────┘

L:  0 = Branch (B), 1 = Branch with Link (BL)
```

The offset is sign-extended to 32 bits and shifted left by 2 (since instructions
are word-aligned), giving a range of ±32 MiB from the current PC.

For BL, the address of the instruction after the branch is saved in R14 (LR)
before jumping. The saved value includes the PSR flags (since R15 = PC + flags
in ARMv1).

### 5. Software Interrupt (SWI)

```
 31  28 27  24 23                                          0
┌─────┬───────┬────────────────────────────────────────────┐
│Cond │ 1111  │        24-bit comment field                 │
└─────┴───────┴────────────────────────────────────────────┘
```

SWI enters Supervisor mode:
1. Save PC (with flags) in R14_svc
2. Set mode bits to SVC (M1:M0 = 11)
3. Set I bit (disable IRQs)
4. Jump to address 0x08

The 24-bit comment field is ignored by the processor but used by the OS to
identify which system call is being requested.

### 6. Coprocessor Instructions

The ARM1 encoding includes coprocessor data operations, register transfers
(MRC/MCR), and data transfers (LDC/STC). Since the ARM1 has no coprocessor,
these generate undefined instruction traps. We implement the trap mechanism
but not any actual coprocessor.

### Exception Vectors

```
Address   Exception              Priority
───────   ─────────              ────────
0x00      Reset                  1 (highest)
0x04      Undefined Instruction  6
0x08      Software Interrupt     6
0x0C      Prefetch Abort         5
0x10      Data Abort             2
0x14      Address Exception      2
0x18      IRQ                    4
0x1C      FIQ                    3
```

The simulator implements Reset, Undefined Instruction, SWI, and Address
Exception (triggered when a 26-bit address overflows). IRQ/FIQ are available
for external code to trigger but are not generated internally.

## Public API

```python
class ARM1Simulator:
    def __init__(self, memory_size: int = 64 * 1024 * 1024) -> None:
        """Create an ARM1 simulator.

        Args:
            memory_size: Size of memory in bytes. Default 64 MiB (full 26-bit
                         address space). Must be a multiple of 4.
        """

    # --- Register Access ---

    def read_register(self, index: int) -> int:
        """Read register R0–R15. Returns the value for the current processor mode
        (i.e., reads banked registers in FIQ/IRQ/SVC modes).

        For R15: returns the full 32-bit value (PC + flags + mode bits).
        """

    def write_register(self, index: int, value: int) -> None:
        """Write register R0–R15 in the current processor mode."""

    @property
    def pc(self) -> int:
        """Current program counter (26-bit address, word-aligned).
        Extracted from bits 25:2 of R15, shifted left by 2.
        """

    @property
    def cpsr(self) -> int:
        """Full R15 value (PC + status flags + mode)."""

    @property
    def flags(self) -> ARM1Flags:
        """Current condition flags (N, Z, C, V)."""

    @property
    def mode(self) -> ProcessorMode:
        """Current processor mode (USR, FIQ, IRQ, SVC)."""

    # --- Memory Access ---

    @property
    def memory(self) -> bytearray:
        """Raw memory access (byte-addressable)."""

    def read_word(self, address: int) -> int:
        """Read 32-bit word from memory (little-endian).
        Address must be word-aligned (multiple of 4).
        """

    def write_word(self, address: int, value: int) -> None:
        """Write 32-bit word to memory (little-endian)."""

    def read_byte(self, address: int) -> int:
        """Read single byte from memory."""

    def write_byte(self, address: int, value: int) -> None:
        """Write single byte to memory."""

    # --- Execution ---

    def load_program(self, machine_code: bytes, start_address: int = 0) -> None:
        """Load machine code into memory at the given address."""

    def step(self) -> ARM1Trace:
        """Execute one instruction. Returns trace of what happened."""

    def run(self, max_steps: int = 100_000) -> list[ARM1Trace]:
        """Run until HLT or max_steps reached. Returns full execution trace."""

    def reset(self) -> None:
        """Reset CPU to power-on state: SVC mode, PC=0, IRQs disabled."""

    # --- Interrupts ---

    def raise_irq(self) -> None:
        """Raise an IRQ interrupt (if not masked by I flag)."""

    def raise_fiq(self) -> None:
        """Raise an FIQ interrupt (if not masked by F flag)."""


class ProcessorMode(Enum):
    USR = 0  # User mode
    FIQ = 1  # Fast interrupt
    IRQ = 2  # Normal interrupt
    SVC = 3  # Supervisor


@dataclass
class ARM1Flags:
    n: bool  # Negative
    z: bool  # Zero
    c: bool  # Carry
    v: bool  # Overflow


@dataclass
class ARM1Trace:
    address: int              # PC where instruction was fetched
    raw: int                  # The 32-bit instruction word
    mnemonic: str             # Disassembled form ("ADDS R0, R1, R2, LSL #3")
    condition: str            # Condition code ("AL", "EQ", "NE", etc.)
    condition_passed: bool    # Did the condition check pass?
    registers_before: list[int]   # R0–R15 snapshot before execution
    registers_after: list[int]    # R0–R15 snapshot after execution
    flags_before: ARM1Flags
    flags_after: ARM1Flags
    memory_reads: list[tuple[int, int]]   # [(address, value), ...]
    memory_writes: list[tuple[int, int]]  # [(address, value), ...]
```

## Instruction Timing (Informational)

The ARM1 has a 3-stage pipeline. Instruction timings in clock cycles:

```
Instruction          Cycles  Notes
───────────          ──────  ─────
Data processing      1       (register shift by immediate)
Data processing      2       (register shift by register — extra decode cycle)
LDR                  3       (address + memory + writeback)
STR                  2       (address + memory)
LDM                  N+1     (N = number of registers transferred)
STM                  N       (N = number of registers transferred)
B / BL               3       (pipeline flush + refill)
SWI                  3       (pipeline flush + refill)
```

The simulator does not model cycle-accurate timing but records the cycle count
in the trace for educational purposes.

## Example Programs

### x = 1 + 2 (Basic Arithmetic)

```asm
; R0 = 1, R1 = 2, R2 = R0 + R1 = 3
MOV R0, #1          ; E3A00001  — Move immediate 1 into R0
MOV R1, #2          ; E3A01002  — Move immediate 2 into R1
ADD R2, R0, R1      ; E0802001  — R2 = R0 + R1
HLT                 ; simulator pseudo-instruction (SWI 0x123456)
```

We use `SWI #0x123456` as a halt instruction (the simulator intercepts this
specific SWI number to stop execution, similar to HLT in the 4004 simulator).

### Conditional Execution

```asm
; Compute abs(R0): if R0 < 0, negate it
CMP  R0, #0         ; E3500000  — Compare R0 with 0 (sets flags)
RSBLT R0, R0, #0    ; B2600000  — If Less Than: R0 = 0 - R0
```

Two instructions, no branch. This is the power of ARM's conditional execution.

### Barrel Shifter — Multiply by 5

```asm
; R1 = R0 * 5 (without MUL — ARM1 has no multiply instruction!)
; 5x = 4x + x = (x << 2) + x
ADD R1, R0, R0, LSL #2   ; E0801100  — R1 = R0 + (R0 << 2)
```

One instruction. The barrel shifter shifts R0 left by 2 (multiply by 4) and
the ADD combines it with the unshifted R0.

### Loop: Sum 1 to 10

```asm
; R0 = sum, R1 = counter (10 down to 0)
        MOV R0, #0        ; sum = 0
        MOV R1, #10       ; counter = 10
loop:   ADD R0, R0, R1    ; sum += counter
        SUBS R1, R1, #1   ; counter-- (S = update flags)
        BNE loop          ; if counter != 0, branch to loop
        ; R0 now contains 55
```

### Subroutine Call (BL / MOV PC, LR)

```asm
        MOV R0, #7        ; argument
        BL  double        ; call subroutine, R14 = return address
        ; R0 now contains 14
        SWI 0x123456      ; halt

double: ADD R0, R0, R0    ; R0 = R0 + R0
        MOVS PC, R14      ; return (restore PC and flags from LR)
```

Note: `MOVS PC, R14` restores both the PC and the flags from R14 because in
ARMv1, R14 contains the full R15 value (PC + flags) saved by BL.

### Block Data Transfer (Stack Operations)

```asm
; Save R0–R3 to stack, call subroutine, restore
STMFD R13!, {R0-R3, R14}   ; Push R0–R3 and return address
BL    some_function
LDMFD R13!, {R0-R3, PC}    ; Pop and return (loading PC returns)
```

## Dependencies

```
arm1-simulator
├── (no internal dependencies — self-contained behavioral simulator)
```

Unlike the 4004 simulator, the ARM1 behavioral simulator does NOT depend on
`virtual-machine`/GenericVM. The ARM1's instruction encoding is complex enough
(conditional execution, barrel shifter, multiple addressing modes) that a
generic opcode dispatch table would not simplify the implementation. Instead,
the simulator has its own fetch-decode-execute loop.

## Implementation Structure

```
arm1-simulator/
├── cpu.{ext}           Top-level ARM1Simulator class
├── decoder.{ext}       Instruction decoding (bit extraction)
├── barrel_shifter.{ext} Operand2 processing (shifts, rotates, immediates)
├── alu.{ext}           32-bit ALU (16 operations + flag computation)
├── registers.{ext}     Register file with banked mode registers
├── memory.{ext}        Byte-addressable memory with word/byte access
├── conditions.{ext}    Condition code evaluation (16 conditions)
└── types.{ext}         ProcessorMode, ARM1Flags, ARM1Trace, etc.
```

## Test Strategy

### Individual Instruction Tests

**Data Processing (16 × extensive):**
- Every opcode with immediate and register operands
- S bit: verify flags update correctly (or not)
- Barrel shifter: LSL, LSR, ASR, ROR by immediate and register
- RRX (rotate right extended through carry)
- Edge cases: Rd=R15 (PC update), shifts by 0, shifts by 32

**Load/Store:**
- LDR/STR with immediate offset
- LDR/STR with register offset (shifted and unshifted)
- Pre-indexed, post-indexed, writeback
- Byte access (LDRB/STRB)
- Unaligned word loads (rotation behavior)
- Address exception on out-of-range address

**Block Transfer:**
- LDM/STM all four modes (IA, IB, DA, DB)
- Various register list combinations
- Writeback
- Loading PC (return from subroutine)
- Empty register list (architecturally undefined — test our choice)

**Branch:**
- Forward and backward branches
- BL: verify R14 saved correctly (with flags)
- Conditional branches: all 15 condition codes
- Branch to self (infinite loop detection)

**SWI:**
- Mode switch to SVC
- R14_svc saved correctly
- Return via `MOVS PC, R14`

### Condition Code Tests
- All 15 conditions (EQ through AL) with appropriate flag combinations
- NV condition (reserved — verify it never executes)
- Conditional data processing, load/store, and branches

### Barrel Shifter Tests
- Immediate: all 16 rotation positions
- Register shifts: LSL/LSR/ASR/ROR by 0, 1, 15, 16, 31, 32
- Carry output from shifts (for S-bit updates)
- RRX with carry=0 and carry=1

### Processor Mode Tests
- Mode switching (USR → SVC via SWI, SVC → USR)
- Banked registers: write R13 in USR, switch to SVC, verify R13_svc is different
- FIQ banked registers R8–R12

### End-to-End Programs
- x = 1 + 2
- abs(x) via conditional RSB
- Multiply by constant via barrel shifter
- Sum 1 to N via loop
- Subroutine call and return
- Nested subroutine calls
- Stack push/pop via LDM/STM
- Fibonacci sequence

### Cross-Language Consistency
- Same programs must produce identical traces across all 6 language implementations
- Register state, memory state, and flag state must match exactly after each step
