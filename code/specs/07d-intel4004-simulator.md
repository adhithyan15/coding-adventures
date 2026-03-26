# 07d — Intel 4004 Simulator (Full Instruction Set)

## Overview

The Intel 4004 simulator implements the complete instruction set of the world's first
commercial microprocessor, released by Intel in November 1971. It was a 4-bit CPU
designed by Federico Faggin, Ted Hoff, and Stanley Mazor for the Busicom 141-PF
calculator. Intel negotiated to retain the chip design rights — one of the most
consequential business decisions in computing history.

This is a **behavioral simulator** — it executes 4004 machine code directly, producing
correct results without modeling internal hardware. For a gate-level simulation that
routes every operation through actual logic gates, see `07d2-intel4004-gatelevel.md`.

The simulator uses **GenericVM** from the `virtual-machine` package as its execution
engine. Each 4004 opcode is registered as a handler, giving us the dispatch loop,
tracing, and step/run infrastructure for free.

## Layer Position

```
Logic Gates → Arithmetic → CPU → [YOU ARE HERE] → Assembler → Lexer → Parser → Compiler → VM
```

This is an alternative Layer 4 alongside RISC-V, ARM, and WASM.

## Why the Intel 4004?

- **Historical** — the chip that started the microprocessor revolution
- **Tiny** — 4-bit data, 46 instructions, 16 registers (4-bit each), 2,300 transistors
- **Real hardware constraints** — forces you to think about how `1 + 2` works with only 4 bits
- **BCD arithmetic** — the 4004 was built for decimal calculators, not general computing
- **Contrast** — shows how far we've come from 1971 (4004) to 2017 (WASM)

## Architecture

| Feature | Value |
|---------|-------|
| Data width | 4 bits (values 0–15) |
| Instruction width | 8 bits (some instructions are 2 bytes) |
| Registers | 16 × 4-bit (R0–R15), organized as 8 pairs (P0–P7) |
| Accumulator | 4-bit (A) — most arithmetic goes through here |
| Carry flag | 1 bit — set on overflow/borrow |
| Program counter | 12 bits (addresses 4096 bytes of ROM) |
| Stack | 3-level hardware stack (12-bit return addresses) |
| ROM | 4096 × 8-bit (program storage) |
| RAM | 4 banks × 4 registers × (16 main + 4 status) nibbles = 320 nibbles |
| Clock | 740 kHz (original hardware) |

### Register Pairs

The 16 registers are organized as 8 pairs for some instructions:

```
Pair P0: R0  (high nibble), R1  (low nibble)
Pair P1: R2  (high nibble), R3  (low nibble)
Pair P2: R4  (high nibble), R5  (low nibble)
Pair P3: R6  (high nibble), R7  (low nibble)
Pair P4: R8  (high nibble), R9  (low nibble)
Pair P5: R10 (high nibble), R11 (low nibble)
Pair P6: R12 (high nibble), R13 (low nibble)
Pair P7: R14 (high nibble), R15 (low nibble)
```

A register pair holds an 8-bit value: `pair_value = (R_high << 4) | R_low`.
Instructions like FIM load 8 bits into a pair. SRC sends a pair as an address.

### RAM Organization

The 4004's RAM is unlike modern flat memory. It is organized hierarchically:

```
RAM
├── Bank 0 (selected by DCL)
│   ├── Register 0
│   │   ├── Main characters: 16 nibbles (addressed by SRC low nibble)
│   │   └── Status characters: 4 nibbles (accessed by WR0–WR3 / RD0–RD3)
│   ├── Register 1 ... (same structure)
│   ├── Register 2
│   └── Register 3
├── Bank 1 ... (same structure)
├── Bank 2
└── Bank 3
```

Total: 4 banks × 4 registers × 20 nibbles = 320 nibbles (160 bytes equivalent).

To access RAM:
1. Load an address into a register pair (FIM)
2. Send it with SRC (sets the "current" register and character)
3. Read/write with RDM/WRM (main) or RD0–RD3/WR0–WR3 (status)

