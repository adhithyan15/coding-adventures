# IR01 — IR to GE-225 Compiler

## Overview

This spec defines the `ir-to-ge225-compiler` Python package: a backend that translates a
`compiler_ir.IrProgram` into a binary image of GE-225 20-bit machine words ready to load
into the GE-225 simulator.

The GE-225 was a 20-bit word-addressed accumulator machine built by General Electric in
the early 1960s. Dartmouth's time-sharing system ran on it in 1964, executing the world's
first BASIC programs. Every variable, every loop counter, every comparison result lives in
a 20-bit word in memory; the single accumulator register A is the bottleneck through which
all arithmetic flows.

This backend sits at layer 4 in the compiled pipeline:

```
IrProgram  (from dartmouth-basic-ir-compiler, spec PL02)
    ↓  ir-to-ge225-compiler   (this spec — emit machine words)
    ↓  ge225-simulator         (execute)
```

The backend knows nothing about BASIC syntax. Its input is a generic `IrProgram`; a future
`ir-to-wasm-compiler` or `ir-to-jvm-compiler` would accept the same input.

## GE-225 Architecture Primer

### Word Format

All memory cells are 20-bit words, stored as three bytes (big-endian, top 4 bits unused):

```
Bit 19: sign bit (1 = negative, two's complement)
Bits 18–0: data/address/opcode field
```

### Instruction Encoding

Memory-reference instructions (the majority) pack into one 20-bit word:

```
[19:15]  opcode    (5 bits — 32 possible opcodes)
[14:13]  modifier  (2 bits — X register group for indexed addressing)
[12:0]   address   (13 bits — direct address 0–8191)
```

Effective address = `address + X[modifier]` when modifier ≠ 0, or `address` when modifier = 0.

### Key Instructions for This Backend

**Memory reference (OP modifier, address):**

| Mnemonic | Opcode | Effect |
|----------|--------|--------|
| `LDA` | 0o00 | A = mem[ea] |
| `ADD` | 0o01 | A = A + mem[ea] |
| `SUB` | 0o02 | A = A - mem[ea] |
| `STA` | 0o03 | mem[ea] = A |
| `MPY` | 0o15 | A,Q = Q × mem[ea] + A (40-bit accumulate multiply) |
| `DVD` | 0o16 | A = (A,Q) ÷ mem[ea] (quotient); Q = remainder |
| `BRU` | 0o26 | PC = ea (unconditional branch) |
| `SPB` | 0o07 | X[mod] = PC; PC = ea (subroutine call) |

**Fixed-word instructions (no address field — full 20-bit word is constant):**

| Mnemonic | Word (octal) | Effect |
|----------|-------------|--------|
| `LDZ` | 2504002 | A = 0 |
| `LDO` | 2504022 | A = 1 |
| `LMO` | 2504102 | A = all ones (0xFFFFF) |
| `NOP` | 2504012 | No operation |
| `NEG` | 2504522 | A = −A |
| `LAQ` | 2504001 | A = Q |
| `LQA` | 2504004 | Q = A |
| `TON` | 2500007 | Turn on typewriter |
| `TYP` | 2500006 | Print character code in N register |
| `BMI` | 2514001 | Skip next if A < 0 (negative) |
| `BPL` | 2516001 | Skip next if A ≥ 0 (non-negative) |
| `BZE` | 2514002 | Skip next if A = 0 |
| `BNZ` | 2516002 | Skip next if A ≠ 0 |

**Shift instruction (parametric fixed word):**

| Mnemonic | Effect |
|----------|--------|
| `SAN k`  | Right-shift the concatenation of A[18:0] and N[5:0] by k bits; bits from A enter N from the top. Used to load the N register. |

`SAN k` encodes as `assemble_shift("SAN", k)` from the simulator's API.

### The Accumulator Model

The GE-225 has no general-purpose register file. All arithmetic flows through a single
accumulator (A). To compute `A + B`, the sequence is:

```
LDA [A]      ; A_reg = mem[A]
ADD [B]      ; A_reg = A_reg + mem[B]
STA [result] ; mem[result] = A_reg
```

