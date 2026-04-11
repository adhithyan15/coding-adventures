"""Dartmouth BASIC 1964 Lexer — tokenizes original BASIC source text.

This module is a thin wrapper around the generic ``GrammarLexer``. It loads
the ``dartmouth_basic.tokens`` file from the ``code/grammars/`` directory,
creates a lexer configured for 1964 Dartmouth BASIC, and applies two
post-tokenize hooks that handle the language's two quirky challenges:

  1. **LINE_NUM disambiguation** — bare integers serve as both line labels
     and numeric literals; we relabel by position.
  2. **REM suppression** — everything after a REM keyword until end-of-line
     is a comment and should be stripped from the token stream.

A Short History of Dartmouth BASIC
------------------------------------

In the spring of 1964, John Kemeny and Thomas Kurtz at Dartmouth College
created BASIC (Beginner's All-purpose Symbolic Instruction Code) with a
single goal: make computing accessible to every student, not just science
and math majors.

Their solution ran on a GE-225 mainframe connected to a room full of
teletype terminals. Students sat down, typed a program in plain English-like
syntax, and got results in seconds — an astonishing experience in 1964 when
most computers required batch-job submissions that took hours.

Key design decisions that shaped the language:

  - **Line numbers**: Every statement carries a numeric label (10, 20, 30
    ...). This let students type lines in any order and re-number them
    without retyping everything. It also provided a human-readable branch
    target: ``GOTO 100`` means exactly "go to the line labeled 100."

  - **Case insensitivity**: The GE-225 teletypes had no lowercase keys.
    Every character was uppercase. So the whole language was designed for
    uppercase-only input, and ``print``, ``Print``, and ``PRINT`` are
    identical.

  - **Pre-initialized variables**: Every variable starts at zero. No
    "undefined variable" errors for beginners to trip on.

  - **Simple type system**: Numbers are floating-point. Strings appear
    only in PRINT and DATA. No type declarations needed.

BASIC went on to become the most widely used programming language of the
1970s and 1980s. Microsoft's first product was a BASIC interpreter for the
Altair 8800 (1975). Apple sold BASIC in ROM. BASIC shipped on every home
computer from the Apple II to the Commodore 64 to the IBM PC.

The 1964 Dartmouth BASIC this lexer handles is the *original* — 20 keywords,
11 built-in functions, integer line numbers, no string variables. Future
dialects (Microsoft BASIC, Applesoft, GW-BASIC) extended it with string
variables, PEEK/POKE, ON GOTO, and many other features, but this lexer
targets only the 1964 specification.

The LINE_NUM Disambiguation
----------------------------

The trickiest part of BASIC lexing: bare integers appear in two roles:

  1. **Line label**:      ``10 LET X = 5``  — the ``10`` names the line
  2. **Numeric literal**: ``LET X = 42``    — the ``42`` is a value
  3. **GOTO target**:     ``GOTO 100``       — the ``100`` is a destination

The grammar file defines both LINE_NUM and NUMBER with an identical regex
``/[0-9]+/``. The lexer will always match NUMBER (since it comes second in
priority). The ``relabel_line_numbers`` post-hook fixes this: it walks the
completed token list and relabels the first NUMBER on each source line as
LINE_NUM.

The algorithm is beautifully simple::

    Walk tokens left to right.
    Maintain a boolean: am I at the start of a new line?
    Start: yes (column 0).
    On NEWLINE: the next token begins a new line.
    On any other token when at_line_start:
        if it is a NUMBER, relabel it LINE_NUM.
        then set at_line_start = False.

This means LINE_NUM can only appear once per line (the first token), which
matches the BASIC language rule exactly.

The REM Suppression
---------------------

In Dartmouth BASIC, ``REM`` introduces a remark (comment) that runs to
the end of the line. Everything after ``REM`` on the same physical line
should be invisible to the parser. For example::

    10 REM THIS IS A COMMENT
    20 LET X = 1

After tokenizing ``10 REM THIS IS A COMMENT``, the lexer produces:
  LINE_NUM("10"), KEYWORD("REM"), NAME("THIS"), KEYWORD("IS"), NAME("A"),
  KEYWORD("COMMENT"), NEWLINE

The ``suppress_rem_content`` hook strips everything between REM and NEWLINE,
leaving only:
  LINE_NUM("10"), KEYWORD("REM"), NEWLINE

The hook is careful to *keep* the NEWLINE (it is the statement terminator
that the parser needs) while suppressing all tokens in between.

Locating the Grammar File
--------------------------

The ``dartmouth_basic.tokens`` file lives in the ``code/grammars/`` directory
at the repository root. We navigate there relative to this file's path::

    tokenizer.py            (this file)
    └── dartmouth_basic_lexer/    (parent: package module)
        └── src/                  (parent: source root)
            └── dartmouth-basic-lexer/  (parent: package root)
                └── python/             (parent: language dir)
                    └── packages/       (parent: packages dir)
                        └── code/       (parent: code root)
                            └── grammars/
                                └── dartmouth_basic.tokens

That is six ``..`` steps from ``tokenizer.py`` to ``code/``, then into
``grammars/``.
"""

