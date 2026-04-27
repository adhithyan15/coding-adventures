"""lexer.py -- Tokenise Intel 4004 assembly source lines.

Why a dedicated lexer?
----------------------

A *lexer* (or *tokeniser*) turns raw text into structured data.  Here,
each line of assembly can be:

  1. Blank / comment-only -> nothing to do
  2. A label definition  -> ``loop_start:``
  3. A directive         -> ``ORG 0x000``
  4. An instruction      -> ``    LDM 5``
  5. A label + instruction on the same line -> ``_start: NOP``

The lexer does NOT look up the symbol table or validate opcodes -- it
merely extracts tokens.  Downstream stages (``encoder``, ``assembler``)
do the semantic work.

Grammar (informal BNF)
----------------------

::

    line        ::= [ label ":" ] [ mnemonic { operand "," } ] [ ";" comment ]
    label       ::= IDENT
    mnemonic    ::= IDENT
    operand     ::= IDENT | NUMBER | "$"
    IDENT       ::= [A-Za-z_][A-Za-z0-9_]*
    NUMBER      ::= "0x" HEX | DECIMAL
    comment     ::= .* (to end of line)

``$`` is a special operand meaning "the current program counter" (used
in ``JUN $`` to create a self-loop / halt equivalent).

Data structures
---------------

A ``ParsedLine`` is a frozen dataclass -- immutable after construction,
safe to pass around freely without defensive copies.

Fields
~~~~~~

``label``     -- Optional label declared on this line.
``mnemonic``  -- The opcode or directive, uppercased.  None if line is blank/comment.
``operands``  -- List of operand strings, stripped of surrounding whitespace.
``source``    -- The original source text (for error messages).

Example
-------

::

    from intel_4004_assembler.lexer import lex_line

    line = "  loop_0_start:  JCN 0x4, loop_0_end  ; check zero"
    parsed = lex_line(line)
    # ParsedLine(label='loop_0_start', mnemonic='JCN',
    #            operands=['0x4', 'loop_0_end'], source='...')
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
    """Tokenise a single line of Intel 4004 assembly.

    Steps
    -----

    1. Strip comments (everything from the first ``;`` onwards).
    2. Check for a leading label.  If found, consume it and continue
       with the remainder.
    3. Split the remainder on the first whitespace to get the mnemonic.
    4. Split the rest on ``,`` to get the operands, stripping each.

    Args:
        source: A single line of assembly text (may include ``\\n``).

    Returns:
        A ``ParsedLine`` with all tokens extracted.

    Examples::

        lex_line("    NOP")
        # ParsedLine(label=None, mnemonic='NOP', operands=(), source='    NOP')

        lex_line("loop:  JUN loop ; forever")
        # ParsedLine(label='loop', mnemonic='JUN', operands=('loop',), ...)

        lex_line("; pure comment")
        # ParsedLine(label=None, mnemonic=None, operands=(), ...)

        lex_line("")
        # ParsedLine(label=None, mnemonic=None, operands=(), ...)
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
        stripped = stripped[m.end() :].lstrip()

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

        from intel_4004_assembler.lexer import lex_program

        src = '''
            ORG 0x000
        _start:
            LDM 5
            HLT
        '''
        lines = lex_program(src)
    """
    return [lex_line(line) for line in text.splitlines()]
