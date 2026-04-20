"""Oct Lexer — tokenizes Oct source text using the grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarLexer``. It loads
the ``oct.tokens`` file from the ``code/grammars/`` directory and creates a
lexer configured for Oct tokenization.

What Is Oct?
-------------

Oct is a safe, statically-typed toy language designed to compile to Intel 8008
machine code. The name comes from *octet* — the networking/communications term
for exactly 8 bits, the native word size of the Intel 8008 ALU.

The Intel 8008 (1972) was Intel's first commercial 8-bit microprocessor —
a direct successor to the 4-bit 4004, and the ancestor of the x86 family.
Its key hardware constraints:

- **8-bit accumulator (A)**: Each ALU operation works on one byte (0–255).
- **7 general-purpose registers**: A (accumulator), B, C, D, E (GP), H, L
  (memory pointer pair). The H:L pair addresses the 14-bit memory space.
- **4 registers for locals**: Only B, C, D, E are available for function
  local variables; A is the accumulator (scratch); H:L are the memory pointer.
  At most 4 locals (including parameters) per function.
- **16 KB of addressable memory**: 14-bit address bus (0x0000–0x3FFF).
  ROM occupies 0x0000–0x1FFF; RAM data segment 0x2000–0x3FFF.
- **8-level push-down call stack**: The hardware maintains an internal stack
  of 8 program counter registers. One is always in use for the current
  function, leaving 7 usable levels for nested calls.
- **Separate I/O port space**: 8 input ports (INP p), 24 output ports (OUT p).
  Port numbers are encoded in the instruction opcode — there is no "variable
  port" addressing mode.
- **4 flags**: CY (carry), Z (zero), S (sign), P (parity).
  Oct exposes these via carry() and parity() intrinsics.

Writing 8008 assembly by hand is tedious and error-prone. Oct gives us a
higher-level notation with static safety guarantees while remaining faithful
to the hardware. Every Oct construct maps to a small, predictable sequence of
8008 instructions — a student reading the generated assembly should be able
to trace every Oct expression back to the hardware instructions they studied.

Why Oct Instead of Extending Nib?
-----------------------------------

Nib targets the Intel 4004 (4-bit). Oct targets the Intel 8008 (8-bit). The
word sizes, instruction sets, and addressing models are different enough that
sharing a compiler would produce worse code and worse error messages. Key
differences:

- Nib's ``u4`` type (4 bits) vs Oct's ``u8`` type (8 bits)
- Nib's wrapping (``+%``) and saturating (``+?``) operators vs Oct's plain
  ``+``/``-`` (carry is always available via carry() in Oct)
- Nib's for-loop with compile-time bounds vs Oct's while/loop/break
- Nib's 3-level call stack vs Oct's 7-level call stack
- 4004 port model vs 8008's separate 8-input / 24-output port model
- 4004's 4 KB ROM vs 8008's 16 KB address space

The Token Set
-------------

Oct's token set reflects the 8008's capabilities. There are no string literals
(the 8008 cannot display text), no floating-point (integer-only hardware), and
no multiplication or division operators (the 8008 has no multiply/divide
instructions — use repeated addition or shift-and-add).

**Multi-character operators (listed first — first-match-wins)**

== (EQ_EQ)
    Equality comparison. Oct uses ``=`` for assignment and ``==`` for equality,
    avoiding the classic C bug of writing ``=`` where ``==`` is intended.

!= (NEQ)
    Not-equal comparison. Tokenized atomically so ``!=`` is never lexed as
    ``!`` followed by ``=``.

<= (LEQ), >= (GEQ)
    Less-or-equal, greater-or-equal (unsigned). Must precede ``<`` and ``>``
    respectively.

&& (LAND)
    Logical AND (short-circuit). Distinct from bitwise AND (``&`` / ANA r).

|| (LOR)
    Logical OR (short-circuit). Distinct from bitwise OR (``|`` / ORA r).

-> (ARROW)
    Return type annotation separator: ``fn f() -> u8 { … }``.

**Single-character arithmetic operators**

+ (PLUS), - (MINUS)
    Unsigned addition and subtraction, wrapping modulo 256. Map to 8008
    ADD r and SUB r instructions. There is no ``*`` or ``/`` — the 8008
    has no multiply or divide hardware.

**Single-character bitwise operators**

& (AMP)
    Bitwise AND. Maps to 8008 ANA r.

| (PIPE)
    Bitwise OR. Maps to 8008 ORA r.

^ (CARET)
    Bitwise XOR. Maps to 8008 XRA r.

~ (TILDE)
    Bitwise NOT (unary). Flips all 8 bits. Lowered to XRI 0xFF on 8008.

! (BANG)
    Logical NOT (unary). Negates a bool expression.

**Delimiters**

{ } (LBRACE, RBRACE)
    Block delimiters. Required on all if/else/while/loop/fn bodies.

( ) (LPAREN, RPAREN)
    Expression grouping and argument/parameter lists.

: (COLON)
    Type annotation separator: ``let x: u8 = 5;``

; (SEMICOLON)
    Statement terminator.

, (COMMA)
    Argument/parameter separator.

**Literals**

BIN_LIT — ``0b00001111``, ``0b11111111``
    Binary integer literals. Must precede INT_LIT to avoid ``0b101`` being
    lexed as ``0`` + NAME(``b101``). Binary is the clearest notation for
    bit masks, flags, and port control bytes on 8-bit hardware.

HEX_LIT — ``0xFF``, ``0x3A``
    Hexadecimal integer literals. Must precede INT_LIT for the same reason.
    Hex is natural for memory addresses and ASCII codes.

INT_LIT — ``0``, ``42``, ``255``
    Decimal integer literals. Valid u8 range is 0–255 (enforced at compile
    time, not by the lexer).

NAME — identifiers and type names (``u8``, ``bool``)
    Type names are NAME tokens with specific values, not keywords. This keeps
    the keyword set minimal.

**Keywords**

Control: ``fn``, ``let``, ``static``, ``if``, ``else``, ``while``, ``loop``,
         ``break``, ``return``

Literals: ``true`` (1), ``false`` (0)

Intrinsics (used as function calls):
  ``in``, ``out``, ``adc``, ``sbb``, ``rlc``, ``rrc``, ``ral``, ``rar``,
  ``carry``, ``parity``

**Skipped automatically**

Whitespace (spaces, tabs, CR, LF) and line comments (``// …`` to end of line).
Block comments (``/* … */``) are not supported in Oct v1.
"""

