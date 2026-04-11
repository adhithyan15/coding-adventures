"""Dartmouth BASIC 1964 Parser — parses BASIC source into ASTs using the grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarParser``. It loads
the ``dartmouth_basic.grammar`` file from the ``code/grammars/`` directory,
tokenizes the input using the Dartmouth BASIC lexer, and produces a generic
``ASTNode`` tree.

What Is 1964 Dartmouth BASIC?
------------------------------

BASIC (Beginner's All-purpose Symbolic Instruction Code) was created by John
Kemeny and Thomas Kurtz at Dartmouth College in 1964. Their goal was radical:
make time-shared computing available to every student on campus, not just
computer science majors. A student should be able to write a useful program in
an afternoon.

The language they designed had 20 keywords and 11 built-in math functions. Every
statement lived on a numbered line:

    10 LET X = 5
    20 PRINT X * X
    30 END

This numbered-line structure served two purposes: it let students edit specific
lines by retyping them (no text editors existed!), and it gave GOTO/GOSUB a
target to jump to. The DTSS system (Dartmouth Time-Sharing System) allowed 30
students to use the same GE-225 mainframe simultaneously via teletypes.

The influence is enormous: Microsoft BASIC (1975), Applesoft BASIC (1977),
Commodore BASIC (1977), and the BBC Micro's BASIC (1981) all descended from
this original design. A generation of programmers learned to code in BASIC.

What the Grammar-Driven Parser Approach Means
----------------------------------------------

A traditional parser is handwritten for a specific language: the JSON parser
knows about { and [ and :, the Python parser knows about def and if and indent.
Each language requires its own parser codebase.

The grammar-driven approach inverts this: instead of writing parser code, we
write a grammar specification. The same generic parser engine reads the grammar
and parses any conforming input. Add a new language? Write a grammar file — no
new parser code needed.

The grammar uses EBNF (Extended Backus-Naur Form) notation::

    program = { line } ;
    line    = LINE_NUM [ statement ] NEWLINE ;
    statement = let_stmt | print_stmt | ... ;
    let_stmt  = "LET" variable EQ expr ;
    expr      = term { ( PLUS | MINUS ) term } ;

This file defines the entire syntax of 1964 Dartmouth BASIC. The same file
drives parsers in Python, Ruby, Go, Rust, TypeScript — one grammar, many
implementations.

How to Read the AST Output
----------------------------

The parser produces ``ASTNode`` objects. Each node records which grammar rule
matched (``rule_name``) and what it matched (``children`` — a list of tokens
and other ``ASTNode`` objects).

For example, parsing ``"10 LET X = 5\\n"`` produces::

    ASTNode(rule_name="program", children=[
        ASTNode(rule_name="line", children=[
            Token(LINE_NUM, '10'),
            ASTNode(rule_name="statement", children=[
                ASTNode(rule_name="let_stmt", children=[
                    Token(KEYWORD, 'LET'),
                    ASTNode(rule_name="variable", children=[
                        Token(NAME, 'X')
                    ]),
                    Token(EQ, '='),
                    ASTNode(rule_name="expr", children=[
                        ASTNode(rule_name="term", children=[
                            ASTNode(rule_name="power", children=[
                                ASTNode(rule_name="unary", children=[
                                    ASTNode(rule_name="primary", children=[
                                        Token(NUMBER, '5')
                                    ])
                                ])
                            ])
                        ])
                    ])
                ])
            ]),
            Token(NEWLINE, '\\n')
        ])
    ])

The tree is deep because expression precedence is encoded by rule nesting:
``expr → term → power → unary → primary``. This is how we represent that
``*`` binds tighter than ``+`` without any special precedence annotations.

Locating the Grammar File
--------------------------

The ``dartmouth_basic.grammar`` file lives in ``code/grammars/`` at the
repository root. We locate it relative to this module's file path::

    parser.py
    └── dartmouth_basic_parser/   (parent)
        └── src/                  (parent)
            └── dartmouth-basic-parser/ (parent)
                └── python/       (parent)
                    └── packages/ (parent)
                        └── code/ (parent)
                            └── grammars/
                                └── dartmouth_basic.grammar
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_parser_grammar
from dartmouth_basic_lexer import tokenize_dartmouth_basic
from lang_parser import ASTNode, GrammarParser

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------
#
# Navigate six levels up from this source file to reach the repository root,
# then down into code/grammars/. This path works whether the package is
# installed in development mode (-e) or used in-place.

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
DARTMOUTH_BASIC_GRAMMAR_PATH = GRAMMAR_DIR / "dartmouth_basic.grammar"


def create_dartmouth_basic_parser(source: str) -> GrammarParser:
    """Create a ``GrammarParser`` configured for Dartmouth BASIC source text.

    This function:

    1. Tokenizes the source text using the Dartmouth BASIC lexer.
       The lexer applies two post-tokenize hooks:
       - LINE_NUM relabeling: the first integer on each source line is
         relabeled from NUMBER to LINE_NUM.
       - REM suppression: everything after a REM keyword until end-of-line
         is stripped (comment removal).
    2. Reads and parses the ``dartmouth_basic.grammar`` file.
    3. Creates a ``GrammarParser`` with those tokens and grammar.

    Args:
        source: The Dartmouth BASIC source text to parse. Lines should be
            newline-terminated (``"10 LET X = 5\\n"``). The language is
            case-insensitive: ``let``, ``LET``, and ``Let`` all work.

    Returns:
        A ``GrammarParser`` instance ready to produce an AST.
        Call ``.parse()`` on it to get the AST.

    Raises:
        FileNotFoundError: If the grammar file cannot be found.
        LexerError: If the source contains invalid characters.

    Example::

        parser = create_dartmouth_basic_parser("10 LET X = 42\\n20 END\\n")
        ast = parser.parse()
        print(ast.rule_name)  # "program"
    """
    tokens = tokenize_dartmouth_basic(source)
    grammar = parse_parser_grammar(DARTMOUTH_BASIC_GRAMMAR_PATH.read_text())
    return GrammarParser(tokens, grammar)


def parse_dartmouth_basic(source: str) -> ASTNode:
    """Parse Dartmouth BASIC 1964 source text and return an AST.

    This is the main entry point for the Dartmouth BASIC parser. Pass in a
    BASIC program as a string, and get back an ``ASTNode`` representing the
    complete parse tree.

    The returned AST always has ``rule_name="program"`` at the root. Each
    child of the root is a ``line`` node, and each line contains an optional
    ``statement`` node surrounded by its ``LINE_NUM`` token and ``NEWLINE``
    token.

    The 17 statement types in 1964 Dartmouth BASIC:

    - ``let_stmt``     — ``LET variable = expr``
    - ``print_stmt``   — ``PRINT [ print_list ]``
    - ``input_stmt``   — ``INPUT variable { , variable }``
    - ``if_stmt``      — ``IF expr relop expr THEN line_num``
    - ``goto_stmt``    — ``GOTO line_num``
    - ``gosub_stmt``   — ``GOSUB line_num``
    - ``return_stmt``  — ``RETURN``
    - ``for_stmt``     — ``FOR name = expr TO expr [ STEP expr ]``
    - ``next_stmt``    — ``NEXT name``
    - ``end_stmt``     — ``END``
    - ``stop_stmt``    — ``STOP``
    - ``rem_stmt``     — ``REM`` (comment, rest of line stripped by lexer)
    - ``read_stmt``    — ``READ variable { , variable }``
    - ``data_stmt``    — ``DATA number { , number }``
    - ``restore_stmt`` — ``RESTORE``
    - ``dim_stmt``     — ``DIM dim_decl { , dim_decl }``
    - ``def_stmt``     — ``DEF user_fn ( name ) = expr``

    Args:
        source: The Dartmouth BASIC source text to parse. Each statement must
            be on its own numbered line, terminated by a newline character.
            Example: ``"10 LET X = 5\\n20 PRINT X\\n30 END\\n"``

    Returns:
        An ``ASTNode`` representing the parse tree. The root node's
        ``rule_name`` is always ``"program"``.

    Raises:
        FileNotFoundError: If the grammar file cannot be found.
        LexerError: If the source contains characters invalid in BASIC.
        GrammarParseError: If the source has syntax errors.

    Example::

        ast = parse_dartmouth_basic("10 PRINT \\"HELLO WORLD\\"\\n20 END\\n")
        # ASTNode(rule_name="program", children=[
        #     ASTNode(rule_name="line", children=[
        #         Token(LINE_NUM, '10'),
        #         ASTNode(rule_name="statement", children=[
        #             ASTNode(rule_name="print_stmt", children=[...])
        #         ]),
        #         Token(NEWLINE, '\\n')
        #     ]),
        #     ASTNode(rule_name="line", children=[...])  # END line
        # ])
    """
    parser = create_dartmouth_basic_parser(source)
    return parser.parse()