### 3-Level Hardware Stack

The 4004 has a 3-deep hardware stack for subroutine calls. It is NOT in RAM — it is
built from dedicated registers inside the chip. There is no stack pointer register
visible to the programmer.

- JMS (Jump to Subroutine): pushes PC+2 onto the stack, jumps to target
- BBL (Branch Back and Load): pops the stack, jumps to the saved address
- Stack wraps silently on overflow (4th push overwrites the oldest entry)

### Accumulator Architecture

The 4004 uses an accumulator architecture. Almost every arithmetic operation works
through the Accumulator (A):

```
RISC-V (register-register):  add x3, x1, x2     Any register to any register.
WASM (stack-based):           i32.add              Pops two, pushes result.
Intel 4004 (accumulator):     ADD R0               A = A + R0. Always uses A.
```

## Complete Instruction Set (46 Instructions)

### Encoding Format

Most instructions are 1 byte. Five instructions are 2 bytes (marked with *).
The upper nibble encodes the opcode, the lower nibble encodes the operand
(register number, register pair, condition, or immediate value).

For 2-byte instructions, the second byte is the data or address low byte.

### Opcode Map

```
0x00       NOP             No operation
0x01       HLT             Halt (simulator-only, not real 4004)
0x1C_AAAA  JCN C,A    *    Jump conditional (C=condition, A=12-bit addr)
0x2R_DDDD  FIM Rp,D   *    Fetch immediate to register pair (D=8-bit data)
0x2R+1     SRC Rp          Send register control (pair as address)
0x3R       FIN Rp          Fetch indirect from ROM via pair P0
0x3R+1     JIN Rp          Jump indirect via register pair
0x4A_AAAA  JUN A      *    Jump unconditional (A=12-bit address)
0x5A_AAAA  JMS A      *    Jump to subroutine (A=12-bit address)
0x6R       INC Rn          Increment register
0x7R_AAAA  ISZ Rn,A   *    Increment and skip if zero (A=8-bit addr)
0x8R       ADD Rn          Add register to accumulator with carry
0x9R       SUB Rn          Subtract register from accumulator with borrow
0xAR       LD Rn           Load register into accumulator
0xBR       XCH Rn          Exchange accumulator and register
0xCN       BBL N           Branch back and load (N=immediate into A)
0xDN       LDM N           Load immediate into accumulator
0xE0       WRM             Write accumulator to RAM main character
0xE1       WMP             Write accumulator to RAM output port
0xE2       WRR             Write accumulator to ROM I/O port
0xE3       WPM             Write accumulator to program RAM (not simulated)
0xE4       WR0             Write accumulator to RAM status character 0
0xE5       WR1             Write accumulator to RAM status character 1
0xE6       WR2             Write accumulator to RAM status character 2
0xE7       WR3             Write accumulator to RAM status character 3
0xE8       SBM             Subtract RAM main character from accumulator
0xE9       RDM             Read RAM main character into accumulator
0xEA       RDR             Read ROM I/O port into accumulator
0xEB       ADM             Add RAM main character to accumulator
0xEC       RD0             Read RAM status character 0 into accumulator
0xED       RD1             Read RAM status character 1 into accumulator
0xEE       RD2             Read RAM status character 2 into accumulator
0xEF       RD3             Read RAM status character 3 into accumulator
0xF0       CLB             Clear both (A=0, carry=0)
0xF1       CLC             Clear carry
0xF2       IAC             Increment accumulator
0xF3       CMC             Complement carry
0xF4       CMA             Complement accumulator
0xF5       RAL             Rotate accumulator left through carry
0xF6       RAR             Rotate accumulator right through carry
0xF7       TCC             Transfer carry to accumulator, clear carry
0xF8       DAC             Decrement accumulator
0xF9       TCS             Transfer carry subtract (A=9+carry, carry=0)
0xFA       STC             Set carry
0xFB       DAA             Decimal adjust accumulator (BCD correction)
0xFC       KBP             Keyboard process (1-hot to binary)
0xFD       DCL             Designate command line (select RAM bank)
```

