"""Oct Parser — parses Oct source text into ASTs using the grammar-driven approach.

This module is a thin wrapper around the generic ``GrammarParser``. It loads
``oct.grammar`` from the ``code/grammars/`` directory, tokenizes input using
the Oct lexer, and produces a generic ``ASTNode`` tree.

What Is Oct?
------------

Oct is a safe, statically-typed toy language that compiles to Intel 8008 machine
code. The name comes from "octet" (8 bits), the native word size of the Intel
8008 microprocessor — a direct ancestor of the x86 family, released in 1972.

The Intel 8008 is highly constrained by modern standards:

- **8-bit words**: The accumulator (A) and GP registers (B, C, D, E) each hold
  one byte (0–255). All arithmetic wraps modulo 256 unless carry is checked.
- **4 registers for locals**: Only B, C, D, E are available for function locals;
  A is the accumulator (scratch); H:L is the memory pointer pair. At most 4
  local variables (including parameters) per function.
- **16 KB address space**: 14-bit address bus (0x0000–0x3FFF). ROM occupies
  0x0000–0x1FFF; the data segment (static variables) is at 0x2000–0x3FFF.
- **8-level push-down call stack**: The hardware maintains an internal stack of
  8 program counter registers. One is always in use, leaving 7 call levels.
- **Separate I/O space**: 8 input ports (INP p, port 0–7) and 24 output ports
  (OUT p, port 0–23), with port numbers encoded directly in the instruction.
- **4 hardware flags**: CY (carry), Z (zero), S (sign), P (parity). Oct exposes
  CY via carry() and adc()/sbb(), and P via parity().

Writing 8008 assembly by hand is tedious and error-prone. Oct gives us a
higher-level notation with static safety guarantees while remaining faithful
to the hardware.

Grammar Structure Overview
---------------------------

The grammar in ``oct.grammar`` defines 8 expression precedence levels
(lowest → highest)::

    Level 1 — or_expr      : logical OR   (||)
    Level 2 — and_expr     : logical AND  (&&)
    Level 3 — eq_expr      : equality     (==, !=)
    Level 4 — cmp_expr     : relational   (<, >, <=, >=)
    Level 5 — add_expr     : additive     (+, -)
    Level 6 — bitwise_expr : bitwise      (&, |, ^)
    Level 7 — unary_expr   : unary        (!, ~)
    Level 8 — primary      : leaves       (literals, names, calls, parens)

Top-level structure::

    program     = { top_decl }
    top_decl    = static_decl | fn_decl
    static_decl = "static" NAME COLON type EQ expr SEMICOLON
    fn_decl     = "fn" NAME LPAREN [ param_list ] RPAREN [ ARROW type ] block

Statement kinds::

    let_stmt    = "let" NAME COLON type EQ expr SEMICOLON
    assign_stmt = NAME EQ expr SEMICOLON
    return_stmt = "return" [ expr ] SEMICOLON
    if_stmt     = "if" expr block [ "else" block ]
    while_stmt  = "while" expr block
    loop_stmt   = "loop" block
    break_stmt  = "break" SEMICOLON
    expr_stmt   = expr SEMICOLON

Intrinsic calls (all begin with keyword tokens, not NAME)::

    in(PORT)          → reads from 8008 input port 0–7
    out(PORT, val)    → writes to 8008 output port 0–23
    adc(a, b)         → add with carry (a + b + CY)
    sbb(a, b)         → subtract with borrow (a - b - CY)
    rlc(a)            → rotate left circular (CY ← bit7; A ← (A<<1)|bit7)
    rrc(a)            → rotate right circular
    ral(a)            → rotate left through carry (9-bit)
    rar(a)            → rotate right through carry (9-bit)
    carry()           → read carry flag → bool
    parity(a)         → read parity flag of a → bool

Design Decisions
-----------------

**Intrinsics before call_expr in primary**: Intrinsic calls start with keyword
tokens (``"in"``, ``"carry"``, etc.), not NAME tokens. If ``call_expr`` (which
requires a NAME) were tried first, the parser would fail to match. Listing
``intrinsic_call`` first ensures keyword-started calls are routed correctly.

**call_expr before NAME in primary**: A user-defined call starts with NAME + ``(``.
If NAME were tried first, the parser would consume the function name as a
variable reference and never see the ``(``. Listing ``call_expr`` before ``NAME``
prevents this ambiguity.

**type = NAME**: Type annotations (``u8``, ``bool``) are NAME tokens with
specific values, not keywords. This keeps the keyword set minimal and moves
type validation to the type-checker, where it belongs.

**return with optional expr**: ``return;`` (void) and ``return expr;`` (non-void)
share one grammar rule with an optional expression. The type-checker validates
that a value is returned when the function declares a return type.

**Both static and let in stmt**: The grammar allows ``static_decl`` inside
function bodies. The type-checker rejects this — static declarations are
only valid at file scope. The parser is deliberately permissive; the type-
checker is where scope rules belong.
"""

from __future__ import annotations

from pathlib import Path

from grammar_tools import parse_parser_grammar
from lang_parser import ASTNode, GrammarParser
from oct_lexer import tokenize_oct