Every IR virtual register is mapped to a *spill slot*: a dedicated memory word. Operations
read from spill slots into A, compute, and write back. This makes every operation 2–8 words
but is completely correct. Future versions could optimize with peephole passes.

### Branch Semantics

The GE-225 conditional branches are **skip-next-if-condition** instructions, not
jump-if-condition. To jump to a far label on a condition, a two-word sequence is needed:

```
; Jump to LABEL if A == 0:
BNZ            ; if A ≠ 0, skip next word (don't jump)
BRU LABEL      ; executed only when A == 0
; fall through here when A ≠ 0
```

```
; Jump to LABEL if A ≠ 0:
BZE            ; if A = 0, skip next word (don't jump)
BRU LABEL      ; executed only when A ≠ 0
```

```
; Jump to LABEL if A < 0 (negative):
BPL            ; if A ≥ 0, skip next word
BRU LABEL
```

This pattern is used for all conditional branches in this backend.

### Halt Convention

The GE-225 has no HALT instruction. Programs that finish are expected to wait for I/O or
return to the OS. For our simulator, we use the **self-loop halt stub**:

```
__halt: BRU __halt   ; PC loops to itself forever
```

The `HALT` IR instruction compiles to `BRU __halt`. After compilation the backend returns
the `halt_address` alongside the binary. The integration layer calls `simulator.step()` in
a loop and stops when `trace.address == halt_address` (the step that *executes* the BRU to
itself is the last meaningful step).

### Typewriter Output

The GE-225's typewriter (Teletype) output works through the 6-bit N register:

1. `TON` — turn on typewriter power (must precede any TYP)
2. Load a 6-bit typewriter code into N via `SAN 6`:
   - Place the 6-bit code in the low bits of A
   - `SAN 6` shifts those 6 bits from A into N
3. `TYP` — print the character whose GE-225 code is in N

`SAN 6` derivation (from the simulator source):

```
combined = (A_data[18:0] << 6) | N[5:0]    ; 25 bits
combined >>= 6                               ; shift right by 6
new_A_data = (combined >> 6) & 0x7FFFF      ; high 19 bits
new_N = combined & 0x3F                      ; low 6 bits
```

For a 6-bit code X stored in A (A = X, 0 ≤ X < 64):
- combined = X << 6 | old_N
- after >>= 6: combined = X (the high 6 bits of X<<6 shifted back)
- new_N = X & 0x3F = X ✓
- new_A = (X >> 6) & mask = 0 (for X < 64) ✓

So `LDA [spill_v0]; SAN 6; TYP` correctly prints the character.

## Memory Layout

The compiled binary occupies a contiguous region of GE-225 memory starting at word 0:

```
┌─────────────────────────────────────────────────┐
│  0 … code_end-1 : Code words                    │
│                   (compiled IR instructions)     │
├─────────────────────────────────────────────────┤
│  code_end       : Halt stub                      │
│                   BRU code_end  (self-loop)      │
├─────────────────────────────────────────────────┤
│  data_base … :  Data words                       │
│                   spill_v0, spill_v1, …          │
│                   (one word per virtual register) │
│                   const_0, const_1, …            │
│                   (constants table for LOAD_IMM) │
└─────────────────────────────────────────────────┘
```

- `data_base = code_end + 1`
- All BASIC variables (v1–v26 for A–Z) are spill slots.
- The constants table holds unique integer values referenced by `LOAD_IMM` instructions.
  If the same constant appears 50 times in the program, it appears once in the table.

## Spill-Slot and Constants Table Allocation

### Pass 0: Collect Virtual Registers and Constants

Before the two-pass assembly, scan all IR instructions once:

1. For each `IrRegister` operand, record `reg.index` in a set.
2. For each `IrImmediate` operand in a `LOAD_IMM` instruction, record the value in a dict
   mapping `value → const_index`. Constants are assigned indices in encounter order.

After pass 0:
- `max_reg` = maximum register index seen
- `n_regs` = max_reg + 1 (number of spill slots)
- `n_consts` = number of unique constants
- `spill_addr(N) = data_base + N`
- `const_addr(K) = data_base + n_regs + K`