from __future__ import annotations

from pathlib import Path
from typing import Callable

from grammar_tools import parse_token_grammar
from lexer import GrammarLexer, Token

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------
#
# GRAMMAR_DIR resolves to: code/grammars/
#
# The six .parent steps navigate up the directory hierarchy:
#   __file__            → .../src/dartmouth_basic_lexer/tokenizer.py
#   .parent             → .../src/dartmouth_basic_lexer/
#   .parent             → .../src/
#   .parent             → .../dartmouth-basic-lexer/
#   .parent             → .../python/
#   .parent             → .../packages/
#   .parent             → .../code/
#   / "grammars"        → .../code/grammars/
#
# ---------------------------------------------------------------------------

GRAMMAR_DIR = (
    Path(__file__).parent  # dartmouth_basic_lexer/
    .parent                # src/
    .parent                # dartmouth-basic-lexer/
    .parent                # python/
    .parent                # packages/
    .parent                # code/
    / "grammars"
)

DARTMOUTH_BASIC_TOKENS_PATH = GRAMMAR_DIR / "dartmouth_basic.tokens"


# ---------------------------------------------------------------------------
# Post-Tokenize Hook 1: relabel_line_numbers
# ---------------------------------------------------------------------------
#
# This hook solves the LINE_NUM vs. NUMBER ambiguity described in the module
# docstring. It runs *after* the grammar lexer has produced its initial token
# list, and relabels the first NUMBER on each source line as LINE_NUM.
#
# Example transformation:
#
#   BEFORE:  NUMBER("10"), KEYWORD("LET"), NAME("X"), EQ("="), NUMBER("5"), NEWLINE
#   AFTER:   LINE_NUM("10"), KEYWORD("LET"), NAME("X"), EQ("="), NUMBER("5"), NEWLINE
#
# The hook uses a simple flag ``at_line_start`` to track position:
#   - True at the very beginning of the token stream (position 0).
#   - True immediately after a NEWLINE token.
#   - False for all other positions.
#
# When at_line_start is True and the current token is a NUMBER, we create
# a new Token with type "LINE_NUM" and identical value/line/column.
#
# ---------------------------------------------------------------------------


