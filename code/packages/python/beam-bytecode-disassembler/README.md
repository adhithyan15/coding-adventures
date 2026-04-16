# beam-bytecode-disassembler

Symbolic BEAM instruction decoding built on top of the reusable bytes decoder.

This package is responsible for:

- Decoding compact BEAM operands
- Turning the `Code` chunk into instruction records
- Building label and export lookup tables
- Producing a neutral disassembled module representation