**Note:** `data_base` is not yet known — it depends on the size of the code region, which
is determined in pass 1. The constants and spill offsets are *relative to data_base* during
passes 1 and 2; absolute addresses are resolved in a final link step.

## Two-Pass Assembly

### Pass 1: Size and Label Assignment

Walk the IR instruction list. For each instruction, compute how many GE-225 words it
produces and record the starting code address.

Maintain a counter `word_addr` starting at 0.

For each IR instruction:
- `LABEL name`: record `labels[name] = word_addr` (labels occupy 0 words).
- `COMMENT text`: 0 words.
- All others: add the word count from the table below to `word_addr`.

After pass 1:
- `code_end = word_addr` (first address after all code words).
- `data_base = code_end + 1` (halt stub is one word at code_end).
- All label addresses in `labels` are now absolute.

### Pass 2: Emit Words

Walk the IR instruction list again. For each instruction, emit the appropriate GE-225 words
using the now-known `data_base` and `labels` map. Collect all emitted words into a list.

After pass 2, append:
1. The halt stub: `encode_instruction(OP_BRU, 0, code_end)` (self-loop).
2. Data words: `n_regs` zero-initialized spill slots, then `n_consts` constant values (each
   in the low 20 bits as a signed two's complement 20-bit integer).

The final binary is `pack_words(code_words + [halt_stub_word] + data_words)`.

## IR Opcode to GE-225 Word Sequence

Notation:
- `spill(vN)` = `data_base + N` = absolute memory address of register vN's spill slot
- `const(K)` = `data_base + n_regs + K` = absolute memory address of constant K
- `label(name)` = resolved code address from the label map

### LABEL name

*0 words.* Side effect: records `labels[name] = current_word_addr` in pass 1.

### COMMENT text

*0 words.*

### NOP

```
NOP                            ; 1 word: fixed word 2504012₈
```

*1 word.*

### HALT

```
BRU code_end                   ; 1 word: jump to halt stub
```

*1 word.*

### LOAD_IMM vDst, imm

```
LDA const_addr                 ; 1 word: A = mem[const_addr]
STA spill(vDst)                ; 1 word: store to dst spill slot
```

*2 words.*

The constant `imm` is pre-stored in the data region at `const_addr`.

### ADD vDst, vA, vB

```
LDA spill(vA)                  ; 1 word: A = vA
ADD spill(vB)                  ; 1 word: A = A + vB
STA spill(vDst)                ; 1 word: store result
```

*3 words.*

### ADD_IMM vDst, vSrc, imm

For immediates that are 0 (register copy):

```
LDA spill(vSrc)                ; 1 word
STA spill(vDst)                ; 1 word
```

For +1:

```
LDA spill(vSrc)                ; 1 word: A = vSrc
ADO                            ; 1 word (fixed): A = A + 1
STA spill(vDst)                ; 1 word: store
```

*3 words.*

For -1:

```
LDA spill(vSrc)
SBO                            ; A = A - 1
STA spill(vDst)
```

*3 words.*

For all other immediates, store the immediate in the constants table and use ADD:

```
LDA spill(vSrc)
ADD const_addr                 ; A = vSrc + imm
STA spill(vDst)
```

*3 words.*

### SUB vDst, vA, vB

```
LDA spill(vA)
SUB spill(vB)                  ; A = vA - vB
STA spill(vDst)
```

*3 words.*

### AND_IMM vDst, vSrc, imm

The GE-225 has no direct AND-immediate instruction. The immediate is stored in the
constants table and the backend emits:

```
LDA spill(vSrc)
EXT const_addr                 ; A = A & ~mem[const_addr]  ← NOT what we want
```

The EXT (extract) instruction computes `A = A & ~mask`, which is NOT a logical AND.

For `AND_IMM v, v, 1` (used to mask the low bit), we can avoid EXT entirely:

```
; A = vSrc & 1 — get parity bit
LDA spill(vSrc)
BOD                            ; skip next if A is odd (bit 0 = 1)
BRU __and_imm_N_zero
LDO                            ; A = 1 (odd: bit 0 was 1)
BRU __and_imm_N_done
__and_imm_N_zero:
LDZ                            ; A = 0 (even: bit 0 was 0)
__and_imm_N_done:
STA spill(vDst)
```

*8 words.* This handles the `AND_IMM v, v, 1` pattern from the `<=` / `>=` NOT idiom
(spec PL02). General AND with arbitrary immediates is deferred to V2 (not needed for V1).

Raise `CodeGenError` if AND_IMM is used with an immediate other than 1 in V1.

### MUL vDst, vA, vB

GE-225 multiply: `MPY mem` computes `A,Q = Q × mem + A`. Setup Q = vA, A = 0,
then MPY vB, then extract the product from Q (for values fitting in 20 bits).

```
LDA  spill(vA)                 ; A = vA
LQA                            ; Q = A = vA
LDZ                            ; A = 0 (accumulator part)
MPY  spill(vB)                 ; A,Q = Q*vB + A = vA*vB + 0
LAQ                            ; A = Q (low 20 bits = product for small values)
STA  spill(vDst)               ; store result
```

*6 words.*

**Note on overflow:** For large products (|vA × vB| ≥ 2¹⁹), the result spills into the A
register (high 20 bits). V1 does not check for overflow and silently gives a wrong result.
A V2 addition would check the A register after LAQ and raise a runtime overflow signal.

### DIV vDst, vA, vB

GE-225 divide: `DVD mem` computes quotient into A, remainder into Q, given 40-bit
dividend in (A, Q). Setup A = 0 (high half), Q = vA (low half of dividend).

```
LDA  spill(vA)                 ; A = vA
LQA                            ; Q = vA
LDZ                            ; A = 0 (high half of 40-bit dividend)
DVD  spill(vB)                 ; A = (A,Q) / vB (quotient); Q = remainder
STA  spill(vDst)               ; store quotient
```

*5 words.*

**Divide by zero:** The GE-225 simulator raises `ZeroDivisionError` in Python. The integration
layer catches this and wraps it in a `RuntimeError`.

### CMP_EQ vDst, vA, vB

```
LDA  spill(vA)
SUB  spill(vB)                 ; A = vA - vB (zero iff equal)
BNZ                            ; if A≠0, skip next (go to not-equal branch)
BRU  __cmp_eq_N_true           ; A==0 → jump to true case
LDZ                            ; A = 0 (not equal)
BRU  __cmp_eq_N_done
__cmp_eq_N_true:
LDO                            ; A = 1 (equal)
__cmp_eq_N_done:
STA  spill(vDst)
```

*8 words* (two labels at zero cost).

### CMP_NE vDst, vA, vB

```
LDA  spill(vA)
SUB  spill(vB)                 ; A = vA - vB
BZE                            ; if A==0, skip next (they're equal, NE is false)
BRU  __cmp_ne_N_true
LDZ                            ; equal → result 0
BRU  __cmp_ne_N_done
__cmp_ne_N_true:
LDO                            ; not equal → result 1
__cmp_ne_N_done:
STA  spill(vDst)
```

*8 words.*

### CMP_LT vDst, vA, vB (vA < vB)

A negative difference means vA < vB:

```
LDA  spill(vA)
SUB  spill(vB)                 ; A = vA - vB (negative iff vA < vB)
BPL                            ; if A ≥ 0, skip next (not less than)
BRU  __cmp_lt_N_true
LDZ                            ; ≥ 0 → result 0
BRU  __cmp_lt_N_done
__cmp_lt_N_true:
LDO                            ; negative → result 1
__cmp_lt_N_done:
STA  spill(vDst)
```

*8 words.*

### CMP_GT vDst, vA, vB (vA > vB)

Swap operands: vA > vB iff vB < vA:

```
LDA  spill(vB)
SUB  spill(vA)                 ; A = vB - vA (negative iff vB < vA, i.e., vA > vB)
BPL
BRU  __cmp_gt_N_true
LDZ
BRU  __cmp_gt_N_done
__cmp_gt_N_true:
LDO
__cmp_gt_N_done:
STA  spill(vDst)
```

*8 words.*

### JUMP label

```
BRU  label(name)               ; 1 word
```

*1 word.*

### BRANCH_Z vN, label  (jump if vN == 0)

```
LDA  spill(vN)                 ; 1 word: A = vN
BNZ                            ; 1 word: if A≠0, skip next
BRU  label(name)               ; 1 word: jump (executed when A==0)
; fall through when A≠0
```

*3 words.*

### BRANCH_NZ vN, label  (jump if vN ≠ 0)

```
LDA  spill(vN)
BZE                            ; if A==0, skip next
BRU  label(name)               ; jump (executed when A≠0)
```

*3 words.*

### SYSCALL IrImmediate(1)  (print char from v0)

The char's GE-225 typewriter code is already in spill_v0 (put there by the preceding
`LOAD_IMM v0, code` instruction from the BASIC compiler):