def relabel_line_numbers(tokens: list[Token]) -> list[Token]:
    """Relabel the first NUMBER token on each source line as LINE_NUM.

    In Dartmouth BASIC, every statement begins with a line number:

        10 LET X = 5
        20 PRINT X
        30 END

    The grammar cannot distinguish a line number from a numeric literal
    by regex alone — both are sequences of digits. This hook walks the
    completed token list and relabels the first NUMBER on each line as
    LINE_NUM based on its position (at the start of a line).

    The hook is position-sensitive:
      - The very first token in the stream is at line start.
      - Any token immediately following a NEWLINE is at line start.
      - A NUMBER at line start → relabeled LINE_NUM.
      - Any other token type at line start → left unchanged.

    Args:
        tokens: The raw token list from the GrammarLexer.

    Returns:
        A new token list with appropriate NUMBER tokens relabeled as LINE_NUM.

    Example::

        # Input:  "10 LET X = 5\\n"
        # Before: NUMBER("10"), KEYWORD("LET"), NAME("X"), EQ("="), NUMBER("5"), NEWLINE, EOF
        # After:  LINE_NUM("10"), KEYWORD("LET"), NAME("X"), EQ("="), NUMBER("5"), NEWLINE, EOF
    """
    # We start at "line start" — the very first token in any BASIC program
    # should be the line number of line 1.
    at_line_start = True
    result: list[Token] = []

    for token in tokens:
        # Determine what type name to compare against. Token.type can be
        # either a string or a TokenType enum value; normalize to a string.
        token_type: str = token.type if isinstance(token.type, str) else token.type.name

        if at_line_start and token_type == "NUMBER":
            # This NUMBER is in line-number position. Relabel it LINE_NUM.
            # We construct a new Token rather than mutating — Tokens are
            # dataclasses and may be immutable in some implementations.
            token = Token(
                type="LINE_NUM",
                value=token.value,
                line=token.line,
                column=token.column,
            )
            # We have consumed the line-start position; all subsequent
            # tokens on this line are NOT at line start.
            at_line_start = False

        elif at_line_start:
            # Something other than a NUMBER at line start (e.g., an empty
            # line with only whitespace, or a bare NEWLINE). Don't relabel;
            # just mark that we are no longer at line start.
            at_line_start = False

        # NEWLINE marks the end of a statement. The NEXT token (if any)
        # will be the first token on a new line, so set at_line_start = True.
        if token_type == "NEWLINE":
            at_line_start = True

        result.append(token)

    return result


# ---------------------------------------------------------------------------
# Post-Tokenize Hook 2: suppress_rem_content
# ---------------------------------------------------------------------------
#
# In Dartmouth BASIC, ``REM`` introduces a remark that runs to end-of-line.
# The grammar lexer does not have a "consume until newline" mode, so the
# lexer tokenizes the comment text as normal tokens (NAME, NUMBER, etc.).
# This hook strips those tokens, leaving only:
#   KEYWORD("REM"), NEWLINE
#
# Suppression algorithm:
#   - Walk tokens left to right.
#   - When suppressing == False: emit the token normally.
#   - After emitting a KEYWORD("REM") token: start suppressing.
#   - When suppressing == True and token type is NEWLINE: stop suppressing
#     and emit the NEWLINE (it is the statement terminator the parser needs).
#
# Crucially, we emit the NEWLINE even after suppression. The parser relies
# on NEWLINE tokens as statement terminators. Without it, the parser would
# see the next line's LINE_NUM immediately following REM, which would break
# the grammar.
#
# Example:
#   Input tokens:  LINE_NUM("10"), KEYWORD("REM"), NAME("THIS"), NAME("IS"),
#                  NAME("A"), NAME("COMMENT"), NEWLINE, LINE_NUM("20"), ...
#   Output tokens: LINE_NUM("10"), KEYWORD("REM"), NEWLINE, LINE_NUM("20"), ...
#
# ---------------------------------------------------------------------------


