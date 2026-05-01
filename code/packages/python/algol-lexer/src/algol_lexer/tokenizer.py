"""ALGOL 60 Lexer — tokenizes ALGOL 60 source text using the grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarLexer``. It loads
the ``algol.tokens`` file from the ``code/grammars/`` directory and creates a
lexer configured for ALGOL 60 tokenization.

A Short History of ALGOL 60
-----------------------------

ALGOL (ALGOrithmic Language) was designed by an international committee between
1958 and 1960. The resulting language report — the *Revised Report on the
Algorithmic Language ALGOL 60* — changed computing forever. Its contributions:

1. **BNF (Backus-Naur Form)**: ALGOL 60 was the first language defined using
   formal grammar notation, invented by John Backus and refined by Peter Naur.
   Every compiler theory textbook still teaches parsing using the notation ALGOL
   introduced.

2. **Block structure and lexical scoping**: Variables are local to the block
   (``begin``...``end``) in which they are declared. This is the direct ancestor
   of how Python, Java, C, and every modern language scopes variables.

3. **Recursion**: ALGOL was the first commercially-relevant language to support
   recursive procedures. This required inventing the call stack — a data
   structure every modern CPU has built in.

4. **Call-by-name semantics**: Parameters were passed by *name* by default,
   meaning the argument expression was re-evaluated every time the parameter was
   used inside the procedure. Jensen's device (using call-by-name to simulate
   summation over a formula) became a famous programming idiom.

5. **Dynamic arrays**: Array bounds could be arbitrary arithmetic expressions
   evaluated at runtime — a feature C still lacks in the base language.

ALGOL 60's family tree:

    ALGOL 60
    ├── ALGOL 68          (generalized ALGOL, 1968)
    ├── Pascal (Wirth)    (simplified ALGOL for teaching, 1970)
    │   └── Modula-2, Oberon, Delphi
    ├── Simula (Dahl, Nygaard) — first OOP language (1967)
    │   └── C++ → Java → C# → Kotlin → Swift → Rust → Go
    └── CPL → BCPL → B → C (Ritchie)
        └── Everything else

What This Module Provides
--------------------------

Two convenience functions:

- ``create_algol_lexer(source)`` — creates a ``GrammarLexer`` configured for
  ALGOL 60. Use this when you want to control tokenization yourself.
- ``tokenize_algol(source)`` — the all-in-one function. Pass in ALGOL 60 text,
  get back a list of tokens. This is the function most callers want.

Token Types
-----------

The lexer produces the following token kinds (defined in ``algol.tokens``):

Value tokens:
- ``REAL_LIT``    — floating-point literals: ``3.14``, ``1.5E3``, ``1.5E-3``
- ``INTEGER_LIT`` — integer literals: ``42``, ``0``, ``1000``
- ``STRING_LIT``  — quoted strings: ``'hello world'`` or ``"hello world"``
- ``IDENT``       — identifiers (reclassified as keywords when applicable)

Multi-character operators (must match before their single-char prefixes):
- ``ASSIGN``  — ``:=``  (ALGOL separates assignment from equality — no C-style bugs)
- ``POWER``   — ``**``  (exponentiation, Fortran convention)
- ``LEQ``     — ``<=``
- ``GEQ``     — ``>=``
- ``NEQ``     — ``!=`` or ``<>``

Single-character operators:
- ``PLUS``, ``MINUS``, ``STAR``, ``SLASH``, ``CARET``, ``EQ``, ``LT``, ``GT``

Delimiters:
- ``LPAREN``, ``RPAREN``, ``LBRACKET``, ``RBRACKET``
- ``SEMICOLON``, ``COMMA``, ``COLON``

Keywords (reclassified from IDENT after full-token match — case-insensitive):
- Block:   ``BEGIN``, ``END``
- Control: ``IF``, ``THEN``, ``ELSE``, ``FOR``, ``DO``, ``STEP``, ``UNTIL``,
           ``WHILE``, ``GOTO``
- Decl:    ``SWITCH``, ``PROCEDURE``, ``OWN``, ``ARRAY``, ``LABEL``, ``VALUE``
- Types:   ``INTEGER``, ``REAL``, ``BOOLEAN``, ``STRING``
- Literals: ``TRUE``, ``FALSE``
- Boolean: ``NOT``, ``AND``, ``OR``, ``IMPL``, ``EQV``
- Arithmetic: ``DIV``, ``MOD``

Skipped (not emitted):
- ``WHITESPACE`` — spaces, tabs, newlines (ALGOL is free-format)
- ``COMMENT``    — ``comment text;`` up to and including the next semicolon

Comment Syntax
--------------

ALGOL 60 uses a distinctive comment syntax::

    comment this is the comment text;

The word ``comment`` is matched case-insensitively and triggers comment-skip
mode: everything from that word up to (and including) the next ``;`` is consumed
silently. This means a ``comment`` appearing after a statement-terminating
semicolon skips the rest of the line::

    x := 42; comment set x to 42;
    y := x + 1

The second semicolon ends both the comment and serves as the logical separator.
Identifiers that merely start with those letters, such as ``commentary``, stay
ordinary identifiers.

Why ALGOL Uses := for Assignment
-----------------------------------

ALGOL chose ``:=`` for assignment and ``=`` for equality. This avoids the
notorious C bug::

    if (x = 1) ...  # C: assigns 1 to x, always true (in C, but error in ALGOL)
    if x = 1 then   # ALGOL: equality test (can't assign in a condition)

Every language since has had to choose: Python, Go, and Rust follow ALGOL
(using ``=`` for assignment but making the assignment-in-condition either
impossible or a compile warning). C chose differently and generations of
programmers wrote ``==`` when they meant ``=``.

Locating the Grammar File
--------------------------

The ``algol.tokens`` file lives in the ``code/grammars/`` directory at the
root of the coding-adventures repository. We locate it relative to this
module's file path using ``pathlib.Path``::

    tokenizer.py
    └── algol_lexer/       (parent)
        └── src/           (parent)
            └── algol-lexer/ (parent)
                └── python/    (parent)
                    └── packages/ (parent)
                        └── code/     (parent)
                            └── grammars/
                                └── algol.tokens
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_token_grammar
from lexer import GrammarLexer, Token, TokenType

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------
#
# Navigate from this file's location up to the repository root's grammars/
# directory. The path is:
#   src/algol_lexer/tokenizer.py -> src/algol_lexer -> src -> algol-lexer
#   -> python -> packages -> code -> code/grammars
# ---------------------------------------------------------------------------

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
VALID_VERSIONS = {"algol60"}


def resolve_tokens_path(version: str = "algol60") -> Path:
    """Resolve a supported ALGOL token grammar path."""
    if version not in VALID_VERSIONS:
        valid = ", ".join(sorted(VALID_VERSIONS))
        raise ValueError(f"Unknown ALGOL version {version!r}. Valid versions: {valid}")
    return GRAMMAR_DIR / "algol" / f"{version}.tokens"


def _normalize_case_insensitive_keywords(
    tokens: list[Token],
    keywords: set[str],
) -> list[Token]:
    """Promote ALGOL keywords without lowercasing the whole source stream."""
    normalized: list[Token] = []
    for token in tokens:
        value = token.value.lower()
        if token.type in (TokenType.NAME, "NAME") and value in keywords:
            normalized.append(
                Token(
                    type=TokenType.KEYWORD,
                    value=value,
                    line=token.line,
                    column=token.column,
                    flags=token.flags,
                )
            )
        elif token.type in (TokenType.KEYWORD, "KEYWORD") and value in keywords:
            normalized.append(
                Token(
                    type=token.type,
                    value=value,
                    line=token.line,
                    column=token.column,
                    flags=token.flags,
                )
            )
        else:
            normalized.append(token)
    return normalized


def create_algol_lexer(source: str, version: str = "algol60") -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for ALGOL 60 text.

    This function reads the ``algol.tokens`` file, parses it into a
    ``TokenGrammar``, and creates a ``GrammarLexer`` ready to tokenize
    the given source text.

    The lexer handles the following ALGOL-specific behaviors automatically:

    - **Keyword reclassification**: identifiers like ``begin``, ``if``, and
      ``integer`` are reclassified from IDENT to their keyword token kind
      after a full-token match. ``beginning`` stays IDENT because the full
      token ``beginning`` does not match the keyword ``begin``.
    - **Case insensitivity**: ``BEGIN``, ``Begin``, and ``begin`` all produce
      the same token kind. The grammar normalizes to lowercase.
    - **Comment skipping**: ``comment text;`` is consumed without emitting
      any token, and the keyword is matched case-insensitively.
    - **Operator ordering**: ``:=`` is matched before ``:``, ``**`` before
      ``*``, ``<=`` before ``<``, ``>=`` before ``>``.

    Args:
        source: The ALGOL 60 text to tokenize.

    Returns:
        A ``GrammarLexer`` instance configured with ALGOL 60 token definitions.
        Call ``.tokenize()`` on it to get the token list.

    Raises:
        FileNotFoundError: If the ``algol.tokens`` file cannot be found.
        TokenGrammarError: If the ``.tokens`` file has syntax errors.

    Example::

        lexer = create_algol_lexer('begin integer x; x := 42 end')
        tokens = lexer.tokenize()
    """
    grammar = parse_token_grammar(resolve_tokens_path(version).read_text())
    lexer = GrammarLexer(source, grammar)
    keyword_set = {keyword.lower() for keyword in grammar.keywords}
    lexer.add_post_tokenize(
        lambda tokens: _normalize_case_insensitive_keywords(tokens, keyword_set)
    )
    return lexer