```
LDA  spill(v0)                 ; A = typewriter code (6-bit value)
SAN  6                         ; shift A[5:0] into N register
TYP                            ; print N as typewriter character
```

*3 words.*

`TON` (typewriter on) is emitted once in the program prologue (see below), not per
character.

### Unsupported opcodes

All other IR opcodes (`LOAD_BYTE`, `STORE_BYTE`, `LOAD_WORD`, `STORE_WORD`, `LOAD_ADDR`,
`AND`, `CALL`, `RET`, `SYSCALL` with other numbers) raise `CodeGenError` in V1.

## Prologue and Epilogue

The backend prepends and appends a small number of fixed words before and after the
compiled IR:

**Prologue** (before all IR code):

```
LABEL _start               ; marks address 0 (where simulator PC begins)
TON                        ; turn on typewriter (enables TYP)
```

The prologue is prepended to the instruction list before pass 1, so the first code word
is always `TON` at address 0.

**Epilogue** (after all IR code):

None — the halt stub is handled structurally as part of the memory layout.

## Word Count Reference Table

| IR Opcode | GE-225 Words |
|-----------|-------------|
| LABEL | 0 |
| COMMENT | 0 |
| NOP | 1 |
| HALT | 1 |
| JUMP | 1 |
| LOAD_IMM | 2 |
| ADD | 3 |
| ADD_IMM (imm=0, copy) | 2 |
| ADD_IMM (imm=±1) | 3 |
| ADD_IMM (other) | 3 |
| SUB | 3 |
| AND_IMM (imm=1 only) | 8 |
| MUL | 6 |
| DIV | 5 |
| CMP_EQ | 8 |
| CMP_NE | 8 |
| CMP_LT | 8 |
| CMP_GT | 8 |
| BRANCH_Z | 3 |
| BRANCH_NZ | 3 |
| SYSCALL 1 | 3 |