def suppress_rem_content(tokens: list[Token]) -> list[Token]:
    """Remove all tokens between a REM keyword and the next NEWLINE.

    In Dartmouth BASIC, the REM statement introduces a remark (comment)
    that runs to the end of the physical line. Everything after REM on the
    same line is documentation for the human reader and should be invisible
    to the parser.

    This hook suppresses all tokens between a KEYWORD("REM") and the next
    NEWLINE. The NEWLINE itself is kept because it serves as the statement
    terminator that the parser needs to advance to the next program line.

    Args:
        tokens: The token list (after relabel_line_numbers has already run).

    Returns:
        A new token list with all REM comment content removed.

    Example::

        # Input:  "10 REM THIS IS A COMMENT\\n20 LET X = 1\\n"
        # Tokens before hook:
        #   LINE_NUM("10"), KEYWORD("REM"), NAME("THIS"), KEYWORD("IS"),
        #   NAME("A"), KEYWORD("COMMENT"), NEWLINE,
        #   LINE_NUM("20"), KEYWORD("LET"), NAME("X"), EQ, NUMBER("1"), NEWLINE, EOF
        # Tokens after hook:
        #   LINE_NUM("10"), KEYWORD("REM"), NEWLINE,
        #   LINE_NUM("20"), KEYWORD("LET"), NAME("X"), EQ, NUMBER("1"), NEWLINE, EOF
    """
    result: list[Token] = []
    suppressing = False

    for token in tokens:
        token_type: str = token.type if isinstance(token.type, str) else token.type.name

        if not suppressing:
            # Not suppressing — emit this token normally.
            result.append(token)

        # Check what this token means for our suppression state.
        if token_type == "KEYWORD" and token.value == "REM":
            # The token we just emitted was REM. Start suppressing everything
            # that follows until (but not including) the next NEWLINE.
            suppressing = True
        elif token_type == "NEWLINE":
            # A NEWLINE ends the REM comment (if we were suppressing) and
            # also ends a normal statement line. In either case, we stop
            # suppressing after this point.
            # Note: if suppressing was True, the NEWLINE itself was NOT
            # emitted above (because we only emit when suppressing == False).
            # We add it now to ensure the parser always sees statement-end.
            if suppressing:
                result.append(token)
            suppressing = False

    return result


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def create_dartmouth_basic_lexer(source: str) -> GrammarLexer:
    """Create a ``GrammarLexer`` configured for 1964 Dartmouth BASIC.

    This function reads ``dartmouth_basic.tokens``, parses it into a
    ``TokenGrammar``, and constructs a ``GrammarLexer`` for the given source.

    **Note**: This function does NOT attach the post-tokenize hooks. Use
    ``tokenize_dartmouth_basic`` for the complete lexing pipeline including
    LINE_NUM relabeling and REM suppression. Use this function only when you
    need direct access to the ``GrammarLexer`` object (e.g., to add your own
    hooks or inspect the grammar).

    The grammar handles the following Dartmouth BASIC behaviors:

    - **Case insensitivity**: ``@case_insensitive true`` in the grammar means
      the source is uppercased before matching. ``print``, ``Print``, and
      ``PRINT`` all produce ``KEYWORD("PRINT")``.

    - **Keyword boundary enforcement**: The ``keywords:`` section in the
      grammar file lists all 20 reserved words. The grammar engine matches
      the full identifier token and only then checks the keyword list. This
      means ``FOREST`` is never mistakenly tokenized as ``FOR`` + ``EST``.

    - **Multi-char operators first**: ``<=``, ``>=``, ``<>`` appear before
      ``<``, ``>``, ``=`` in the grammar, so they match as single LE/GE/NE
      tokens rather than two separate tokens each.

    - **Scientific notation**: ``1.5E3`` matches as one NUMBER token. The
      regex in the grammar greedily matches the exponent part.

    Args:
        source: The Dartmouth BASIC source text to tokenize.

    Returns:
        A ``GrammarLexer`` instance with the Dartmouth BASIC grammar loaded.
        Call ``.tokenize()`` to get the raw token list (without post-hooks).
        Call ``.add_post_tokenize(hook)`` to add your own transformations.

    Raises:
        FileNotFoundError: If ``dartmouth_basic.tokens`` cannot be found at
            the expected path.
        TokenGrammarError: If the tokens file has syntax errors.

    Example::

        lexer = create_dartmouth_basic_lexer("10 PRINT X\\n")
        # Add custom hook:
        lexer.add_post_tokenize(my_hook)
        tokens = lexer.tokenize()
    """
    grammar = parse_token_grammar(DARTMOUTH_BASIC_TOKENS_PATH.read_text())
    return GrammarLexer(source, grammar)


