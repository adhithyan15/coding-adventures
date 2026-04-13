"""Nib Parser — parses Nib source text into ASTs using the grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarParser``. It loads
the ``nib.grammar`` file from the ``code/grammars/`` directory, tokenizes
the input using the Nib lexer, and produces a generic ``ASTNode`` tree.

What Is Nib?
------------

Nib is a safe, statically-typed toy language that compiles to Intel 4004 machine
code. The name comes from "nibble" (4 bits), the native word size of the Intel
4004 microprocessor — the world's first commercial microprocessor, released in
1971.

The Intel 4004 is extraordinarily constrained by modern standards:

- **4-bit words**: The accumulator holds one nibble (0–15). Registers R0–R15
  each hold a single nibble. Register pairs (P0–P7) hold one byte (0–255).
- **160 bytes of usable RAM**: Not kilobytes, not megabytes — 160 bytes. Every
  static variable in a Nib program must fit within this budget.
- **4 KB of ROM**: The compiled program (instructions + constants) must fit in
  4,096 bytes of read-only memory.
- **3-level hardware call stack**: The CPU has exactly three program counter
  registers (PC0, PC1, PC2). One is always in use for the current function.
  This means a call chain of depth greater than 2 is physically impossible.
- **No multiply, no divide, no floating point**: The 4004's instruction set
  contains ADD, SUB, AND, OR, XOR, and shifts — but no MUL or DIV. Floating
  point does not exist.

Writing 4004 assembly by hand is tedious and error-prone. Nib gives us a
higher-level notation with static safety guarantees, while remaining close
enough to the hardware that the compiler can produce efficient 4004 code.

Nib's Safety Model
-------------------

Nib enforces a strict subset of operations suited to the 4004 constraints:

1. **Typed nibbles**: All types fit in 4-bit (``u4``), 8-bit (``u8``),
   BCD (``bcd``), or boolean (``bool``) — no integer overflow surprises,
   because the types match the hardware words.

2. **Explicit overflow**: The ``+%`` (wrapping add) and ``+?`` (saturating add)
   operators force the programmer to choose what happens at overflow. There is
   no silent overflow.

3. **Static call depth**: Nib checks at compile time that no call chain exceeds
   depth 2. This matches the 4004's 3-level stack minus the frame already
   occupied by the current function.

4. **No recursion**: The static call graph must be acyclic. The compiler rejects
   recursive programs.

5. **Const loop bounds**: The bounds of a ``for`` loop must be compile-time
   constants. This ensures the compiler can generate DJNZ (decrement-and-jump)
   patterns with a known trip count.

6. **No heap**: The 4004 has no heap. All data is either static (RAM-mapped) or
   on the (tiny) call stack. Nib enforces this through its type system.

Grammar Structure Overview
---------------------------

The grammar in ``nib.grammar`` defines 8 precedence levels (lowest → highest)::

    Level 1 — or_expr     : logical OR  (||)
    Level 2 — and_expr    : logical AND (&&)
    Level 3 — eq_expr     : equality    (==, !=)
    Level 4 — cmp_expr    : relational  (<, >, <=, >=)
    Level 5 — add_expr    : additive    (+, -, +%, +?)
    Level 6 — bitwise_expr: bitwise     (&, |, ^)
    Level 7 — unary_expr  : unary       (!, ~)
    Level 8 — primary     : leaves      (literals, names, calls, parens)

Top-level structure::

    program   = { top_decl }
    top_decl  = const_decl | static_decl | fn_decl
    const_decl = "const" NAME COLON type EQ expr SEMICOLON
    static_decl = "static" NAME COLON type EQ expr SEMICOLON
    fn_decl   = "fn" NAME LPAREN [ param_list ] RPAREN [ ARROW type ] block

Statement structure::

    stmt      = let_stmt | assign_stmt | return_stmt | for_stmt | if_stmt | expr_stmt
    let_stmt  = "let" NAME COLON type EQ expr SEMICOLON
    assign_stmt = NAME EQ expr SEMICOLON
    return_stmt = "return" expr SEMICOLON
    for_stmt  = "for" NAME COLON type "in" expr RANGE expr block
    if_stmt   = "if" expr block [ "else" block ]
    expr_stmt = expr SEMICOLON

The RANGE token is ``..`` produced atomically by the lexer — this is not
two dot operators, but a single token. This works because Nib has no
floating-point and no struct field access, so ``.`` can only appear as
part of a ``..`` range separator.

AST Node Shapes
----------------

Callers will encounter the following node types (``rule_name`` values):

**Top-level**:

- ``program``     — root; children are zero or more ``top_decl`` nodes
- ``top_decl``    — wrapper; its single child is ``const_decl``,
  ``static_decl``, or ``fn_decl``
- ``const_decl``  — ``"const" NAME COLON type EQ expr SEMICOLON``
- ``static_decl`` — ``"static" NAME COLON type EQ expr SEMICOLON``
- ``fn_decl``     — ``"fn" NAME LPAREN [...] RPAREN [ARROW type] block``

**Parameters**:

- ``param_list``  — comma-separated list of ``param`` nodes
- ``param``       — ``NAME COLON type``

**Statements**:

- ``block``       — ``LBRACE { stmt } RBRACE``
- ``stmt``        — wrapper; single child is one of the statement types below
- ``let_stmt``    — ``"let" NAME COLON type EQ expr SEMICOLON``
- ``assign_stmt`` — ``NAME EQ expr SEMICOLON``
- ``return_stmt`` — ``"return" expr SEMICOLON``
- ``for_stmt``    — ``"for" NAME COLON type "in" expr RANGE expr block``
- ``if_stmt``     — ``"if" expr block [ "else" block ]``
- ``expr_stmt``   — ``expr SEMICOLON``

**Types**:

- ``type``        — leaf node; token value is ``"u4"``, ``"u8"``,
  ``"bcd"``, or ``"bool"``

**Expressions** (lowest to highest precedence):

- ``expr``         — top-level; wraps ``or_expr``
- ``or_expr``      — ``and_expr { LOR and_expr }``
- ``and_expr``     — ``eq_expr { LAND eq_expr }``
- ``eq_expr``      — ``cmp_expr { (EQ_EQ | NEQ) cmp_expr }``
- ``cmp_expr``     — ``add_expr { (LT | GT | LEQ | GEQ) add_expr }``
- ``add_expr``     — ``bitwise_expr { (PLUS | MINUS | WRAP_ADD | SAT_ADD) bitwise_expr }``
- ``bitwise_expr`` — ``unary_expr { (AMP | PIPE | CARET) unary_expr }``
- ``unary_expr``   — ``(BANG | TILDE) unary_expr | primary``
- ``primary``      — integer/hex literal, ``true``, ``false``, ``call_expr``,
  NAME, or ``(expr)``

**Calls**:

- ``call_expr`` — ``NAME LPAREN [ arg_list ] RPAREN``
- ``arg_list``  — ``expr { COMMA expr }``

Grammar-Driven Approach
------------------------

Rather than writing a hand-coded recursive-descent parser, this module loads
``nib.grammar`` at runtime and feeds it to the generic ``GrammarParser`` engine.
The approach has two key advantages:

1. **Single source of truth**: The grammar file defines both what parses and
   serves as documentation. Changing the grammar immediately changes what the
   parser accepts, with no code changes required.

2. **Language-agnostic engine**: The same ``GrammarParser`` that handles Nib
   also handles ALGOL 60, Python, JSON, and every other language in this repo.
   The engine is general; only the grammar file changes per language.

Locating the Grammar File
--------------------------

The ``nib.grammar`` file lives in ``code/grammars/`` at the repository root.
We locate it relative to this module's file path::

    parser.py
    └── nib_parser/      (parent)
        └── src/         (parent)
            └── nib-parser/ (parent)
                └── python/    (parent)
                    └── packages/ (parent)
                        └── code/     (parent)
                            └── grammars/
                                └── nib.grammar
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_parser_grammar
from lang_parser import ASTNode, GrammarParser
from nib_lexer import tokenize_nib

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
NIB_GRAMMAR_PATH = GRAMMAR_DIR / "nib.grammar"


def create_nib_parser(source: str) -> GrammarParser:
    """Create a ``GrammarParser`` configured for Nib text.

    This function:

    1. Tokenizes the source text using the Nib lexer.
    2. Reads and parses the ``nib.grammar`` file.
    3. Creates a ``GrammarParser`` with those tokens and grammar rules.

    The parser handles all Nib grammar features, including:

    - Top-level declarations (``const``, ``static``, ``fn``) with no
      enclosing module or main block — the entry point is a function
      named ``main`` found by convention.
    - Full expression precedence hierarchy with 8 levels, from logical
      OR down to primary expressions. The 8-level hierarchy means that
      ``1 +% 2 == 3 && true`` parses as ``((1 +% 2) == 3) && true``
      without any explicit parentheses.
    - Explicit overflow operators ``+%`` (wrapping) and ``+?``
      (saturating), matching what the Intel 4004's ADD instruction
      can physically express.
    - Range-based ``for`` loops with the RANGE token ``..`` handled
      atomically by the lexer.
    - Brace-delimited blocks for function bodies, if/else branches,
      and for-loop bodies — no dangling-else ambiguity possible.
    - Function calls before bare names in primary expressions, avoiding
      the ambiguity where a function name could be consumed as a variable
      reference before the parser sees the opening parenthesis.

    Args:
        source: The Nib source text to parse.

    Returns:
        A ``GrammarParser`` instance ready to produce an AST.
        Call ``.parse()`` on it to get the root ``ASTNode``.

    Raises:
        FileNotFoundError: If the grammar files cannot be found.
        LexerError: If the source contains characters not valid in Nib.

    Example::

        parser = create_nib_parser("fn main() { let x: u4 = 5; }")
        ast = parser.parse()
    """
    tokens = tokenize_nib(source)
    grammar = parse_parser_grammar(NIB_GRAMMAR_PATH.read_text())
    return GrammarParser(tokens, grammar)


def parse_nib(source: str) -> ASTNode:
    """Parse Nib text and return an AST.

    This is the main entry point for the Nib parser. Pass in a string of
    Nib source text, and get back an ``ASTNode`` representing the complete
    parse tree rooted at the ``program`` rule.

    The returned AST has the following high-level structure:

    - Root node: ``rule_name="program"``
    - Children of the root: zero or more ``top_decl`` nodes.
    - A ``top_decl`` node has one child: either ``const_decl``,
      ``static_decl``, or ``fn_decl``.

    A Nib program with no top-level declarations is valid (the grammar uses
    ``{ top_decl }`` meaning zero or more). An empty string is also valid
    and produces a ``program`` node with no children.

    Statement node types (``rule_name`` values you will encounter):

    - ``let_stmt``    — local variable binding: ``let x: u4 = 5;``
    - ``assign_stmt`` — variable mutation: ``x = x +% 1;``
    - ``return_stmt`` — function return: ``return result;``
    - ``for_stmt``    — range loop: ``for i: u8 in 0..10 { }``
    - ``if_stmt``     — conditional: ``if carry { } else { }``
    - ``expr_stmt``   — call-expression-as-statement: ``update(row);``

    Top-level declaration node types:

    - ``const_decl``  — compile-time constant, inlined into ROM
    - ``static_decl`` — static RAM variable, limited to 160 bytes total
    - ``fn_decl``     — function definition with optional return type

    Expression node types (lowest to highest precedence):

    - ``or_expr``, ``and_expr``, ``eq_expr``, ``cmp_expr``
    - ``add_expr``, ``bitwise_expr``, ``unary_expr``, ``primary``

    Args:
        source: The Nib source text to parse.

    Returns:
        An ``ASTNode`` representing the parse tree. The root node's
        ``rule_name`` is ``"program"``.

    Raises:
        FileNotFoundError: If the grammar files cannot be found.
        LexerError: If the source contains characters not valid in Nib.
        GrammarParseError: If the source has syntax errors according to
            the Nib grammar.

    Example::

        ast = parse_nib("fn main() { let x: u4 = 5; }")
        # ASTNode(rule_name="program", children=[
        #   ASTNode(rule_name="top_decl", children=[
        #     ASTNode(rule_name="fn_decl", children=[
        #       Token(FN, 'fn'),
        #       Token(NAME, 'main'),
        #       Token(LPAREN, '('),
        #       Token(RPAREN, ')'),
        #       ASTNode(rule_name="block", children=[
        #         Token(LBRACE, '{'),
        #         ASTNode(rule_name="stmt", children=[
        #           ASTNode(rule_name="let_stmt", children=[...])
        #         ]),
        #         Token(RBRACE, '}')
        #       ])
        #     ])
        #   ])
        # ])
    """
    parser = create_nib_parser(source)
    return parser.parse()