These counts are used in pass 1 to assign absolute addresses to all labels.

## Public API

```python
from ir_to_ge225_compiler import compile_to_ge225, CompileResult, CodeGenError

@dataclass
class CompileResult:
    binary: bytes           # packed 3-bytes-per-word GE-225 image
    halt_address: int       # code word address of the halt stub (self-loop)
    data_base: int          # first data word address
    label_map: dict[str, int]   # label name → code word address

def compile_to_ge225(program: IrProgram) -> CompileResult:
    """Compile an IrProgram to a GE-225 binary image.

    Args:
        program: A validated IrProgram (all LABEL targets must be defined).

    Returns:
        CompileResult with the binary image and metadata.

    Raises:
        CodeGenError: if the program uses an unsupported IR opcode (V1 subset),
                      if an AND_IMM uses a non-1 immediate, or if a referenced
                      label is undefined.
    """
```

## Package Structure

```
packages/python/ir-to-ge225-compiler/
├── pyproject.toml
├── README.md
├── CHANGELOG.md
└── src/
    └── ir_to_ge225_compiler/
        ├── __init__.py           (exports compile_to_ge225, CompileResult, CodeGenError)
        ├── codegen.py            (internal two-pass _CodeGen class)
        └── ge225_encoding.py     (re-exports encode_instruction, assemble_fixed,
                                   assemble_shift, pack_words from ge225_simulator)
```

## Dependencies

```toml
[project]
dependencies = [
    "coding-adventures-compiler-ir",
    "coding-adventures-ge225-simulator",
]

[tool.uv.sources]
coding-adventures-compiler-ir = { path = "../compiler-ir", editable = true }
coding-adventures-ge225-simulator = { path = "../ge225-simulator", editable = true }
```

