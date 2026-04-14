"""Nib Lexer — tokenizes Nib source text using the grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarLexer``. It loads
the ``nib.tokens`` file from the ``code/grammars/`` directory and creates a
lexer configured for Nib tokenization.

What Is Nib?
-------------

Nib is a safe, statically-typed toy language designed to compile down to Intel
4004 machine code. The name comes from *nibble* — the 4-bit unit of data that
is the native word size of the Intel 4004.

The Intel 4004 was the world's first commercial microprocessor, released by
Intel in November 1971. Intel engineer Federico Faggin led its design for
Japanese calculator company Busicom. It is a 4-bit CPU with extreme hardware
constraints:

- **4-bit accumulator**: Each register holds exactly 4 bits (0–15).
- **160 bytes of RAM**: Split across 16 RAM chips, each with 4 registers of
  4 nibbles each. Shared between program data and the hardware call stack.
- **4 KB of ROM**: Program storage. All code and constants live here.
- **3-level hardware call stack**: The CPU hardware itself only supports 3
  nested function calls. A 4th level silently overwrites the return address.
- **No multiply or divide instructions**: Integer arithmetic was add, subtract,
  and bitwise. Multiplication had to be emulated with repeated addition.

Writing 4004 assembly by hand is error-prone. Nib gives us a safer,
higher-level notation that can be verified and compiled rather than
hand-assembled.

Why a New Language for the 4004?
----------------------------------

Existing high-level languages (C, Pascal, Fortran) assume at minimum a 16-bit
word size and a heap allocator. Neither assumption holds on the 4004.

Nib is designed ground-up for the 4004's constraints:

1. **Type system matches the hardware**: ``u4`` is a 4-bit unsigned integer,
   ``u8`` is 8-bit (two nibbles), ``bcd`` is a binary-coded decimal digit,
   and ``bool`` is a boolean. There are no 16-bit or 32-bit types — the 4004
   hardware cannot hold them in a single register.

2. **Static call depth bound**: The compiler enforces that the static call
   graph is acyclic and no path is deeper than 2 frames (reserving one level
   for the current frame), preventing silent stack overflow.

3. **No recursion**: A recursive call graph would require a heap-allocated
   call stack. Nib bans it at compile time.

4. **No heap allocation**: The 4004 has no heap. All data is either on the
   static RAM chip registers or in ROM as constants.

5. **Bounded loops**: Loop bounds must be compile-time constants so the
   compiler can predict ROM usage exactly. No unbounded ``while`` loops.

The Token Set
-------------

Nib's token set reflects these constraints. There are no string literals
(the 4004 cannot display text), no floating-point (the hardware is integer-
only), and no exponentiation (multiply is already banned in v1).

**Multi-character operators (listed first — first-match-wins)**

+% (WRAP_ADD)
    Wrapping addition. On a u4, ``15 +% 1 = 0`` — the result wraps around
    modularly, discarding the carry. The ``%`` sigil signals "modular wrap",
    mirroring Rust's ``wrapping_add()`` and Zig's ``+%`` operator. Explicit
    wrapping prevents accidental silent overflow in safety-critical embedded
    code.

+? (SAT_ADD)
    Saturating addition. ``15 +? 1 = 15`` on u4 — the result clamps at the
    maximum value instead of wrapping. The ``?`` sigil asks "did we overflow?"
    and saturates at the limit. Useful for BCD digit carry: you want to stay
    at 9 rather than wrap to 0. Mirrors ARM's UQADD instruction and Rust's
    ``saturating_add()``.

Why ``+%`` and ``+?`` are separate tokens:
    Both start with ``+``, but they are fundamentally different arithmetic
    operations — modular vs. clamped. Making them separate tokens (rather
    than ``+`` with a flag) keeps the grammar unambiguous and forces the
    programmer to choose explicitly, preventing silent accidental overflow.
    Both must appear before the plain ``+`` (PLUS) token in the grammar so
    the two-character sequences are consumed atomically.

.. (RANGE)
    Range separator for for-loops: ``for i: u4 in 0..8 { ... }``. The two-dot
    notation is borrowed from Rust and is exclusive on the right (0..8 means
    0, 1, …, 7 — eight iterations). Exclusive ranges make the trip count equal
    to upper − lower, matching the 4004's DJNZ loop pattern. The lexer must
    see ``..`` as one token so ``0..8`` doesn't lex as ``0`` ``.`` ``.`` ``8``.
    Nib has no floating-point, so a single ``.`` never appears outside string
    or comment context.

-> (ARROW)
    Function return type annotation: ``fn add(a: u4, b: u4) -> u4 { ... }``.
    The arrow reads "produces" — borrowed from Haskell and Rust. Must appear
    before MINUS so ``->`` is not lexed as MINUS GT.

== (EQ_EQ)
    Equality comparison. Nib uses ``=`` for assignment (in declarations) and
    ``==`` for comparison (in expressions). This is the Rust/C convention.
    Must appear before EQ so ``==`` is not lexed as EQ EQ.

!= (NEQ)
    Not-equal comparison. Standard C-family syntax. ``!`` is the logical NOT
    unary operator, so ``!=`` is a two-character sequence, not BANG EQ. Must
    appear before BANG so ``!=`` is lexed atomically.

<= (LEQ), >= (GEQ)
    Comparison operators. Must appear before LT and GT so the two-character
    sequences win over their single-character prefixes.

&& (LAND), || (LOR)
    Short-circuit logical AND and OR. Double characters distinguish logical
    operators from bitwise operators (``&`` for ANL, ``|`` for ORL on the
    4004). Must appear before AMP and PIPE.

**Literals**

HEX_LIT before INT_LIT:
    ``0xFF`` must be lexed as a single HEX_LIT token, not INT_LIT("0")
    followed by NAME("xFF"). The ``0x`` prefix indicates a hexadecimal literal.
    Hex is crucial for 4004 programming: nibble masks (``0xF``), port values,
    and ROM addresses are all naturally expressed in hex. If INT_LIT appeared
    first in the grammar, the leading ``0`` would match as INT_LIT("0"), and
    the ``x`` would fail to start a new token. By placing HEX_LIT first, the
    full hex token is consumed before the decimal rule fires.

**Keywords** (reclassified from NAME after a full-token match)

Keywords are case-sensitive and lowercase-only. Partial matches do not
qualify: ``format`` is NAME (identifier), not ``for`` + NAME(``mat``). The
keyword match fires only on the complete token.

Notable non-keywords: ``u4``, ``u8``, ``bcd``, ``bool`` are NOT keywords —
they are NAME tokens handled as type productions in the parser. Keeping types
out of the keyword list keeps the keyword set minimal and allows user-defined
type aliases in the future.

**Comments**

Nib uses C++/Java/Rust-style line comments: ``//`` to end of line. The ``//``
sequence was chosen over ``#`` (Python/Ruby) because 4004 assembly
conventionally uses semicolons for comments — keeping ``//`` as the Nib
comment marker avoids confusion between the two syntaxes.

Locating the Grammar File
--------------------------

The ``nib.tokens`` file lives in the ``code/grammars/`` directory at the root
of the coding-adventures repository. We locate it relative to this module's
file path using ``pathlib.Path``::

    tokenizer.py
    └── nib_lexer/       (parent)
        └── src/         (parent)
            └── nib-lexer/ (parent)
                └── python/    (parent)
                    └── packages/ (parent)
                        └── code/     (parent)
                            └── grammars/
                                └── nib.tokens
"""

