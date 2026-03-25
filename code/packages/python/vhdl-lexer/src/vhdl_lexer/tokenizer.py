"""VHDL Lexer — tokenizes VHDL source code using the grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarLexer``. It
loads the ``vhdl.tokens`` grammar file and applies VHDL-specific
post-processing: **case normalization**.

Case Normalization
------------------

VHDL is case-insensitive. The IEEE 1076 standard says that basic
identifiers and keywords are equivalent regardless of case:

    ENTITY counter IS      -- equivalent to:
    entity counter is      -- this

To normalize, we lowercase the ``value`` field of every token whose
type is ``NAME`` or ``KEYWORD`` *after* tokenization. This is a
post-processing step rather than a pre-processing step because:

1. String literals must preserve their original case ("Hello" stays "Hello").
2. Extended identifiers (``\\My Name\\``) preserve case by definition.
3. Only NAME and KEYWORD tokens are case-insensitive.

Why Post-Process Instead of Pre-Process?
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

A pre-processing approach (lowercasing the entire source string) would
destroy case information in string literals and character literals.
By lowercasing only after the lexer has identified token boundaries,
we preserve the original case in strings while normalizing identifiers.

No Preprocessor
---------------

Unlike Verilog (which has `` `define ``, `` `ifdef ``, etc.), VHDL has
**no preprocessor**. All configuration in VHDL is done through:

- ``generic`` parameters (compile-time constants)
- ``generate`` statements (conditional/iterative structure)
- ``configuration`` declarations (binding components to architectures)

These are all first-class language constructs that the parser handles,
not text-level transformations that the lexer must deal with.

Locating the Grammar File
--------------------------

The ``vhdl.tokens`` file lives in the ``code/grammars/`` directory::

    tokenizer.py
    └── vhdl_lexer/     (parent 1)
        └── src/            (parent 2)
            └── vhdl-lexer/   (parent 3)
                └── python/       (parent 4)
                    └── packages/     (parent 5)
                        └── code/         (parent 6)
                            └── grammars/
                                └── vhdl.tokens
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_token_grammar
from lexer import GrammarLexer, Token, TokenType

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------
#
# We navigate six parent levels from this file (tokenizer.py) to reach
# the ``code/`` directory, then descend into ``grammars/``.

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
VHDL_TOKENS_PATH = GRAMMAR_DIR / "vhdl.tokens"


# ---------------------------------------------------------------------------
# Case Normalization
# ---------------------------------------------------------------------------
#
# VHDL's case insensitivity is one of its defining characteristics,
# inherited from Ada. The normalization function walks the token list
# and lowercases NAME and KEYWORD values. It returns a new list (no
# mutation of the originals) to avoid surprising side effects.
#
# Token types can be either:
#   - A ``TokenType`` enum value (has a ``.name`` attribute)
#   - A plain string (custom types from the ``.tokens`` file)
#
# We check for both when determining the token type name.


def _normalize_case(tokens: list[Token], keywords: set[str]) -> list[Token]:
    """Lowercase the value of NAME and KEYWORD tokens, and reclassify keywords.

    This implements VHDL's case-insensitivity rule. After this step,
    ``ENTITY``, ``Entity``, and ``entity`` all have value ``"entity"``
    and type ``KEYWORD``.

    Because the grammar engine matches keywords case-sensitively (comparing
    against lowercase entries in the ``.tokens`` file), an uppercase token
    like ``ENTITY`` is initially classified as ``NAME``. This function
    fixes that by checking whether the lowercased value is in the keyword
    set and promoting ``NAME`` to ``KEYWORD`` when it matches.

    Args:
        tokens: The raw token list from the grammar lexer.
        keywords: The set of lowercase keyword strings from the grammar.

    Returns:
        A new list with NAME/KEYWORD values lowercased and NAME tokens
        reclassified as KEYWORD when appropriate. Other token types
        (STRING, NUMBER, CHAR_LITERAL, etc.) are unchanged.

    Example::

        # Before normalization:
        #   Token(NAME, 'ENTITY', ...)     -- uppercase, misclassified
        #   Token(NAME, 'Counter', ...)
        #
        # After normalization:
        #   Token(KEYWORD, 'entity', ...)  -- reclassified + lowercased
        #   Token(NAME, 'counter', ...)    -- lowercased only
    """
    result: list[Token] = []
    for token in tokens:
        # Determine the type name -- could be an enum or a plain string.
        type_name = token.type.name if hasattr(token.type, "name") else token.type

        if type_name in ("NAME", "KEYWORD"):
            lowered = token.value.lower()
            # Determine the correct token type: if the lowercased value
            # is a keyword, use TokenType.KEYWORD; otherwise keep the
            # original type (NAME stays NAME, KEYWORD stays KEYWORD).
            if lowered in keywords:
                new_type = TokenType.KEYWORD
            else:
                new_type = token.type
            # Create a new Token with the lowercased value and correct type.
            # Token is a frozen dataclass with fields: type, value, line, column.
            result.append(Token(new_type, lowered, token.line, token.column))
        else:
            result.append(token)
    return result


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def create_vhdl_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for VHDL source code.

    Args:
        source: The VHDL source code to tokenize.

    Returns:
        A ``GrammarLexer`` instance configured with VHDL token definitions.

    Example::

        lexer = create_vhdl_lexer('entity e is end entity e;')
        tokens = lexer.tokenize()

    Note
    ----
    This returns the raw lexer without case normalization. Use
    ``tokenize_vhdl()`` for the full pipeline (tokenization + normalization).
    If you need the raw tokens (preserving original case), use this function
    and call ``lexer.tokenize()`` directly.
    """
    grammar = parse_token_grammar(VHDL_TOKENS_PATH.read_text())
    return GrammarLexer(source, grammar)


def _load_keywords() -> set[str]:
    """Load the keyword set from the VHDL grammar file.

    This is called once and cached in a module-level variable so we
    don't re-parse the grammar file on every call to ``tokenize_vhdl()``.
    """
    grammar = parse_token_grammar(VHDL_TOKENS_PATH.read_text())
    return set(grammar.keywords)


# Cache the keyword set at module load time.
_VHDL_KEYWORDS: set[str] = _load_keywords()


def tokenize_vhdl(source: str) -> list[Token]:
    """Tokenize VHDL source code and return a list of normalized tokens.

    This is the main entry point for the VHDL lexer. It performs two steps:

    1. **Tokenize** -- run the grammar-driven lexer to produce raw tokens.
    2. **Normalize** -- lowercase NAME and KEYWORD values for case insensitivity,
       and reclassify NAME tokens as KEYWORD when the lowercased value matches
       a VHDL keyword (e.g., ``ENTITY`` -> ``KEYWORD("entity")``).

    The list always ends with an ``EOF`` token.

    Args:
        source: The VHDL source code to tokenize.

    Returns:
        A list of ``Token`` objects with normalized case.

    Example::

        tokens = tokenize_vhdl('''
            entity and_gate is
                port(a, b : in std_logic; y : out std_logic);
            end entity and_gate;
        ''')
        # All keywords and names are lowercase:
        # [Token(KEYWORD, 'entity', ...), Token(NAME, 'and_gate', ...),
        #  Token(KEYWORD, 'is', ...), ...]
    """
    lexer = create_vhdl_lexer(source)
    raw_tokens = lexer.tokenize()
    return _normalize_case(raw_tokens, _VHDL_KEYWORDS)