## Test Plan

Each test verifies correct word emission by loading the binary into a `GE225Simulator`
and executing it, then inspecting the simulator state.

- **HALT**: program with only HALT → PC reaches halt_address; simulator stays there.
- **LOAD_IMM + ADD**: `v1 = 3; v2 = 4; v3 = v1 + v2` → `spill(v3)` holds 7.
- **SUB**: `v1 = 10; v2 = 3; v3 = v1 - v2` → `spill(v3)` holds 7.
- **MUL**: `v1 = 6; v2 = 7; v3 = v1 * v2` → `spill(v3)` holds 42.
- **DIV**: `v1 = 15; v2 = 4; v3 = v1 / v2` → `spill(v3)` holds 3.
- **CMP_EQ true**: equal inputs → `spill(vDst)` = 1.
- **CMP_EQ false**: unequal inputs → `spill(vDst)` = 0.
- **CMP_LT true/false**: both directions.
- **CMP_GT true/false**: both directions.
- **JUMP**: unconditional jump skips an intervening LOAD_IMM.
- **BRANCH_Z**: conditional skip when register is zero and non-zero.
- **BRANCH_NZ**: same.
- **FOR-like pattern**: LABEL + CMP + BRANCH + ADD + JUMP + LABEL forms a working countdown.
- **SYSCALL 1**: after compilation, typewriter output contains the expected characters.
- **Undefined label**: `CodeGenError` raised with the missing label name.
- **AND_IMM 1**: low-bit masking correct (0 and 1 inputs).
- **AND_IMM non-1**: `CodeGenError` in V1.
- **Negative numbers**: LOAD_IMM with negative immediate stored and retrieved correctly.

Coverage target: 90%.

## Example: Full Trace for `1 + 2`

BASIC `10 LET A = 1 + 2` → IR:

```
LABEL _start
LABEL _line_10
LOAD_IMM v287, 1          ; temp: 1
LOAD_IMM v288, 2          ; temp: 2
ADD      v289, v287, v288 ; temp: 1 + 2
ADD_IMM  v1, v289, 0      ; A = temp (copy)
HALT
```

With 1 variable (A=v1), one syscall reg (v0), and temporaries v287–v289:
- `max_reg` = 289
- `n_regs` = 290 (one spill slot per v0..v289)
- `n_consts` = 2 (constants: 1 and 2)

Pass 1 (word address assignment):

| IR | Words | Code addr |
|----|-------|----------|
| TON (prologue) | 1 | 0 |
| LABEL _start | 0 | 1 |
| LABEL _line_10 | 0 | 1 |
| LOAD_IMM v287, 1 | 2 | 1 |
| LOAD_IMM v288, 2 | 2 | 3 |
| ADD v289, v287, v288 | 3 | 5 |
| ADD_IMM v1, v289, 0 | 2 | 8 |
| HALT | 1 | 10 |

`code_end = 11`, `halt_stub at 11`, `data_base = 12`.

Pass 2 emitted words (decimal addresses):

```
addr 0:  TON                    (fixed word 2500007₈)
addr 1:  LDA const(1)=302       ; 1 at data_base + n_regs + 0 = 12+290+0=302
addr 2:  STA spill(287)=299     ; 299 = 12 + 287
addr 3:  LDA const(2)=303       ; 2 at 12+290+1=303
addr 4:  STA spill(288)=300
addr 5:  LDA spill(287)=299     ; vA
addr 6:  ADD spill(288)=300     ; vB
addr 7:  STA spill(289)=301     ; result
addr 8:  LDA spill(289)=301     ; copy
addr 9:  STA spill(1)=13        ; store to v1 (BASIC variable A)
addr 10: BRU 11                 ; HALT → jump to halt stub
addr 11: BRU 11                 ; halt stub self-loop
addr 12-301: 290 zero words     ; spill slots v0..v289
addr 302: 1                     ; constant 1
addr 303: 2                     ; constant 2
```

After execution, `simulator.read_word(13)` = 3 (variable A = 3). ✓
