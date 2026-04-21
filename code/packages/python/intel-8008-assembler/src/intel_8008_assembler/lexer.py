"""lexer.py -- Tokenise Intel 8008 assembly source lines.

Why a dedicated lexer?
----------------------

A *lexer* (or *tokeniser*) turns raw text into structured data.  Here,
each line of assembly can be:

  1. Blank / comment-only -> nothing to do
  2. A label definition  -> ``loop_start:``
  3. A directive         -> ``ORG 0x0000``
  4. An instruction      -> ``    MVI  B, 42``
  5. A label + instruction on the same line -> ``_start: MVI B, 0``

The Intel 8008 assembly syntax produced by ``ir-to-intel-8008-compiler``
follows these conventions:
  - 4-space indent for instructions
  - Labels at column 0 with a colon suffix
  - Comma-separated operands
  - ``hi(symbol)`` and ``lo(symbol)`` directives for 14-bit address halves
  - Semicolon for line comments

The lexer does NOT look up the symbol table or validate opcodes -- it
merely extracts tokens.  Downstream stages (``encoder``, ``assembler``)
do the semantic work.

Grammar (informal BNF)
----------------------

::

    line        ::= [ label ":" ] [ mnemonic { operand "," } ] [ ";" comment ]
    label       ::= IDENT
    mnemonic    ::= IDENT
    operand     ::= IDENT | NUMBER | "hi(" IDENT ")" | "lo(" IDENT ")" | "$"
    IDENT       ::= [A-Za-z_][A-Za-z0-9_]*
    NUMBER      ::= "0x" HEX | DECIMAL

``$`` is a special operand meaning "the current program counter" (used
in ``JMP $`` to create a self-loop / halt equivalent).

``hi(symbol)`` and ``lo(symbol)`` are special expressions that the
encoder resolves to the high-6 and low-8 bits of the symbol's address.
They appear as operands to ``MVI`` instructions emitted for LOAD_ADDR.

Data structures
---------------

A ``ParsedLine`` is a frozen dataclass -- immutable after construction,
safe to pass around freely without defensive copies.

Fields
~~~~~~

``label``     -- Optional label declared on this line.
``mnemonic``  -- The opcode or directive, uppercased.  None if line is blank/comment.
``operands``  -- Tuple of operand strings, stripped of surrounding whitespace.
``source``    -- The original source text (for error messages).

Example
-------

::

    from intel_8008_assembler.lexer import lex_line

    line = "  loop_0_start:  MVI  B, 42  ; load 42 into B"
    parsed = lex_line(line)
    # ParsedLine(label='loop_0_start', mnemonic='MVI',
    #            operands=['B', '42'], source='...')
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field


@dataclass(frozen=True)
class ParsedLine:
    """Structured representation of a single source line.

    Frozen so callers cannot mutate it after the lexer returns it.
    The ``source`` field is kept for diagnostic messages.

    Attributes:
        label:    Label name declared on this line, or ``None``.
        mnemonic: Uppercased opcode / directive, or ``None`` for blank lines.
        operands: Zero or more operand strings in source order.
        source:   The raw source text of this line (for error messages).
    """

    label: str | None
    mnemonic: str | None
    operands: tuple[str, ...]
    source: str = field(compare=False)


# ---------------------------------------------------------------------------
# Internal helpers
# ---------------------------------------------------------------------------

# Matches an optional label at the start: e.g.  "loop_start:"  or  "_start:"
# The label may appear with or without leading whitespace.
_LABEL_RE = re.compile(r"^([A-Za-z_][A-Za-z0-9_]*):")


def lex_line(source: str) -> ParsedLine:
    """Tokenise a single line of Intel 8008 assembly.

    Steps
    -----

    1. Strip comments (everything from the first ``;`` onwards).
    2. Check for a leading label.  If found, consume it and continue
       with the remainder.
    3. Split the remainder on the first whitespace to get the mnemonic.
    4. Split the rest on ``,`` to get the operands, stripping each.

    The ``hi(sym)`` and ``lo(sym)`` expressions are preserved verbatim as
    operand strings; the encoder resolves them in Pass 2.

    Args:
        source: A single line of assembly text (may include ``\\n``).

    Returns:
        A ``ParsedLine`` with all tokens extracted.

    Examples::

        lex_line("    MVI  B, 42")
        # ParsedLine(label=None, mnemonic='MVI', operands=('B', '42'), ...)

        lex_line("loop:  JMP loop ; forever")
        # ParsedLine(label='loop', mnemonic='JMP', operands=('loop',), ...)

        lex_line("; pure comment")
        # ParsedLine(label=None, mnemonic=None, operands=(), ...)

        lex_line("    MVI  H, hi(counter)")
        # ParsedLine(label=None, mnemonic='MVI', operands=('H', 'hi(counter)'), ...)
    """
    # --- Step 1: strip comment -------------------------------------------------
    # Split at first ";" to remove everything after it.
    text = source.split(";", 1)[0].rstrip()

    # --- Step 2: check for label -----------------------------------------------
    stripped = text.lstrip()
    label: str | None = None
    m = _LABEL_RE.match(stripped)
    if m:
        label = m.group(1)
        # Remove the "label:" prefix and continue with the rest.
        stripped = stripped[m.end():].lstrip()

    # --- Step 3: blank after stripping? ----------------------------------------
    if not stripped:
        return ParsedLine(label=label, mnemonic=None, operands=(), source=source)

    # --- Step 4: split mnemonic from operands ----------------------------------
    # The mnemonic is the first whitespace-delimited token.
    parts = stripped.split(None, 1)
    mnemonic = parts[0].upper()

    if len(parts) == 1:
        # No operands.
        return ParsedLine(label=label, mnemonic=mnemonic, operands=(), source=source)

    # --- Step 5: split operands on comma ---------------------------------------
    # Each operand is stripped of surrounding whitespace.
    # Special care: "hi(sym)" and "lo(sym)" contain parentheses but no commas,
    # so a simple comma-split works correctly here.
    raw_operands = parts[1]
    operands = tuple(op.strip() for op in raw_operands.split(","))

    return ParsedLine(
        label=label,
        mnemonic=mnemonic,
        operands=operands,
        source=source,
    )


def lex_program(text: str) -> list[ParsedLine]:
    """Tokenise every line in a multi-line assembly program.

    Blank lines and comment-only lines are included as ``ParsedLine``
    objects with ``mnemonic=None``.  This preserves line numbers for
    error reporting.

    Args:
        text: The entire assembly source, potentially multi-line.

    Returns:
        A list of ``ParsedLine`` objects, one per source line.

    Example::

        from intel_8008_assembler.lexer import lex_program

        src = '''
            ORG 0x0000
        _start:
            MVI  B, 0
            HLT
        '''
        lines = lex_program(src)
    """
    return [lex_line(line) for line in text.splitlines()]