### Instruction Details

#### Machine Control
- **NOP (0x00):** Do nothing. PC advances by 1.
- **HLT (0x01):** Halt execution. Not a real 4004 opcode — added for the simulator.

#### Immediate Load
- **LDM N (0xDN):** Load 4-bit immediate N into accumulator. A = N.

#### Register Operations
- **LD Rn (0xAR):** Load register Rn into accumulator. A = Rn. (Non-destructive read.)
- **XCH Rn (0xBR):** Exchange accumulator and register. Swap A and Rn.
- **INC Rn (0x6R):** Increment register. Rn = (Rn + 1) & 0xF. No carry flag affected.

#### Arithmetic (Register)
- **ADD Rn (0x8R):** A = A + Rn + carry. Carry set if result > 15.
- **SUB Rn (0x9R):** A = A + ~Rn + (1 - carry). Carry CLEARED if borrow needed.
  (The 4004 uses complement-add for subtraction. Carry=1 means no borrow.)

#### Arithmetic (RAM)
- **ADM (0xEB):** A = A + RAM[current] + carry. Carry set if result > 15.
- **SBM (0xE8):** A = A + ~RAM[current] + (1 - carry). Same borrow logic as SUB.

#### Accumulator Operations
- **CLB (0xF0):** Clear both. A = 0, carry = 0.
- **CLC (0xF1):** Clear carry. carry = 0.
- **IAC (0xF2):** Increment accumulator. A = (A + 1) & 0xF. Carry set if A was 15.
- **CMC (0xF3):** Complement carry. carry = !carry.
- **CMA (0xF4):** Complement accumulator. A = ~A & 0xF (bitwise NOT, 4-bit).
- **RAL (0xF5):** Rotate left through carry. `[carry|A3|A2|A1|A0]` shifts left:
  old carry → A0, A3 → new carry.
- **RAR (0xF6):** Rotate right through carry. `[carry|A3|A2|A1|A0]` shifts right:
  old carry → A3, A0 → new carry.
- **TCC (0xF7):** Transfer carry to accumulator. A = 1 if carry else 0. carry = 0.
- **DAC (0xF8):** Decrement accumulator. A = (A - 1) & 0xF. Carry CLEARED if borrow
  (i.e., carry=0 when A was 0, carry=1 otherwise).
- **TCS (0xF9):** Transfer carry subtract. A = 10 if carry else 9. carry = 0.
  (Used in BCD subtraction: provides the complement correction factor.)
- **STC (0xFA):** Set carry. carry = 1.
- **DAA (0xFB):** Decimal adjust accumulator. If A > 9 or carry is set, add 6 to A.
  If the addition causes overflow, set carry. (BCD correction after addition.)
- **KBP (0xFC):** Keyboard process. Converts 1-hot encoding to binary:
  0→0, 1→1, 2→2, 4→3, 8→4, else→15 (error).
- **DCL (0xFD):** Designate command line. Selects RAM bank based on A bits 0–2.

#### Jump Instructions
- **JUN addr (0x4H 0xLL):** Unconditional jump. PC = (H << 8) | LL (12-bit address).
  The upper nibble of the first byte provides bits 11–8 of the address.
- **JCN cond,addr (0x1C 0xAA):** Conditional jump. Condition C is 4 bits:
  - Bit 3 (invert): if set, invert the test result
  - Bit 2 (accumulator): test if A == 0
  - Bit 1 (carry): test if carry == 1
  - Bit 0 (test pin): test input pin (always 0 in simulator)
  Tests are OR'd together. If condition met, PC = (PC & 0xF00) | AA (same page jump).
- **ISZ Rn,addr (0x7R 0xAA):** Increment register Rn. If Rn != 0 after increment,
  jump to (PC & 0xF00) | AA. Otherwise continue. (Loop counter instruction.)

#### Subroutine Instructions
- **JMS addr (0x5H 0xLL):** Push PC+2 onto hardware stack, jump to 12-bit address.
- **BBL N (0xCN):** Pop address from hardware stack, set A = N, jump to popped address.
  (Return from subroutine with a return value in the accumulator.)

