# intel-8008-assembler

A two-pass assembler for Intel 8008 assembly source text.  Part of the
Oct → Intel 8008 compiler pipeline in `coding-adventures`.

---

## Pipeline position

```
Oct source (.oct)
  → oct-lexer, oct-parser, oct-type-checker
AST / Typed AST
  → oct-ir-compiler
IrProgram
  → intel-8008-ir-validator        (constraint checks)
Validated IrProgram
  → ir-to-intel-8008-compiler      (IR → assembly text)
8008 Assembly text (.asm)          ← THIS CRATE reads this
  → intel-8008-assembler
Binary bytes                       → fed to intel-8008-packager
  → intel-8008-packager
Intel HEX file (.hex)              → fed to intel8008-simulator
```

---

## What it does

**Pass 1 — symbol collection:**  
Walks every source line, tracks the program counter (`pc`), and records
every label definition `my_label:` as `symbols["my_label"] = pc`.
`ORG addr` sets `pc = addr` directly.

**Pass 2 — code emission:**  
Walks every source line again, encoding each instruction with the completed
symbol table.  Forward references (a `JMP done` before `done:` is declared)
work because Pass 1 has already resolved all addresses.

`ORG addr` in Pass 2 pads the output with `0xFF` bytes (erased flash / ROM
state) up to the new address.

---

## Instruction set

| Form | Size | Example |
|------|------|---------|
| `HLT` | 1 byte | halt |
| `RFC` / `RET` | 1 byte | return (unconditional) |
| `RTC/RFZ/RTZ/RFS/RTS/RFP/RTP` | 1 byte | conditional return |
| `RLC/RRC/RAL/RAR` | 1 byte | rotate accumulator |
| `MOV dst, src` | 1 byte | register move |
| `INR r` | 1 byte | increment register |
| `DCR r` | 1 byte | decrement register |
| `ADD/ADC/SUB/SBB/ANA/XRA/ORA/CMP r` | 1 byte | ALU reg op |
| `IN p` | 1 byte | read port p (0–7) |
| `OUT p` | 1 byte | write port p (0–23) |
| `RST n` | 1 byte | restart (n = 0–7) |
| `MVI r, d8` | 2 bytes | move immediate |
| `ADI/ACI/SUI/SBI/ANI/XRI/ORI/CPI d8` | 2 bytes | ALU immediate |
| `JMP/CAL addr` | 3 bytes | unconditional jump/call |
| `JFC/JTC/JFZ/JTZ/…` | 3 bytes | conditional jump |
| `CFC/CTC/CFZ/CTZ/…` | 3 bytes | conditional call |
| `ORG addr` | 0 bytes | set origin (assembler directive) |

### Registers

`A`, `B`, `C`, `D`, `E`, `H`, `L`, `M` (memory at H:L).

### Address encoding (3-byte instructions)

```
[opcode, lo8(addr), hi6(addr)]
  lo8 = addr & 0xFF
  hi6 = (addr >> 8) & 0x3F
```

### hi() / lo() expressions

Load a 14-bit symbol address into the H:L register pair:

```asm
MVI  H, hi(counter)   ; high 6 bits of counter's address
MVI  L, lo(counter)   ; low 8 bits
```

---

## Usage

```rust
use intel_8008_assembler::{assemble, Intel8008Assembler, AssemblerError};

// Using the convenience function
let binary: Vec<u8> = assemble("
    ORG 0x0000
_start:
    MVI  B, 5
loop:
    DCR  B
    JTZ  done
    JMP  loop
done:
    HLT
").unwrap();

// Using the struct (same behaviour)
let binary = Intel8008Assembler.assemble("    HLT\n").unwrap();
assert_eq!(binary, vec![0xFF]);
```

---

## Error handling

`assemble()` returns `Err(AssemblerError)` on:
- Unknown mnemonic
- Undefined label reference
- Immediate value out of 8-bit range (0–255)
- Address out of 14-bit range (0–0x3FFF)
- Port number out of range
- Wrong operand count

```rust
use intel_8008_assembler::assemble;
let err = assemble("    JMP undefined\n").unwrap_err();
println!("{err}");  // "Undefined label: \"undefined\""
```

---

## Tests

```
cargo test -p intel-8008-assembler
```

44 tests (40 unit + 4 doc-tests), all passing.