# ---------------------------------------------------------------------------
# Grammar File Location
# ---------------------------------------------------------------------------

GRAMMAR_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "grammars"
OCT_GRAMMAR_PATH = GRAMMAR_DIR / "oct.grammar"


def create_oct_parser(source: str) -> GrammarParser:
    """Create a ``GrammarParser`` configured for Oct source text.

    This function:

    1. Tokenizes the source text using the Oct lexer.
    2. Reads and parses the ``oct.grammar`` file.
    3. Creates a ``GrammarParser`` with those tokens and grammar rules.

    The parser handles all Oct grammar features, including:

    - Top-level declarations (``static``, ``fn``) with no enclosing module or
      main block — the entry point is ``fn main()`` found by convention.
    - Full expression precedence hierarchy with 8 levels (logical OR down to
      primary). For example, ``a + b & c == d && e`` parses as
      ``((a + (b & c)) == d) && e`` without explicit parentheses.
    - Intrinsic calls (``in``, ``out``, ``adc``, ``sbb``, ``rlc``, ``rrc``,
      ``ral``, ``rar``, ``carry``, ``parity``) matched before user-defined
      calls, because they begin with keyword tokens not NAME tokens.
    - User-defined function calls matched before bare NAME references, to avoid
      consuming the function name as a variable before seeing the ``(``.
    - Brace-delimited blocks on all if/else/while/loop/fn bodies — no dangling-
      else ambiguity.
    - Optional else branch on ``if`` statements.
    - Optional expression on ``return`` (for void functions: ``return;``).
    - Both ``while`` and ``loop`` loops; ``break`` exits the innermost one.

    Args:
        source: The Oct source text to parse.

    Returns:
        A ``GrammarParser`` instance ready to produce an AST.
        Call ``.parse()`` on it to get the root ``ASTNode``.

    Raises:
        FileNotFoundError: If the grammar files cannot be found.
        LexerError: If the source contains characters not valid in Oct.
        GrammarParseError: If the source has syntax errors per the Oct grammar.

    Example::

        parser = create_oct_parser("fn main() { let x: u8 = 0xFF; }")
        ast = parser.parse()
    """
    tokens = tokenize_oct(source)
    grammar = parse_parser_grammar(OCT_GRAMMAR_PATH.read_text())
    return GrammarParser(tokens, grammar)


def parse_oct(source: str) -> ASTNode:
    """Parse Oct source text and return the root AST node.

    This is the main entry point for the Oct parser. It tokenizes the source
    with the Oct lexer, parses with the Oct grammar, and returns an ``ASTNode``
    tree rooted at a ``"program"`` node.

    The AST is a generic tree of ``ASTNode`` and ``Token`` objects — the same
    structure used by every grammar-driven parser in this repository (Nib,
    ALGOL 60, Python, JSON, etc.). The ``rule_name`` attribute identifies each
    interior node; leaf nodes are ``Token`` objects.

    Top-level node types (``rule_name`` values at depth 2):

    - ``static_decl`` — static variable: ``static counter: u8 = 0;``
    - ``fn_decl`` — function definition with optional return type

    Statement node types (``rule_name`` values you will encounter):

    - ``let_stmt``    — local variable binding: ``let x: u8 = 42;``
    - ``assign_stmt`` — variable mutation: ``x = x + 1;``
    - ``return_stmt`` — function return: ``return result;`` or ``return;``
    - ``if_stmt``     — conditional: ``if carry() { … } else { … }``
    - ``while_stmt``  — while loop: ``while n != 0 { … }``
    - ``loop_stmt``   — infinite loop: ``loop { … }``
    - ``break_stmt``  — loop exit: ``break;``
    - ``expr_stmt``   — expression as statement: ``out(1, x);``

    Expression node types (lowest to highest precedence):

    - ``or_expr``, ``and_expr``, ``eq_expr``, ``cmp_expr``
    - ``add_expr``, ``bitwise_expr``, ``unary_expr``, ``primary``
    - ``intrinsic_call`` — e.g. ``carry()``, ``in(0)``, ``adc(a, b)``
    - ``call_expr`` — user-defined function call

    A program with no top-level declarations is valid (the grammar uses
    ``{ top_decl }`` meaning zero or more). An empty string produces a
    ``program`` node with no children.

    Args:
        source: The Oct source text to parse.

    Returns:
        An ``ASTNode`` representing the parse tree. The root node's
        ``rule_name`` is ``"program"``.

    Raises:
        FileNotFoundError: If the grammar files cannot be found.
        LexerError: If the source contains characters not valid in Oct.
        GrammarParseError: If the source has syntax errors per the Oct grammar.

    Example — simple function::

        ast = parse_oct("fn main() { let x: u8 = 5; }")
        # ASTNode(rule_name="program", children=[
        #   ASTNode(rule_name="top_decl", children=[
        #     ASTNode(rule_name="fn_decl", children=[
        #       Token(fn, 'fn'),
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

    Example — intrinsic call::

        ast = parse_oct("fn main() { let b: u8 = in(0); out(8, b); }")
        # The 'in(0)' appears as an intrinsic_call node under primary.
    """
    parser = create_oct_parser(source)
    return parser.parse()
