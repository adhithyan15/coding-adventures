"""IR Printer — IrProgram → human-readable text.

The printer converts an IrProgram into its canonical text format.
This format serves three purposes:

  1. Debugging — humans can read the IR to understand what the compiler did
  2. Golden-file tests — expected IR output is committed as .ir text files
  3. Roundtrip — parse(print(program)) == program is a testable invariant

Text Format
-----------

::

  .version 1

  .data tape 30000 0

  .entry _start

  _start:
    LOAD_ADDR   v0, tape          ; #0
    LOAD_IMM    v1, 0             ; #1
    HALT                          ; #2

Key rules:

- ``.version N`` is always the first non-comment line
- ``.data`` declarations come before ``.entry``
- Labels are on their own line with a trailing colon, no indentation
- Instructions are indented with two spaces
- ``; #N`` comments show instruction IDs (informational, not semantic)
- COMMENT instructions emit as ``; <text>`` on their own indented line
- Opcode names are left-padded to 11 characters for column alignment
"""

from __future__ import annotations

from compiler_ir.opcodes import IrOp
from compiler_ir.types import IrProgram

_OPCODE_COLUMN_WIDTH = max(len(op.name) for op in IrOp) + 1


def print_ir(program: IrProgram) -> str:
    """Convert an IrProgram to its canonical text representation.

    This is the inverse of ``parse_ir()``. The output is deterministic and
    stable — the same program always produces the same text. The format
    is designed for both human readability and roundtrip parsing.

    Args:
        program: The IR program to print.

    Returns:
        A multi-line string in the canonical IR text format.

    Example::

        prog = IrProgram(entry_label="_start")
        prog.add_data(IrDataDecl("tape", 30000, 0))
        prog.add_instruction(IrInstruction(IrOp.HALT, [], id=0))
        text = print_ir(prog)
        # ".version 1\\n\\n.data tape 30000 0\\n\\n.entry _start\\n\\n"
        # "  HALT       ; #0\\n"
    """
    parts: list[str] = []

    # Version directive — always first
    parts.append(f".version {program.version}\n")

    # Data declarations — before the entry point
    for decl in program.data:
        parts.append(f"\n.data {decl.label} {decl.size} {decl.init}\n")

    # Entry point
    parts.append(f"\n.entry {program.entry_label}\n")

    # Instructions
    for instr in program.instructions:
        if instr.opcode == IrOp.LABEL:
            # Labels get their own unindented line with a trailing colon.
            # Example: "loop_0_start:\n"
            label_name = str(instr.operands[0]) if instr.operands else ""
            parts.append(f"\n{label_name}:\n")
            continue

        if instr.opcode == IrOp.COMMENT:
            # Comments emit as "  ; <text>\n" — indented to align with instructions.
            text = str(instr.operands[0]) if instr.operands else ""
            parts.append(f"  ; {text}\n")
            continue

        # Regular instruction: "  OPCODE     operands  ; #ID\n"
        # The opcode name is padded to 11 characters so operand columns align.
        opcode_name = instr.opcode.name
        operand_str = ", ".join(str(op) for op in instr.operands)

        line = f"  {opcode_name:<{_OPCODE_COLUMN_WIDTH}}"
        if operand_str:
            line += operand_str
        line += f"  ; #{instr.id}\n"

        parts.append(line)

    return "".join(parts)
