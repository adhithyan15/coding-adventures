"""AST node types for the Tetrad parser (spec TET02).

Every node is a Python dataclass with ``line`` and ``column`` fields so that
later stages (type checker, compiler) can produce useful error messages that
point back to the original source text.

The AST hierarchy mirrors the grammar exactly:

    Program
        FnDecl | GlobalDecl      (top-level declarations)
            Block                (statement sequence in braces)
                Stmt             (LetStmt | AssignStmt | IfStmt | ...)
                    Expr         (BinaryExpr | UnaryExpr | NameExpr | ...)

Forward references are handled automatically by ``from __future__ import
annotations``, which makes all annotation strings lazy.  The union type
aliases at the bottom are runtime values (Python 3.10+ ``X | Y`` syntax),
used by type checkers such as mypy.
"""

from __future__ import annotations

from dataclasses import dataclass

# ---------------------------------------------------------------------------
# Top-level
# ---------------------------------------------------------------------------


@dataclass
class Program:
    """The root of every Tetrad AST.

    ``decls`` is ordered as it appears in the source.  Top-level ``let``
    declarations are ``GlobalDecl``; top-level ``fn`` declarations are
    ``FnDecl``.  The compiler processes globals before functions.
    """

    decls: list[FnDecl | GlobalDecl]
    line: int = 0
    column: int = 0


# ---------------------------------------------------------------------------
# Declarations
# ---------------------------------------------------------------------------


@dataclass
class FnDecl:
    """A function declaration: ``fn name(params...) -> ret { body }``.

    ``param_types`` is parallel to ``params``: element i is the declared type
    of params[i], or ``None`` if that parameter has no annotation.
    ``return_type`` is ``None`` if the return type is unannotated.

    Both are ``None``-heavy for untyped functions (most Tetrad v1 programs).
    The type checker reads them to classify functions into FULLY_TYPED,
    PARTIALLY_TYPED, or UNTYPED tiers.
    """

    name: str
    params: list[str]
    param_types: list[str | None]
    return_type: str | None
    body: Block
    line: int
    column: int


@dataclass
class GlobalDecl:
    """A top-level variable: ``let name = expr;``.

    ``declared_type`` is the optional ``: u8`` annotation.  The compiler
    allocates a global variable slot for it.
    """

    name: str
    declared_type: str | None
    value: Expr
    line: int
    column: int


# ---------------------------------------------------------------------------
# Statements
# ---------------------------------------------------------------------------


@dataclass
class Block:
    """A brace-delimited sequence of statements: ``{ stmt* }``.

    Blocks appear as function bodies, if-then branches, if-else branches, and
    while-loop bodies.  The parser never creates an empty Block node — even
    ``{ }`` produces ``Block(stmts=[])``.
    """

    stmts: list[Stmt]
    line: int
    column: int


@dataclass
class LetStmt:
    """A local variable introduction: ``let name: type = expr;``.

    The ``: type`` annotation is optional (``declared_type = None`` when absent).
    """

    name: str
    declared_type: str | None
    value: Expr
    line: int
    column: int


@dataclass
class AssignStmt:
    """An assignment to an already-declared variable: ``name = expr;``.

    The parser distinguishes assignment from expression statements by peeking
    two tokens ahead: IDENT followed immediately by ``=`` (not ``==``) is an
    assignment.  Everything else is an expression statement.
    """

    name: str
    value: Expr
    line: int
    column: int


@dataclass
class IfStmt:
    """Conditional branch: ``if expr block [else block]``.

    ``else_block`` is ``None`` when no ``else`` clause is written.  There is
    no ``elif``; chained conditionals nest: ``if a {} else { if b {} }``.
    """

    condition: Expr
    then_block: Block
    else_block: Block | None
    line: int
    column: int


@dataclass
class WhileStmt:
    """Loop: ``while expr block``.

    The compiler emits ``JMP_LOOP`` (distinct from plain ``JMP``) for the
    backward branch so the VM can cheaply count loop back-edges for the JIT
    hot-loop detector.
    """

    condition: Expr
    body: Block
    line: int
    column: int


