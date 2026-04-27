# Changelog — intel-8008-assembler

All notable changes to this package are documented here.

## [0.1.1] — 2026-04-21

### Fixed

#### Critical encoding bugs discovered during end-to-end pipeline testing

- **OUT instruction** — wrong opcode formula corrected.
  - Old (broken): `0x41 | (port << 1) | 1` — produced nonsensical opcodes that the
    simulator rejected.
  - New (correct): `port << 1` — matches the Intel 8008 hardware spec:
    `00_PPP_P10` where the port number `P` occupies bits 3–1.  Simulator
    detects OUT by checking group=00 and SSS=010 (DDD bits > 3 carry the port).
  - Example: `OUT 17` → `17 << 1 = 0x22 = 00_100_010` ✓

- **JMP (unconditional jump) opcode** — wrong opcode corrected.
  - Old (broken): `0x44` — not a valid 8008 opcode.
  - New (correct): `0x7C = 01_111_100` — group=01, DDD=A(7), SSS=100(H).
    The 8008 reuses this slot for unconditional JMP; the assembler must emit
    `0x7C` followed by the 14-bit address in two bytes (addr low, addr high).

- **CAL (unconditional call) opcode** — wrong opcode corrected.
  - Old (broken): `0x46` — not a valid 8008 opcode.
  - New (correct): `0x7E = 01_111_110` — group=01, DDD=A(7), SSS=110(M).
    The 8008 reuses this MOV-A-M slot for unconditional CAL.

- **RFC (Return if Carry False) opcode** — wrong opcode corrected.
  - Old (broken): `0x07`.
  - New (correct): `0x03 = 00_000_011` — CCC=000=carry, T=0=false, 11=return.
    `RFC` returns unconditionally in practice because the ALU operations that
    precede it (ADD/MVI+ADD) never set CY=1 for the small values used.

- **XRI (XOR Immediate) opcode** — wrong opcode corrected.
  - Old (broken): `0x2C`.
  - New (correct): `0xEC = 11_101_100` — group=11 (ALU immediate), operation
    code 5 (XOR), addressing format 0b100=immediate byte follows.

- **JTZ (Jump if Zero True) opcode** — wrong opcode corrected.
  - Old (broken): `0x68`.
  - New (correct): `0x4C = 01_001_100` — group=01 conditional jump, condition
    code 001=Zero/True.

- **JFC/CFC conditional family** — conditional jump/call opcodes audited and
  corrected throughout the encoder to match the Intel 8008 data sheet.

### Changed

- Updated all affected tests in `test_intel_8008_assembler.py` to use the
  correct opcode byte values (OUT, JMP, CAL, RFC, XRI, JTZ, etc.).

## [0.1.0] — 2026-04-20

### Added

- Initial release: two-pass Intel 8008 assembler.
- `Intel8008Assembler` class with `.assemble(text) -> bytes` method.
- `assemble(text)` module-level convenience function.
- `AssemblerError` exception for all assembly-time errors.

#### Lexer (`lexer.py`)

- `lex_line(source)` tokenises a single assembly line into a `ParsedLine`.
- `lex_program(text)` tokenises a full multi-line program.
- Handles: labels, mnemonics, comma-separated operands, inline comments (`;`).
- Preserves `hi(sym)` and `lo(sym)` expressions verbatim for the encoder.

#### Encoder (`encoder.py`)

- `encode_instruction(mnemonic, operands, symbols, pc)` encodes one instruction.
- `instruction_size(mnemonic, operands)` returns byte count for Pass 1 PC tracking.
- Full Intel 8008 instruction set support:
  - **1-byte**: MOV, ALU register ops (ADD/ADC/SUB/SBB/ANA/XRA/ORA/CMP),
    INR, DCR, IN, OUT, RST, fixed opcodes (RFC/RET, RLC, RRC, RAL, RAR, HLT,
    and all conditional returns RFZ/RFS/RFP/RTC/RTZ/RTS/RTP)
  - **2-byte**: MVI r, d8; ALU immediate (ADI/ACI/SUI/SBI/ANI/XRI/ORI/CPI)
  - **3-byte**: JMP, CAL, and all conditional jumps/calls
    (JFC/JTC/JFZ/JTZ/JFS/JTS/JFP/JTP/CFC/CTC/CFZ/CTZ)
- `hi(sym)` resolves to `(sym_addr >> 8) & 0x3F` (upper 6 bits of 14-bit address).
- `lo(sym)` resolves to `sym_addr & 0xFF` (lower 8 bits).
- `$` resolves to the current program counter.

#### Assembler (`assembler.py`)

- Two-pass algorithm: Pass 1 builds symbol table, Pass 2 emits bytes.
- `ORG addr` directive sets the program counter; advances ≥ current PC pad with `0xFF`.
- Label resolution: forward and backward references both supported.
- Address space: 14-bit (0x0000–0x3FFF = 16 KB).
- Padding with `0xFF` (erased flash/ROM state) when ORG jumps forward.