#### Register Pair Instructions
- **FIM Rp,data (0x2P 0xDD):** Load 8-bit immediate DD into register pair Pp.
  R_high = (DD >> 4) & 0xF, R_low = DD & 0xF. P is even (0,2,4,...,14).
- **SRC Rp (0x2P+1):** Send register pair Pp as address for RAM/ROM operations.
  Sets the current RAM register and character for subsequent WRM/RDM/etc.
  High nibble selects RAM register (0–3), low nibble selects character (0–15).
- **FIN Rp (0x3P):** Fetch indirect. Read ROM at address in P0 (pair 0 = R0:R1),
  store the byte into register pair Pp. R_high = ROM_byte >> 4, R_low = ROM_byte & 0xF.
- **JIN Rp (0x3P+1):** Jump indirect. PC = (PC & 0xF00) | (R_high << 4) | R_low.
  Same-page jump to address in register pair.

#### RAM/ROM I/O
- **WRM (0xE0):** Write A to RAM main character at current address (set by SRC).
- **RDM (0xE9):** Read RAM main character at current address into A.
- **WR0–WR3 (0xE4–0xE7):** Write A to RAM status character 0–3.
- **RD0–RD3 (0xEC–0xEF):** Read RAM status character 0–3 into A.
- **WMP (0xE1):** Write A to RAM output port (stored but not externally connected).
- **WRR (0xE2):** Write A to ROM I/O port.
- **RDR (0xEA):** Read ROM I/O port into A.
- **WPM (0xE3):** Write program RAM. Not simulated (was for EPROM programming).

## Execution Engine: GenericVM Integration

The simulator uses `GenericVM` from the `virtual-machine` package. This provides:

1. **Opcode dispatch** — each 4004 opcode is registered as a handler
2. **Fetch-decode-execute loop** — GenericVM's `execute()` drives the main loop
3. **Step/run API** — single-step debugging comes free
4. **Tracing** — every instruction execution produces a `VMTrace` snapshot

### Loader

Raw ROM bytes are pre-parsed into `Instruction` objects:
- 1-byte instructions: `Instruction(opcode=raw_byte, operand=None)`
- 2-byte instructions: `Instruction(opcode=first_byte, operand=second_byte)`
- The loader detects 2-byte instructions by checking the upper nibble

The resulting `CodeObject` is passed to `GenericVM.execute()`.

### State

4004-specific state is stored as attributes on the `Intel4004Simulator` instance:
- `accumulator`, `registers`, `carry` — CPU registers
- `hw_stack`, `stack_pointer` — 3-level hardware call stack
- `ram`, `ram_bank`, `ram_address` — RAM with bank selection
- `rom_port` — I/O port for WRR/RDR
- `output_port` — RAM output port for WMP

The `GenericVM` stack is unused (4004 is accumulator-based, not stack-based).

## Public API

```python
class Intel4004Simulator:
    def __init__(self) -> None: ...

    # --- CPU State ---
    @property
    def accumulator(self) -> int: ...       # 0–15
    @property
    def registers(self) -> list[int]: ...   # 16 values, each 0–15
    @property
    def carry(self) -> bool: ...
    @property
    def pc(self) -> int: ...                # 0–4095

    # --- Memory ---
    @property
    def ram(self) -> list[list[list[int]]]: ...  # banks × registers × nibbles
    @property
    def rom(self) -> bytearray: ...              # 4096 bytes

    # --- Execution ---
    def load_program(self, rom: bytes, start_address: int = 0) -> None: ...
    def step(self) -> Intel4004Trace: ...
    def run(self, program: bytes, max_steps: int = 10000) -> list[Intel4004Trace]: ...
    def reset(self) -> None: ...

@dataclass
class Intel4004Trace:
    address: int                # PC where this instruction was fetched
    raw: int                    # Raw first byte
    raw2: int | None            # Raw second byte (for 2-byte instructions)
    mnemonic: str               # "LDM 5", "JUN 0x100", "ADD R3"
    accumulator_before: int
    accumulator_after: int
    carry_before: bool
    carry_after: bool
```

