"""logic-bytecode — the first compact opcode form for LP07 logic programs.

`logic-instructions` gave the logic stack a structured instruction stream.
`logic-bytecode` lowers that stream into compact opcode/operand pairs plus
indexed pools, then provides decoding and disassembly so the format stays easy
to inspect and round-trip.
"""

from logic_bytecode.bytecode import (
    LogicBytecodeDisassemblyLine,
    LogicBytecodeError,
    LogicBytecodeInstruction,
    LogicBytecodeOp,
    LogicBytecodeProgram,
    compile_program,
    decode_program,
    disassemble,
    disassemble_text,
)

__all__ = [
    "__version__",
    "LogicBytecodeDisassemblyLine",
    "LogicBytecodeError",
    "LogicBytecodeInstruction",
    "LogicBytecodeOp",
    "LogicBytecodeProgram",
    "compile_program",
    "decode_program",
    "disassemble",
    "disassemble_text",
]

__version__ = "0.1.0"