@dataclass
class ReturnStmt:
    """Function return: ``return [expr] ;``.

    ``value`` is ``None`` for a bare ``return;``, which the compiler treats as
    ``return 0`` (loads zero then emits ``RET``).
    """

    value: Expr | None
    line: int
    column: int


@dataclass
class ExprStmt:
    """An expression used as a statement: ``expr ;``.

    The primary use is ``out(n);``, but any expression with side effects
    (such as a void-typed call) can appear here.
    """

    expr: Expr
    line: int
    column: int


# ---------------------------------------------------------------------------
# Expressions
# ---------------------------------------------------------------------------


@dataclass
class IntLiteral:
    """A decimal or hexadecimal integer constant (``42`` or ``0xFF``).

    The lexer already evaluates the value to a Python ``int``.  The parser
    stores it verbatim; the compiler rejects values outside 0–255.
    """

    value: int
    line: int
    column: int


@dataclass
class NameExpr:
    """A variable or parameter reference: ``x``, ``counter``, ``_tmp``.

    If the same identifier is followed by ``(`` the parser produces a
    ``CallExpr`` instead.
    """

    name: str
    line: int
    column: int


@dataclass
class BinaryExpr:
    """An infix binary operation.

    ``op`` is one of: ``+  -  *  /  %  &  |  ^  <<  >>
                       ==  !=  <  <=  >  >=  &&  ||``

    ``&&`` and ``||`` are short-circuit operators; the *compiler* emits
    conditional jumps for them — the parser just captures the AST shape.

    ``left`` and ``right`` are themselves ``Expr`` nodes, possibly nested.
    Precedence is captured by the shape of the tree, not by explicit fields.
    """

    op: str
    left: Expr
    right: Expr
    line: int
    column: int


@dataclass
class UnaryExpr:
    """A prefix unary operation.

    ``op`` is one of: ``!`` (logical not), ``~`` (bitwise not), ``-`` (negate).

    Unary operators bind tighter than all binary operators (bp=110 in the
    Pratt table), so ``-a * b`` parses as ``(-a) * b``.
    """

    op: str
    operand: Expr
    line: int
    column: int


@dataclass
class CallExpr:
    """A function call: ``name(arg, ...)``.

    Arguments are evaluated left-to-right.  The compiler places them in
    registers R0..R(argc-1) before emitting ``CALL``.
    """

    name: str
    args: list[Expr]
    line: int
    column: int


@dataclass
class InExpr:
    """I/O read: ``in()``.

    Reads one u8 value from the hardware I/O port.  In the Python VM this
    calls ``input()`` and parses the result as an integer.  In the 4004
    interpreter it maps to the WRM/RDM instruction sequence.

    ``in()`` always has parentheses — ``in`` alone is a ParseError.
    """

    line: int
    column: int


@dataclass
class OutExpr:
    """I/O write: ``out(expr)``.

    Sends one u8 value to the hardware output port.  In the Python VM this
    prints to stdout.  In the 4004 interpreter it maps to WMP.

    Modelled as an expression so it can appear as an ``ExprStmt``.
    Its type is ``Void`` — the result is meaningless.
    """

    value: Expr
    line: int
    column: int


@dataclass
class GroupExpr:
    """A parenthesized expression: ``(expr)``.

    Preserved as a distinct node rather than stripped, so that later stages
    can reconstruct source positions accurately.  The compiler treats it as
    fully transparent.
    """

    expr: Expr
    line: int
    column: int


# ---------------------------------------------------------------------------
# Union type aliases
# ---------------------------------------------------------------------------

Expr = (
    BinaryExpr
    | UnaryExpr
    | CallExpr
    | InExpr
    | OutExpr
    | NameExpr
    | IntLiteral
    | GroupExpr
)

Stmt = Block | LetStmt | AssignStmt | IfStmt | WhileStmt | ReturnStmt | ExprStmt