## Example Programs

### x = 1 + 2 (Basic Arithmetic)
```asm
LDM 1       ; A = 1                   → 0xD1
XCH R0      ; R0 = 1, A = 0           → 0xB0
LDM 2       ; A = 2                   → 0xD2
ADD R0      ; A = 2 + 1 = 3           → 0x80
XCH R1      ; R1 = 3 (result in R1)   → 0xB1
HLT         ; stop                    → 0x01
```

### Multiply 3 × 4 (Loop with ISZ)
```asm
; R0 = multiplier (3), R1 = counter (-4 = 12 in 4-bit), R2:R3 = result
        FIM P0, 0x30    ; R0=3, R1=0           → 0x20 0x30
        FIM P1, 0x0C    ; R2=0, R3=12 (-4)     → 0x22 0x0C
LOOP:   LD R0           ; A = R0 (3)           → 0xA0
        ADD R2          ; A = A + R2            → 0x82
        XCH R2          ; R2 = running total    → 0xB2
        ISZ R3, LOOP    ; R3++, if R3≠0 goto   → 0x73 0x04
        HLT                                     → 0x01
```

### Subroutine Call (JMS/BBL)
```asm
        LDM 5           ; A = 5                → 0xD5
        JMS ADD_THREE   ; call subroutine      → 0x50 0x08
        XCH R0          ; R0 = result           → 0xB0
        HLT             ;                       → 0x01
        NOP             ; padding               → 0x00
        NOP             ;                       → 0x00
        NOP             ;                       → 0x00
        NOP             ;                       → 0x00
ADD_THREE:
        IAC             ; A++                   → 0xF2
        IAC             ; A++                   → 0xF2
        IAC             ; A++                   → 0xF2
        BBL 0           ; return, A unchanged   → 0xC0
```

### BCD Addition (Decimal Adjust)
```asm
; Add decimal 7 + 8 = 15 (BCD: 0001 0101)
; Result: R0 = 5 (low digit), carry = 1 (high digit)
        LDM 7           ; A = 7                → 0xD7
        XCH R0          ; R0 = 7               → 0xB0
        LDM 8           ; A = 8                → 0xD8
        ADD R0          ; A = 15, carry = 0    → 0x80
        DAA             ; A = 5, carry = 1     → 0xFB
        XCH R0          ; R0 = 5 (low digit)   → 0xB0
        HLT                                     → 0x01
```

## Test Strategy

### Individual Instruction Tests
- Every instruction tested in isolation
- Verify accumulator, registers, carry, and PC after execution
- Edge cases: A=0xF with IAC (overflow), A=0 with DAC (underflow)

### Arithmetic Tests
- ADD with carry propagation
- SUB with borrow (carry semantics inverted)
- ADM/SBM with RAM values
- DAA: BCD correction for values > 9

### Control Flow Tests
- JUN: verify PC jumps to 12-bit address
- JCN: all 16 condition codes (4 bits × invert)
- ISZ: loop counting from N down to 0
- JMS/BBL: subroutine call and return with stack

### Stack Tests
- 1, 2, 3 levels of nesting
- 4th call wraps (overwrites oldest)
- BBL restores correct address at each level

### RAM Tests
- Write/read main characters via SRC + WRM/RDM
- Write/read status characters via WR0–WR3/RD0–RD3
- Bank selection via DCL
- Multiple banks accessed in sequence

### Register Pair Tests
- FIM loads both halves correctly
- SRC sets RAM address from pair
- FIN reads ROM indirectly via P0
- JIN jumps to address in pair

### End-to-End Programs
- x = 1 + 2 (basic)
- Multiplication via loop (ISZ)
- Subroutine call/return (JMS/BBL)
- BCD addition with DAA
- KBP truth table verification
- RAM bank switching (DCL + SRC + WRM/RDM)

### Cross-Language Consistency
- Same programs must produce identical results across all 6 language implementations
