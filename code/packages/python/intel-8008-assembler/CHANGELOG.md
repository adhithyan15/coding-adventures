# Changelog — intel-8008-assembler

All notable changes to this package are documented here.

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