def tokenize_algol(source: str, version: str = "algol60") -> list[Token]:
    """Tokenize ALGOL 60 text and return a list of tokens.

    This is the main entry point for the ALGOL 60 lexer. Pass in a string of
    ALGOL 60 source text, and get back a flat list of ``Token`` objects. The
    list always ends with an ``EOF`` token.

    ALGOL 60 token kinds you will see:

    Value tokens:
    - **REAL_LIT**    — floating-point: ``3.14``, ``1.5E3``, ``100E2``
    - **INTEGER_LIT** — integer: ``42``, ``0``, ``1000``
    - **STRING_LIT**  — quoted: ``'hello world'`` or ``"hello world"``
    - **IDENT**       — identifiers not matching any keyword

    Operators:
    - **ASSIGN** (``:=``), **POWER** (``**``), **CARET** (``^``)
    - **LEQ** (``<=``), **GEQ** (``>=``), **NEQ** (``!=`` or ``<>``)
    - **PLUS**, **MINUS**, **STAR**, **SLASH**, **EQ**, **LT**, **GT**

    Delimiters:
    - **LPAREN**, **RPAREN**, **LBRACKET**, **RBRACKET**
    - **SEMICOLON**, **COMMA**, **COLON**

    Keywords (case-insensitive):
    - **BEGIN**, **END**, **IF**, **THEN**, **ELSE**, **FOR**, **DO**
    - **STEP**, **UNTIL**, **WHILE**, **GOTO**
    - **SWITCH**, **PROCEDURE**, **OWN**, **ARRAY**, **LABEL**, **VALUE**
    - **INTEGER**, **REAL**, **BOOLEAN**, **STRING**
    - **TRUE**, **FALSE**
    - **NOT**, **AND**, **OR**, **IMPL**, **EQV**
    - **DIV**, **MOD**

    Skipped automatically:
    - Whitespace (spaces, tabs, newlines)
    - Comments (``comment text;``)

    Args:
        source: The ALGOL 60 text to tokenize.

    Returns:
        A list of ``Token`` objects. The last token is always EOF.

    Raises:
        FileNotFoundError: If the ``algol.tokens`` file cannot be found.
        LexerError: If the source contains characters that don't match
            any token pattern in the ALGOL 60 grammar.

    Example::

        tokens = tokenize_algol('begin integer x; x := 42 end')
        # [Token(BEGIN, 'begin'), Token(INTEGER, 'integer'),
        #  Token(IDENT, 'x'), Token(SEMICOLON, ';'),
        #  Token(IDENT, 'x'), Token(ASSIGN, ':='),
        #  Token(INTEGER_LIT, '42'), Token(END, 'end'),
        #  Token(EOF, '')]
    """
    lexer = create_algol_lexer(source, version=version)
    return lexer.tokenize()
