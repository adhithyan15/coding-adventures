"""ALGOL 60 Parser — parses ALGOL 60 source text into ASTs.

This module is a thin wrapper around the generic ``GrammarParser``. It imports
the compiled ``algol/algol60.grammar`` grammar as native Python data, tokenizes
the input using the ALGOL 60 lexer, and produces a generic ``ASTNode`` tree
without runtime grammar-file I/O.

A Short History of ALGOL 60 Grammar
-------------------------------------

ALGOL 60 was the first programming language whose grammar was formally
specified in BNF (Backus-Naur Form). Peter Naur edited the *Revised Report on
the Algorithmic Language ALGOL 60* (1963), and John Backus invented the
notation. Every compiler theory textbook — from Aho/Ullman (Dragon Book) to
Wirth (Compiler Construction) — uses BNF because ALGOL introduced it.

The ALGOL 60 grammar is historically significant for several design decisions
that influenced all subsequent languages:

1. **Block structure** (``begin``...``end``): A block opens a new lexical
   scope. Declarations precede statements. Nested blocks shadow outer names.
   This is the origin of Python's ``def``, JavaScript's ``{}``, and C's
   compound statements.

2. **Dangling else resolution**: ``if b then S`` — what does ``else`` bind to
   when S is itself a conditional? ALGOL resolves this *at the grammar level*
   by requiring the then-branch to be an ``unlabeled_stmt`` (which excludes
   conditionals). The else-branch is a full ``statement`` (which includes
   conditionals). If you want a nested if as the then-branch, you must write
   ``if b then begin if c then ... end``. This forces explicit intent.

   C, Java, and most C-syntax languages resolve the dangling else by
   convention (else binds to nearest if), not grammar. This is a weaker
   solution — the grammar itself is ambiguous, and the convention must be
   memorized separately.

3. **Conditional expressions**: ``if b then x else y`` can appear inside
   arithmetic and boolean expressions, not just as statements. This is
   cleaner than C's ternary ``b ? x : y`` operator (which requires
   different syntax for the same concept in expression vs. statement form).

4. **Call-by-name semantics**: Procedure parameters are passed by name by
   default — the argument expression is re-evaluated at every use inside
   the body. This makes Algol naturally lazy in function arguments. The
   ``value`` keyword opts into call-by-value. Jensen's device (Knuth
   Vol. 1, p. 19) demonstrates the power of call-by-name: passing ``A[i]``
   and ``i`` to a summation procedure computes the sum of a vector without
   any special array-sum function.

5. **Dynamic arrays**: Array bounds are arithmetic expressions evaluated
   at block entry. The stack grows or shrinks at runtime to hold the array.
   This is more flexible than C's fixed-size stack arrays (added in C99 as
   VLAs, but optional and discouraged).

6. **Left-associative exponentiation**: ``2^3^4 = (2^3)^4 = 4096``. Most
   modern languages and mathematics use right-associativity for ``^``
   (``2^3^4 = 2^(3^4) = 2^81``). The ALGOL 60 report explicitly chose
   left-associativity, which surprises most programmers. Our grammar
   implements this faithfully.

Grammar Structure
------------------

The grammar in ``algol/algol60.grammar`` uses EBNF extensions::

    { x }    zero or more repetitions
    [ x ]    optional (zero or one)
    |        alternation

Entry point is ``program``, which expands to a single ``block``::

    program = block ;
    block   = BEGIN { declaration SEMICOLON }
              statement { SEMICOLON statement } END ;

The grammar is split into five sections:

1. **Top level**: ``program``, ``block``
2. **Declarations**: type, array, switch, procedure declarations
3. **Statements**: assignment, goto, procedure call, compound, for, if, empty
4. **Expressions**: arithmetic, boolean, designational, with full precedence
5. **Variables and calls**: subscripted variables, procedure calls

What This Module Provides
--------------------------

Two convenience functions:

- ``create_algol_parser(source)`` — tokenizes the source with ``algol_lexer``
  and creates a ``GrammarParser`` configured with the ALGOL 60 grammar.
- ``parse_algol(source)`` — the all-in-one function. Pass in ALGOL 60 text,
  get back an AST.

Compiled Grammar
----------------

The source grammar lives at ``code/grammars/algol/algol60.grammar`` for
authoring and regeneration. Runtime code imports ``PARSER_GRAMMAR`` from
``algol_parser._grammar`` so installed packages do not need repository-relative
grammar files.
"""

from __future__ import annotations

from algol_lexer import (
    DEFAULT_VERSION,
    SUPPORTED_VERSIONS,
    resolve_version,
    tokenize_algol,
)
from lang_parser import ASTNode, GrammarParser

from algol_parser._grammar import PARSER_GRAMMAR

_PARSER_GRAMMARS = {version: PARSER_GRAMMAR for version in SUPPORTED_VERSIONS}