def tokenize_dartmouth_basic(source: str) -> list[Token]:
    """Tokenize 1964 Dartmouth BASIC source text and return a list of tokens.

    This is the main entry point for the Dartmouth BASIC lexer. Pass in a
    string of BASIC source text, and get back a flat list of ``Token`` objects.
    The list always ends with an ``EOF`` token.

    This function applies two post-tokenize hooks in order:

    1. **relabel_line_numbers**: The first NUMBER on each source line is
       relabeled as LINE_NUM. This distinguishes line labels from numeric
       literals in expressions.

    2. **suppress_rem_content**: All tokens between a KEYWORD("REM") and the
       next NEWLINE are removed. The NEWLINE is kept. This implements BASIC's
       comment syntax.

    Token types you will see in the output:

    **Source structure**:
    - ``LINE_NUM``  — Integer at the start of a line (e.g., ``10``, ``999``)
    - ``NEWLINE``   — Line ending ``\\n`` or ``\\r\\n``; marks end of statement
    - ``EOF``       — Always the last token

    **Values**:
    - ``NUMBER``    — Numeric literal in an expression (``42``, ``3.14``, ``1.5E3``)
    - ``STRING``    — Double-quoted string (``"HELLO WORLD"``, includes quotes)
    - ``NAME``      — Variable name: one letter + optional digit (``X``, ``A1``)

    **Keywords** (always uppercase, case-insensitive input):
    - ``KEYWORD``   — One of 20 reserved words: LET, PRINT, INPUT, IF, THEN,
                      GOTO, GOSUB, RETURN, FOR, TO, STEP, NEXT, END, STOP,
                      REM, READ, DATA, RESTORE, DIM, DEF

    **Functions**:
    - ``BUILTIN_FN`` — One of 11 built-ins: SIN, COS, TAN, ATN, EXP, LOG,
                       ABS, SQR, INT, RND, SGN
    - ``USER_FN``   — User-defined function: FN + one letter (FNA through FNZ)

    **Operators**:
    - ``PLUS``, ``MINUS``, ``STAR``, ``SLASH``, ``CARET`` — arithmetic
    - ``EQ``, ``LT``, ``GT``, ``LE``, ``GE``, ``NE`` — comparison
    - ``LPAREN``, ``RPAREN``, ``COMMA``, ``SEMICOLON`` — punctuation

    **Error recovery**:
    - ``UNKNOWN``   — Unrecognized character; lexer continues after emitting

    Args:
        source: The Dartmouth BASIC source text to tokenize. May be a single
            line or an entire program with multiple newline-separated lines.

    Returns:
        A list of ``Token`` objects. The last token is always ``EOF``.
        NEWLINE tokens are included (they are significant in line-oriented
        BASIC). Whitespace and REM comment content are not included.

    Raises:
        FileNotFoundError: If ``dartmouth_basic.tokens`` cannot be found.
        TokenGrammarError: If the tokens file has syntax errors.

    Example::

        tokens = tokenize_dartmouth_basic("10 LET X = 5\\n20 PRINT X\\n30 END\\n")
        # [Token(LINE_NUM, "10"), Token(KEYWORD, "LET"), Token(NAME, "X"),
        #  Token(EQ, "="), Token(NUMBER, "5"), Token(NEWLINE, "\\n"),
        #  Token(LINE_NUM, "20"), Token(KEYWORD, "PRINT"), Token(NAME, "X"),
        #  Token(NEWLINE, "\\n"), Token(LINE_NUM, "30"), Token(KEYWORD, "END"),
        #  Token(NEWLINE, "\\n"), Token(EOF, "")]

    REM comment example::

        tokens = tokenize_dartmouth_basic("10 REM HELLO\\n20 LET X = 1\\n")
        # [Token(LINE_NUM, "10"), Token(KEYWORD, "REM"), Token(NEWLINE, "\\n"),
        #  Token(LINE_NUM, "20"), Token(KEYWORD, "LET"), Token(NAME, "X"),
        #  Token(EQ, "="), Token(NUMBER, "1"), Token(NEWLINE, "\\n"), Token(EOF, "")]
        # Note: "HELLO" is suppressed by the REM hook.
    """
    lexer = create_dartmouth_basic_lexer(source)
    lexer.add_post_tokenize(relabel_line_numbers)
    lexer.add_post_tokenize(suppress_rem_content)
    return lexer.tokenize()