from __future__ import annotations

from pathlib import Path

from dataclasses import replace

from grammar_tools import parse_token_grammar
from lexer import GrammarLexer, Token
from lexer.tokenizer import TokenType

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------
#
# Navigate from this file's location up to the repository root's grammars/
# directory. The path is:
#   src/nib_lexer/tokenizer.py -> src/nib_lexer -> src -> nib-lexer
#   -> python -> packages -> code -> code/grammars
# ---------------------------------------------------------------------------

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
NIB_TOKENS_PATH = GRAMMAR_DIR / "nib.tokens"


def create_nib_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for Nib text.

    This function reads the ``nib.tokens`` file, parses it into a
    ``TokenGrammar``, and creates a ``GrammarLexer`` ready to tokenize
    the given source text.

    The lexer handles the following Nib-specific behaviors automatically:

    - **Keyword reclassification**: identifiers like ``fn``, ``let``, and
      ``return`` are reclassified from NAME to their keyword token kind after
      a full-token match. ``format`` stays NAME because the full token
      ``format`` does not match the keyword ``for``.
    - **Case sensitivity**: Nib keywords are lowercase only. ``FN`` stays NAME;
      only ``fn`` becomes the ``fn`` keyword.
    - **Multi-character operators**: ``+%`` matches before ``+``, ``+?`` before
      ``+``, ``..`` before any single-dot, ``->`` before ``-``, ``==`` before
      ``=``, ``!=`` before ``!``, ``<=`` before ``<``, ``>=`` before ``>``,
      ``&&`` before ``&``, ``||`` before ``|``.
    - **HEX_LIT priority**: ``0xFF`` is consumed as one hex literal before the
      plain decimal digit rule fires on the leading ``0``.
    - **Comment skipping**: ``// text`` to end of line is consumed silently.
    - **Whitespace skipping**: spaces, tabs, carriage returns, and newlines
      between tokens are all ignored.

    Args:
        source: The Nib source text to tokenize.

    Returns:
        A ``GrammarLexer`` instance configured with Nib token definitions.
        Call ``.tokenize()`` on it to get the token list.

    Raises:
        FileNotFoundError: If the ``nib.tokens`` file cannot be found.
        TokenGrammarError: If the ``.tokens`` file has syntax errors.

    Example::

        lexer = create_nib_lexer('fn add(a: u4, b: u4) -> u4 { return a +% b; }')
        tokens = lexer.tokenize()
    """
    grammar = parse_token_grammar(NIB_TOKENS_PATH.read_text())
    return GrammarLexer(source, grammar)


def tokenize_nib(source: str) -> list[Token]:
    """Tokenize Nib source text and return a list of tokens.

    This is the main entry point for the Nib lexer. Pass in a string of Nib
    source text, and get back a flat list of ``Token`` objects. The list
    always ends with an ``EOF`` token.

    Nib token kinds you will see:

    Multi-character operators (first-match ordering):
    - **WRAP_ADD** (``+%``) — wrapping addition: ``15 +% 1 = 0`` on u4
    - **SAT_ADD** (``+?``) — saturating addition: ``15 +? 1 = 15`` on u4
    - **RANGE** (``..``) — for-loop range: ``0..8``
    - **ARROW** (``->``) — return type: ``fn f() -> u4``
    - **EQ_EQ** (``==``) — equality comparison
    - **NEQ** (``!=``) — not-equal comparison
    - **LEQ** (``<=``), **GEQ** (``>=``) — comparison operators
    - **LAND** (``&&``), **LOR** (``||``) — short-circuit logical operators

    Single-character arithmetic:
    - **PLUS** (``+``), **MINUS** (``-``), **STAR** (``*``), **SLASH** (``/``)

    Single-character bitwise:
    - **AMP** (``&``), **PIPE** (``|``), **CARET** (``^``), **TILDE** (``~``)

    Comparison and logical:
    - **BANG** (``!``), **LT** (``<``), **GT** (``>``)

    Assignment:
    - **EQ** (``=``) — used only in declarations: ``let x: u4 = 5``

    Delimiters:
    - **LBRACE** (``{``), **RBRACE** (``}``), **LPAREN** (``(``), **RPAREN** (``)``
    - **COLON** (``:``) — type annotation separator
    - **SEMICOLON** (``;``) — statement terminator
    - **COMMA** (``,``) — argument/parameter separator

    Literals:
    - **HEX_LIT** — ``0xA``, ``0xFF``, ``0x1F`` (hex, case-insensitive digits)
    - **INT_LIT** — ``0``, ``42``, ``255`` (decimal integers)

    Identifiers:
    - **NAME** — identifiers and type names: ``counter``, ``u4``, ``u8``,
      ``bcd``, ``bool`` (types are NOT keywords — they are NAME tokens)

    Keywords (reclassified from NAME, lowercase only):
    - **fn**, **let**, **static**, **const**, **return**
    - **for**, **in**, **if**, **else**
    - **true**, **false**

    Skipped automatically:
    - Whitespace (spaces, tabs, carriage returns, newlines)
    - Line comments (``// text`` to end of line)

    Args:
        source: The Nib source text to tokenize.

    Returns:
        A list of ``Token`` objects. The last token is always EOF.

    Raises:
        FileNotFoundError: If the ``nib.tokens`` file cannot be found.
        LexerError: If the source contains characters that don't match
            any token pattern in the Nib grammar.

    Example::

        tokens = tokenize_nib('let x: u4 = 5;')
        # [Token('let', 'let'), Token(NAME, 'x'), Token(COLON, ':'),
        #  Token(NAME, 'u4'), Token(EQ, '='), Token(INT_LIT, '5'),
        #  Token(SEMICOLON, ';'), Token(EOF, '')]

    Example with wrapping arithmetic::

        tokens = tokenize_nib('15 +% 1')
        # [Token(INT_LIT, '15'), Token(WRAP_ADD, '+%'), Token(INT_LIT, '1'),
        #  Token(EOF, '')]
    """
    lexer = create_nib_lexer(source)
    raw = lexer.tokenize()

    # The GrammarLexer uses the keywords: section in nib.tokens, which causes
    # keyword tokens to come back with type=TokenType.KEYWORD and value="fn",
    # "let", etc.  Nib convention (and all downstream consumers) expect the
    # type to equal the lowercase keyword text — Token("fn", "fn"),
    # Token("let", "let"), etc.  This post-processing pass promotes
    # TokenType.KEYWORD → the value string as the type.
    return [
        replace(tok, type=tok.value) if tok.type is TokenType.KEYWORD else tok
        for tok in raw
    ]