from __future__ import annotations

from dataclasses import replace
from pathlib import Path

from grammar_tools import parse_token_grammar
from lexer import GrammarLexer, Token
from lexer.tokenizer import TokenType

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------
#
# Navigate from this file's location up to the repository root's grammars/
# directory. The path is:
#   src/oct_lexer/tokenizer.py -> src/oct_lexer -> src -> oct-lexer
#   -> python -> packages -> code -> code/grammars
# ---------------------------------------------------------------------------

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
OCT_TOKENS_PATH = GRAMMAR_DIR / "oct.tokens"


def create_oct_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for Oct source text.

    This function reads the ``oct.tokens`` file, parses it into a
    ``TokenGrammar``, and creates a ``GrammarLexer`` ready to tokenize
    the given source text.

    The lexer handles the following Oct-specific behaviours automatically:

    - **Keyword reclassification**: identifiers like ``fn``, ``let``,
      ``while``, ``in``, ``carry``, etc. are reclassified from NAME to
      their keyword token kind after a full-token match. ``invert`` stays
      NAME because the full token ``invert`` does not match keyword ``in``.
    - **Case sensitivity**: Oct keywords are lowercase only. ``FN`` stays
      NAME; only ``fn`` becomes the ``fn`` keyword.
    - **Multi-character operators**: ``==`` before ``=``, ``!=`` before
      ``!``, ``<=`` before ``<``, ``>=`` before ``>``, ``&&`` before ``&``,
      ``||`` before ``|``, ``->`` before ``-``.
    - **BIN_LIT priority**: ``0b101`` consumed as one token before the
      decimal digit rule fires on the leading ``0``.
    - **HEX_LIT priority**: ``0xFF`` consumed as one hex literal token.
    - **Comment skipping**: ``// text`` to end of line consumed silently.
    - **Whitespace skipping**: spaces, tabs, CR, LF between tokens ignored.

    Args:
        source: The Oct source text to tokenize.

    Returns:
        A ``GrammarLexer`` instance configured with Oct token definitions.
        Call ``.tokenize()`` on it to get the token list.

    Raises:
        FileNotFoundError: If the ``oct.tokens`` file cannot be found.
        TokenGrammarError: If the ``.tokens`` file has syntax errors.

    Example::

        lexer = create_oct_lexer('fn main() { let x: u8 = 0xFF; }')
        tokens = lexer.tokenize()
    """
    grammar = parse_token_grammar(OCT_TOKENS_PATH.read_text())
    return GrammarLexer(source, grammar)


def tokenize_oct(source: str) -> list[Token]:
    """Tokenize Oct source text and return a list of tokens.

    This is the main entry point for the Oct lexer. It creates a configured
    ``GrammarLexer`` for Oct, runs it over the source text, and applies a
    post-processing pass that promotes keyword tokens:

    The ``GrammarLexer`` produces keyword tokens with ``type=TokenType.KEYWORD``
    and ``value="fn"``, ``"let"``, ``"carry"``, etc.  Oct convention (and all
    downstream consumers including oct-parser and oct-type-checker) expect the
    token type to equal the keyword value string — ``Token("fn", "fn")``,
    ``Token("let", "let")``, ``Token("carry", "carry")``, etc.  The post-
    processing pass normalises this.

    Intrinsic keywords (``in``, ``out``, ``adc``, ``sbb``, ``rlc``, ``rrc``,
    ``ral``, ``rar``, ``carry``, ``parity``) are promoted exactly the same way
    as control-flow keywords. The parser's ``intrinsic_call`` rule matches
    them by their string value (e.g. ``"carry"``), so no special handling is
    needed here beyond the standard KEYWORD → value promotion.

    Args:
        source: The Oct source text to tokenize.

    Returns:
        A list of ``Token`` objects. The last token is always EOF.

    Raises:
        FileNotFoundError: If the ``oct.tokens`` file cannot be found.
        LexerError: If the source contains characters that don't match
            any token pattern in Oct.

    Example — simple variable declaration::

        tokens = tokenize_oct('let x: u8 = 42;')
        # [Token('let', 'let'), Token(NAME, 'x'), Token(COLON, ':'),
        #  Token(NAME, 'u8'), Token(EQ, '='), Token(INT_LIT, '42'),
        #  Token(SEMICOLON, ';'), Token(EOF, '')]

    Example — binary and hex literals::

        tokens = tokenize_oct('let mask: u8 = 0b11110000 & 0xFF;')
        # [Token('let', 'let'), Token(NAME, 'mask'), Token(COLON, ':'),
        #  Token(NAME, 'u8'), Token(EQ, '='), Token(BIN_LIT, '0b11110000'),
        #  Token(AMP, '&'), Token(HEX_LIT, '0xFF'), Token(SEMICOLON, ';'),
        #  Token(EOF, '')]

    Example — intrinsic keyword tokens::

        tokens = tokenize_oct('let b: u8 = in(0);')
        # [Token('let', 'let'), Token(NAME, 'b'), Token(COLON, ':'),
        #  Token(NAME, 'u8'), Token(EQ, '='), Token('in', 'in'),
        #  Token(LPAREN, '('), Token(INT_LIT, '0'), Token(RPAREN, ')'),
        #  Token(SEMICOLON, ';'), Token(EOF, '')]
    """
    lexer = create_oct_lexer(source)
    raw = lexer.tokenize()

    # The GrammarLexer uses the keywords: section in oct.tokens, which causes
    # keyword tokens to come back with type=TokenType.KEYWORD and value="fn",
    # "let", "carry", "in", etc.  Oct convention (and all downstream consumers)
    # expect the token type to equal the lowercase keyword text:
    #   Token("fn", "fn"), Token("carry", "carry"), Token("in", "in"), etc.
    # This post-processing pass promotes TokenType.KEYWORD → the value string.
    return [
        replace(tok, type=tok.value) if tok.type is TokenType.KEYWORD else tok
        for tok in raw
    ]
