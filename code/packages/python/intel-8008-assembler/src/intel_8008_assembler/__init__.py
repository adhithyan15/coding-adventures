"""intel_8008_assembler -- Two-pass assembler for Intel 8008 assembly text.

Overview
--------

This package sits in the Oct → Intel 8008 compiler pipeline, after the
``ir-to-intel-8008-compiler`` package (which produces assembly text from an
``IrProgram``) and produces raw binary bytes suitable for packaging into
an Intel HEX file by the ``intel-8008-packager`` package.

The pipeline::

    Oct source (.oct)
        ↓  (oct-lexer, oct-parser, oct-type-checker)
    AST / Typed AST
        ↓  (oct-ir-compiler)
    IrProgram
        ↓  (intel-8008-ir-validator)
    Validated IrProgram
        ↓  (ir-to-intel-8008-compiler)
    8008 Assembly text (.asm)    <- intel-8008-assembler reads THIS
        ↓  (this package)
    Binary bytes                 -> fed to intel-8008-packager
        ↓  (intel-8008-packager)
    Intel HEX file (.hex)        -> fed to intel8008-simulator

Two-Pass Algorithm
------------------

**Pass 1** -- Symbol collection
    Walk every line.  Track a program counter (PC).  When a label is
    encountered, record ``{label: PC}`` in the symbol table.  Advance
    PC by the instruction's encoded byte size (1, 2, or 3 bytes).

**Pass 2** -- Code emission
    Walk every line again.  For each instruction, look up any label
    operands in the now-complete symbol table.  Encode to bytes and
    append to the output buffer.

Instruction Sizes
-----------------

The Intel 8008 has three instruction widths:

- **1 byte**: fixed opcodes (RFC/RET, HLT, RLC, RRC, RAL, RAR, etc.),
  register operations (MOV, ADD, ADC, SUB, SBB, ANA, XRA, ORA, CMP,
  INR, DCR), I/O (IN p, OUT p)
- **2 bytes**: immediate operations (MVI r, d8; ADI, ACI, SUI, SBI,
  ANI, XRI, ORI, CPI)
- **3 bytes**: jump/call instructions (JMP, CAL, JFC, JTC, JFZ, JTZ,
  JFS, JTS, JFP, JTP, CFC, CTC, CFZ, CTZ)

The 14-bit address for 3-byte instructions is encoded as:
  byte 2 = addr & 0xFF      (low 8 bits)
  byte 3 = (addr >> 8) & 0x3F  (high 6 bits)

hi()/lo() Directives
--------------------

The code generator uses ``hi(symbol)`` and ``lo(symbol)`` as MVI operands
to load 14-bit static variable addresses into H:L:

    hi(addr) = (addr >> 8) & 0x3F   (upper 6 bits)
    lo(addr) = addr & 0xFF           (lower 8 bits)

These are resolved by the assembler during Pass 2.

Supported Input Format
----------------------

::

        ORG 0x0000       ; set PC to 0
    _start:
        MVI  B, 0        ; zero register B (constant zero)
        CAL  _fn_main    ; call main
        HLT              ; halt after return
    _fn_main:
        MVI  D, 42       ; load constant into D
        MOV  A, D        ; copy D → accumulator (for return value)
        RFC              ; return (unconditional return if carry false)

Quick Start
-----------

::

    from intel_8008_assembler import assemble, Intel8008Assembler, AssemblerError

    # Option 1: convenience function
    binary = assemble(\"\"\"
        ORG 0x0000
    _start:
        MVI  B, 0
        HLT
    \"\"\")
    print(binary.hex())  # "0600ff"

    # Option 2: class instance (reusable)
    asm = Intel8008Assembler()
    binary = asm.assemble(source_text)

    # Error handling
    try:
        assemble("    JMP undefined_label")
    except AssemblerError as e:
        print(f"Assembly failed: {e}")

Exports
-------

- ``AssemblerError``        -- raised on unknown mnemonics, undefined labels, etc.
- ``Intel8008Assembler``    -- two-pass assembler class; ``assemble(text) -> bytes``
- ``assemble``              -- module-level convenience function

Submodules
----------

- ``lexer``    -- ``lex_line``, ``lex_program``, ``ParsedLine`` dataclass
- ``encoder``  -- ``encode_instruction``, ``instruction_size``, ``AssemblerError``
- ``assembler``-- ``Intel8008Assembler``, ``assemble``
"""

from intel_8008_assembler.assembler import Intel8008Assembler, assemble
from intel_8008_assembler.encoder import AssemblerError

__all__ = [
    "AssemblerError",
    "Intel8008Assembler",
    "assemble",
]
