"""IR Parser — text → IrProgram.

The parser reads the canonical IR text format (produced by ``print_ir``) and
reconstructs an IrProgram. This enables:

  1. Golden-file testing — load an expected .ir file, parse it, compare
  2. Roundtrip verification — parse(print(program)) == program
  3. Manual IR authoring — write IR by hand for testing backends

Parsing Strategy
----------------

The parser processes the text line by line:

  1. Lines starting with ``.version`` set the program version
  2. Lines starting with ``.data`` add a data declaration
  3. Lines starting with ``.entry`` set the entry label
  4. Lines ending with ``:`` define a label
  5. Lines starting with whitespace are instructions
  6. Lines starting with ``;`` are standalone comments
  7. Blank lines are skipped

Each instruction line is split into: opcode, operands, and optional ``; #N``
ID comment. Operands are parsed as:

  - Registers: starts with ``v`` followed by digits (e.g., ``v0``, ``v1``)
  - Immediates: parseable as integer (e.g., ``42``, ``-1``, ``255``)
  - Labels: anything else (e.g., ``tape``, ``loop_0_start``)

Security Limits
---------------

The parser enforces conservative limits to prevent denial-of-service from
adversarial input:

  - Maximum 1,000,000 lines
  - Maximum 16 operands per instruction
  - Register indices must be in range [0, 65535]
"""

from __future__ import annotations

import re

from compiler_ir.opcodes import IrOp, parse_op
from compiler_ir.types import (
    IrDataDecl,
    IrFloatImmediate,
    IrImmediate,
    IrInstruction,
    IrLabel,
    IrOperand,
    IrProgram,
    IrRegister,
)

# Maximum limits for IR text parsing. These prevent denial-of-service
# from adversarial input by capping memory allocation.
_MAX_LINES = 1_000_000       # max lines in an IR text file
_MAX_OPERANDS_PER_INSTR = 16  # max operands per instruction
_MAX_REGISTER_INDEX = 65535   # max virtual register index (v0..v65535)
_FLOAT_IMMEDIATE_RE = re.compile(
    r"^[+-]?(?:"
    r"(?:\d+\.\d*|\.\d+)(?:[eE][+-]?\d+)?"
    r"|"
    r"\d+[eE][+-]?\d+"
    r")$"
)


class IrParseError(ValueError):
    """Raised when the IR text is malformed.

    Attributes:
        message: Human-readable description of what went wrong.

    Example::

        try:
            parse_ir("FROBNITZ v0")
        except IrParseError as e:
            print(e)  # "line 1: unknown opcode 'FROBNITZ'"
    """


def parse_ir(text: str) -> IrProgram:
    """Convert IR text into an IrProgram.

    Parses the canonical IR text format (as produced by ``print_ir``) and
    returns a fully populated IrProgram. This is the inverse of ``print_ir``.

    The parser is line-oriented. Each line is classified by its prefix:

    - ``.version N`` → sets program version
    - ``.data label size init`` → adds a data declaration
    - ``.entry label`` → sets the entry label
    - ``label:`` → adds a LABEL instruction
    - ``  OPCODE ...`` → parses an instruction
    - ``; text`` → adds a COMMENT instruction
    - (blank) → skipped

    Args:
        text: The IR text to parse.

    Returns:
        A fully populated ``IrProgram``.

    Raises:
        IrParseError: If the text is malformed.

    Example::

        prog = parse_ir(\"\"\".version 1

    .data tape 30000 0

    .entry _start

    _start:
      LOAD_ADDR   v0, tape  ; #0
      HALT                  ; #1
    \"\"\")
        prog.version      # 1
        prog.entry_label  # "_start"
    """
    program = IrProgram(entry_label="", version=1)
    lines = text.split("\n")

    if len(lines) > _MAX_LINES:
        raise IrParseError(
            f"input too large: {len(lines)} lines (max {_MAX_LINES})"
        )

    for line_idx, line in enumerate(lines):
        line_num = line_idx + 1
        trimmed = line.strip()

        # Skip blank lines
        if not trimmed:
            continue

        # Version directive: ".version 1"
        if trimmed.startswith(".version"):
            parts = trimmed.split()
            if len(parts) != 2:
                raise IrParseError(
                    f"line {line_num}: invalid .version directive: {line!r}"
                )
            try:
                program.version = int(parts[1])
            except ValueError as exc:
                raise IrParseError(
                    f"line {line_num}: invalid version number: {parts[1]!r}"
                ) from exc
            continue

        # Data declaration: ".data tape 30000 0"
        if trimmed.startswith(".data"):
            parts = trimmed.split()
            if len(parts) != 4:
                raise IrParseError(
                    f"line {line_num}: invalid .data directive: {line!r}"
                )
            try:
                size = int(parts[2])
            except ValueError as exc:
                raise IrParseError(
                    f"line {line_num}: invalid data size: {parts[2]!r}"
                ) from exc
            try:
                init = int(parts[3])
            except ValueError as exc:
                raise IrParseError(
                    f"line {line_num}: invalid data init: {parts[3]!r}"
                ) from exc
            program.add_data(IrDataDecl(label=parts[1], size=size, init=init))
            continue

        # Entry point: ".entry _start"
        if trimmed.startswith(".entry"):
            parts = trimmed.split()
            if len(parts) != 2:
                raise IrParseError(
                    f"line {line_num}: invalid .entry directive: {line!r}"
                )
            program.entry_label = parts[1]
            continue

        # Label definition: "loop_0_start:"
        # Must not start with ";" (which would be a comment) and must end with ":"
        if trimmed.endswith(":") and not trimmed.startswith(";"):
            label_name = trimmed[:-1]  # strip the trailing colon
            program.add_instruction(IrInstruction(
                opcode=IrOp.LABEL,
                operands=[IrLabel(name=label_name)],
                id=-1,  # labels have no meaningful ID
            ))
            continue

        # Standalone comment line: "; text" (but NOT "; #N" which is an ID comment)
        if trimmed.startswith(";"):
            comment_text = trimmed[1:].strip()
            # Skip ID-only comments like "; #42" — those are part of instructions
            if not comment_text.startswith("#"):
                program.add_instruction(IrInstruction(
                    opcode=IrOp.COMMENT,
                    operands=[IrLabel(name=comment_text)],
                    id=-1,
                ))
            continue

        # Regular instruction line (indented or not after stripping)
        instr = _parse_instruction_line(trimmed, line_num)
        program.add_instruction(instr)

    return program


