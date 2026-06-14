# Changelog — intel-8008-assembler

All notable changes to this crate are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-04-28

### Added

- **Two-pass assembler** (`Intel8008Assembler`, `assemble()`) for Intel 8008
  assembly source text → binary bytes.
- **Pass 1 — symbol collection**: walks all lines, tracks `pc`, records label
  addresses in a `HashMap<String, usize>`.  `ORG` directive sets `pc` directly.
- **Pass 2 — code emission**: encodes each instruction using the completed
  symbol table.  `ORG` pads forward with `0xFF` (erased flash / ROM state).
- **Lexer** (`lex_line`, `lex_program`, `ParsedLine`): comment stripping,
  label detection via character-by-character scan (avoids `regex` dependency),
  mnemonic/operand splitting.
- **`parse_label_prefix`**: scans for `ident:` prefix; only matches if the
  token is immediately followed by `:`.
- **Encoder** (`encode_instruction`): dispatches on mnemonic; handles:
  - Fixed 1-byte: `HLT`, `RFC`/`RET`, `RTC`, `RFZ`/`RTZ`, `RFS`/`RTS`,
    `RFP`/`RTP`, `RLC`, `RRC`, `RAL`, `RAR`
  - `MOV dst, src` — Group 01: `0x40 | (dst << 3) | src`
  - `MVI r, d8` — Group 00: `(r << 3) | 0x06` + immediate byte
  - `INR r` — Group 00: `r << 3`
  - `DCR r` — Group 00: `(r << 3) | 0x01`
  - `RST n` — Group 00: `(n << 3) | 0x05`
  - ALU-register (`ADD`, `ADC`, `SUB`, `SBB`, `ANA`, `XRA`, `ORA`, `CMP`):
    Group 10 base-opcode `| reg_code`
  - ALU-immediate (`ADI`, `ACI`, `SUI`, `SBI`, `ANI`, `XRI`, `ORI`, `CPI`):
    Group 11 opcode + d8
  - `IN p` — `0x41 | (p << 3)`, port 0–7
  - `OUT p` — `p << 1`, port 0–23
  - Jump/call (unconditional: `JMP=0x7C`, `CAL=0x7E`; conditional: `JFC`,
    `JTC`, `JFZ`, `JTZ`, `JFS`, `JTS`, `JFP`, `JTP`, `CFC`, `CTC`, `CFZ`,
    `CTZ`, `CFS`, `CTS`, `CFP`, `CTP`): 3 bytes `[opcode, lo8(addr), hi6(addr)]`
- **`resolve_operand`**: handles `$` (current PC), `0x`-prefixed hex, decimal
  literals, `hi(sym)` / `lo(sym)` expressions, and label references.
- **`resolve_hi_lo`**: `hi(addr) = (addr >> 8) & 0x3F`, `lo(addr) = addr & 0xFF`
  — enables loading 14-bit symbol addresses into H:L register pair.
- **`AssemblerError`** — public error type implementing `Display` + `Error`.
- **44 tests** (40 unit + 4 doc-tests) with 100% pass rate:
  - Lexer: blank line, comment-only, label-only, label+instruction, hi/lo
    preservation, comment stripping
  - Instruction sizes: fixed, ALU-reg, 2-byte, 3-byte, ORG, unknown
  - Encoder: every instruction form; hi/lo operands; error cases
  - Full assembler: minimal halt, MVI+HLT, forward label, CAL+RET, JTZ loop,
    ORG padding, error cases, `$` operand
