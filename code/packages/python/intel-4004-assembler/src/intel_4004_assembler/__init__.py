"""intel_4004_assembler -- Two-pass assembler for Intel 4004 assembly text.

Overview
--------

This package is PR 10 in the Nib language -> Intel 4004 compiler pipeline.
It sits *after* the ``ir-to-intel-4004-compiler`` package (which produces assembly text
from an ``IrProgram``) and produces raw binary bytes suitable for loading
into a simulator or burning into a ROM.

The pipeline so far::

    Nib source
        ↓  (nib-lexer, nib-parser)
    AST
        ↓  (nib-type-checker)
    Typed AST
        ↓  (nib-ir-compiler)
    IrProgram
        ↓  (ir-optimizer)
    Optimised IrProgram
        ↓  (ir-to-intel-4004-compiler)
    Assembly text             <- intel-4004-assembler reads THIS
        ↓  (this package)
    Binary bytes              -> fed to intel4004-simulator

Two-Pass Algorithm
------------------

**Pass 1** -- Symbol collection
    Walk every line.  Track a program counter (PC).  When a label is
    encountered, record ``{label: PC}`` in the symbol table.  Advance
    PC by the instruction's encoded byte size (1 or 2).

**Pass 2** -- Code emission
    Walk every line again.  For each instruction, look up any label
    operands in the now-complete symbol table.  Encode to bytes and
    append to the output buffer.

Supported Input Format
----------------------

::

        ORG 0x000       ; set PC to 0
    _start:
        LDM 5           ; load immediate 5 -> ACC
        XCH R2          ; swap ACC <-> R2
        NOP
    loop_0_start:
        LD R2
        JCN 0x4, loop_0_end
        ADD_IMM R2, R2, 1
        JUN loop_0_start
    loop_0_end:
        JUN $           ; self-loop (halt equivalent)

Quick Start
-----------

::

    from intel_4004_assembler import assemble, Intel4004Assembler, AssemblerError

    # Option 1: convenience function
    binary = assemble(\"\"\"
        ORG 0x000
    _start:
        LDM 5
        HLT
    \"\"\")
    print(binary.hex())  # "d501"

    # Option 2: class instance (reusable)
    asm = Intel4004Assembler()
    binary = asm.assemble(source_text)

    # Error handling
    try:
        assemble("    JUN undefined_label")
    except AssemblerError as e:
        print(f"Assembly failed: {e}")

Exports
-------

- ``AssemblerError``       -- raised on unknown mnemonics, undefined labels, etc.
- ``Intel4004Assembler``   -- two-pass assembler class; ``assemble(text) -> bytes``
- ``assemble``             -- module-level convenience function

Submodules
----------

- ``lexer``    -- ``lex_line``, ``lex_program``, ``ParsedLine`` dataclass
- ``encoder``  -- ``encode_instruction``, ``instruction_size``, ``AssemblerError``
- ``assembler``-- ``Intel4004Assembler``, ``assemble``
"""

from intel_4004_assembler.assembler import Intel4004Assembler, assemble
from intel_4004_assembler.encoder import AssemblerError

__all__ = [
    "AssemblerError",
    "Intel4004Assembler",
    "assemble",
]