def _parse_instruction_line(line: str, line_num: int) -> IrInstruction:
    """Parse a single instruction line like ``"LOAD_IMM   v0, 42  ; #3"``.

    Args:
        line:     The trimmed instruction line text.
        line_num: The 1-based line number (for error messages).

    Returns:
        A populated ``IrInstruction``.

    Raises:
        IrParseError: If the line is malformed.
    """
    # Split off the "; #N" ID comment if present.
    # We search from the right to avoid false positives in comment text.
    id_ = -1
    instruction_part = line

    semicolon_idx = line.rfind("; #")
    if semicolon_idx >= 0:
        id_str = line[semicolon_idx + 3:].strip()
        try:
            id_ = int(id_str)
            instruction_part = line[:semicolon_idx].strip()
        except ValueError:
            pass  # not a numeric ID — treat as part of the instruction text

    # Split into opcode and operand tokens
    fields = instruction_part.split()
    if not fields:
        raise IrParseError(f"line {line_num}: empty instruction")

    opcode_name = fields[0]
    opcode = parse_op(opcode_name)
    if opcode is None:
        raise IrParseError(f"line {line_num}: unknown opcode {opcode_name!r}")

    # Parse operands — everything after the opcode, comma-separated.
    # We rejoin the remaining fields and split on "," to handle
    # the canonical "v0, v1, 42" format.
    operands: list[IrOperand] = []
    if len(fields) > 1:
        operand_text = " ".join(fields[1:])
        raw_parts = operand_text.split(",")
        if len(raw_parts) > _MAX_OPERANDS_PER_INSTR:
            raise IrParseError(
                f"line {line_num}: too many operands "
                f"({len(raw_parts)}, max {_MAX_OPERANDS_PER_INSTR})"
            )
        for raw in raw_parts:
            raw = raw.strip()
            if not raw:
                continue
            operand = _parse_operand(raw, line_num)
            operands.append(operand)

    return IrInstruction(opcode=opcode, operands=operands, id=id_)


def _parse_operand(s: str, line_num: int) -> IrOperand:
    """Parse a single operand string into an IrOperand.

    Parsing rules (in order of precedence):

    1. Starts with ``v`` followed by digits → ``IrRegister``
    2. Parseable as integer → ``IrImmediate``
    3. Parseable as float literal → ``IrFloatImmediate``
    4. Anything else → ``IrLabel``

    Args:
        s:        The operand text (already stripped).
        line_num: Line number for error messages.

    Returns:
        An ``IrRegister``, ``IrImmediate``, ``IrFloatImmediate``, or ``IrLabel``.

    Raises:
        IrParseError: If the register index is out of range.
    """
    # Register: v0, v1, v2, ...
    if len(s) > 1 and s[0] == "v":
        try:
            idx = int(s[1:])
            if idx < 0 or idx > _MAX_REGISTER_INDEX:
                raise IrParseError(
                    f"line {line_num}: register index {idx} out of range "
                    f"(max {_MAX_REGISTER_INDEX})"
                )
            return IrRegister(index=idx)
        except ValueError:
            pass  # not a valid register index — fall through to label

    # Immediate: 42, -1, 255, ...
    try:
        return IrImmediate(value=int(s))
    except ValueError:
        pass

    # Floating immediate: 1.5, -0.25, 1e3, ...
    if _FLOAT_IMMEDIATE_RE.match(s):
        try:
            return IrFloatImmediate(value=float(s))
        except ValueError:
            pass

    # Label: _start, loop_0_end, tape, ...
    return IrLabel(name=s)