def create_algol_parser(
    source: str,
    version: str | None = DEFAULT_VERSION,
) -> GrammarParser:
    """Create a ``GrammarParser`` configured for ALGOL 60 text.

    This function:

    1. Tokenizes the source text using the ALGOL 60 lexer.
    2. Selects the compiled ``algol/algol60.grammar`` grammar.
    3. Creates a ``GrammarParser`` with those tokens and grammar rules.

    The parser handles all ALGOL 60 grammar features, including:

    - Block structure (``begin``...``end``) with declaration-before-statement
      ordering enforced at the grammar level.
    - The dangling else resolution: the then-branch of a conditional cannot
      itself be a conditional without being wrapped in ``begin``...``end``.
    - Full expression precedence for arithmetic (7 levels) and boolean (5 levels).
    - For loop with step/until, while, and simple element forms.
    - Procedure declarations with ``value`` and ``spec`` parts.
    - Array declarations with dynamic bounds.
    - Switch declarations and designational expressions for computed gotos.

    Args:
        source: The ALGOL 60 text to parse.

    Returns:
        A ``GrammarParser`` instance ready to produce an AST.
        Call ``.parse()`` on it to get the AST.

    Raises:
        LexerError: If the source contains characters invalid in ALGOL 60.

    Example::

        parser = create_algol_parser('begin integer x; x := 42 end')
        ast = parser.parse()
    """
    resolved = resolve_version(version)
    tokens = tokenize_algol(source, version=resolved)
    grammar = _PARSER_GRAMMARS[resolved]
    return GrammarParser(tokens, grammar)


def parse_algol(source: str, version: str | None = DEFAULT_VERSION) -> ASTNode:
    """Parse ALGOL 60 text and return an AST.

    This is the main entry point for the ALGOL 60 parser. Pass in a string of
    ALGOL 60 source text, and get back an ``ASTNode`` representing the complete
    parse tree rooted at the ``program`` rule.

    The returned AST has the following high-level structure:

    - Root node: ``rule_name="program"``
    - The root's single child is a ``block`` node.
    - A block contains:
      - ``BEGIN`` token
      - Zero or more ``declaration`` nodes (each followed by SEMICOLON)
      - Zero or more ``statement`` nodes (separated by SEMICOLONs)
      - ``END`` token

    Statement node types (``rule_name`` values you will encounter):

    - ``assign_stmt`` — assignment: left-hand sides + expression
    - ``cond_stmt``   — if/then/else conditional
    - ``for_stmt``    — for loop (step/until, while, or simple forms)
    - ``compound_stmt`` — begin/end with only statements (no declarations)
    - ``goto_stmt``   — goto with a designational expression
    - ``proc_stmt``   — procedure call as a statement
    - ``dummy_stmt``  — zero-width ALGOL dummy statement
    - ``block``       — nested block (introduces a new scope)

    Declaration node types:

    - ``type_decl``       — ``integer x, y, z``
    - ``own_decl``        — ``own integer counter``
    - ``own_array_decl``  — ``own integer array A[1:10]``
    - ``array_decl``      — ``array A[1:10]``
    - ``switch_decl``     — ``switch s := label1, label2``
    - ``procedure_decl``  — ``procedure p(x); ...``

    Expression node types:

    - ``arith_expr``, ``simple_arith``, ``term``, ``factor``, ``primary``
    - ``bool_expr``, ``simple_bool``, ``implication``, ``bool_term``
    - ``bool_factor``, ``bool_secondary``, ``bool_primary``, ``relation``
    - ``expression`` — top-level (arithmetic or boolean)
    - ``desig_expr``  — designational (for goto targets)

    Args:
        source: The ALGOL 60 text to parse.

    Returns:
        An ``ASTNode`` representing the parse tree. The root node's
        ``rule_name`` is ``"program"``.

    Raises:
        LexerError: If the source contains characters invalid in ALGOL 60.
        GrammarParseError: If the source has syntax errors according to
            the ALGOL 60 grammar.

    Example::

        ast = parse_algol('begin integer x; x := 42 end')
        # ASTNode(rule_name="program", children=[
        #   ASTNode(rule_name="block", children=[
        #     Token(BEGIN, 'begin'),
        #     ASTNode(rule_name="declaration", children=[
        #       ASTNode(rule_name="type_decl", children=[
        #         Token(INTEGER, 'integer'), Token(IDENT, 'x')
        #       ])
        #     ]),
        #     Token(SEMICOLON, ';'),
        #     ASTNode(rule_name="statement", children=[
        #       ASTNode(rule_name="assign_stmt", ...)
        #     ]),
        #     Token(END, 'end')
        #   ])
        # ])
    """
    parser = create_algol_parser(source, version=version)
    return parser.parse()
