"""AST → IR compilation.

Design notes
------------

The grammar-driven parser produces a "concrete" AST — one node per
grammar rule — which mirrors the precedence cascade literally. Most of
those nodes carry no information beyond "we descended through this
level of precedence on the way to the actual operator", so the first
thing we do is *unwrap* single-child nodes until we reach a node that
actually carries content.

Once unwrapped, the meaningful shapes we care about are:

- ``atom`` with a single token child → IR literal or symbol
- ``postfix`` with N children → function call chain
- ``power`` with 3 children → ``Pow(base, exponent)``
- ``unary`` with 2 children → ``Neg(expr)`` (unary plus collapses)
- ``multiplicative``/``additive``/``logical_*`` with 2k+1 children →
  left-associative chain of binary ops
- ``comparison`` with 3 children → ``Equal``, ``Less``, etc.
- ``assign`` with 3 children → ``Assign`` or ``Define``
- ``group`` → just its inner expression
- ``list`` → ``List(elem1, elem2, ...)``

This file is one ``Compiler`` class plus a couple of module-level
helpers. The class is stateful only in the sense that it carries a
token→value cache for performance; it has no mutation semantics.
"""

from __future__ import annotations

from lang_parser import ASTNode
from lexer import Token
from symbolic_ir import (
    ACOS,
    ACOSH,
    ADD,
    ASIN,
    ASINH,
    ASSIGN,
    ATAN,
    ATANH,
    COS,
    COSH,
    DEFINE,
    DIV,
    EQUAL,
    EXP,
    GREATER,
    GREATER_EQUAL,
    INTEGRATE,
    LESS,
    LESS_EQUAL,
    LIST,
    LOG,
    MUL,
    NEG,
    NOT,
    NOT_EQUAL,
    POW,
    SIN,
    SINH,
    SQRT,
    SUB,
    TAN,
    TANH,
    D,
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRString,
    IRSymbol,
)
from symbolic_ir.nodes import AND, OR

# Statement-terminator wrappers. The compiler emits one of these around
# every top-level statement so the REPL can distinguish ``;`` (display
# the result) from ``$`` (suppress). The macsyma-runtime backend
# installs identity handlers for both, so non-REPL consumers (e.g.
# tests that drive the VM directly) don't have to think about them.
DISPLAY = IRSymbol("Display")
SUPPRESS = IRSymbol("Suppress")

# ---------------------------------------------------------------------------
# Errors
# ---------------------------------------------------------------------------


class CompileError(Exception):
    """Raised when the compiler encounters an AST shape it cannot handle.

    This should only happen if the grammar and the compiler fall out of
    sync — the shapes the compiler expects are documented at the top of
    this module.
    """


# ---------------------------------------------------------------------------
# Maps: token type → IR head symbol, function name → canonical head
# ---------------------------------------------------------------------------
#
# Storing these as module-level dicts makes the compiler's dispatch
# table explicit and easy to extend. Adding support for a new binary
# operator is a one-line change here.

_BINARY_OP_HEADS: dict[str, IRSymbol] = {
    "PLUS": ADD,
    "MINUS": SUB,
    "STAR": MUL,
    "SLASH": DIV,
}

_COMPARISON_HEADS: dict[str, IRSymbol] = {
    "EQ": EQUAL,
    "HASH": NOT_EQUAL,
    "LT": LESS,
    "GT": GREATER,
    "LEQ": LESS_EQUAL,
    "GEQ": GREATER_EQUAL,
}

# MACSYMA function names that map to a different canonical IR head.
# Names NOT in this table pass through unchanged — `f(x)` with a
# user-defined `f` becomes `Apply(Symbol('f'), (Symbol('x'),))`.
_STANDARD_FUNCTIONS: dict[str, IRSymbol] = {
    "diff": D,
    "integrate": INTEGRATE,
    "sin": SIN,
    "cos": COS,
    "tan": TAN,
    "asin": ASIN,
    "acos": ACOS,
    "atan": ATAN,
    "sinh": SINH,
    "cosh": COSH,
    "tanh": TANH,
    "asinh": ASINH,
    "acosh": ACOSH,
    "atanh": ATANH,
    "log": LOG,
    "exp": EXP,
    "sqrt": SQRT,
}


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _token_type_name(token: Token) -> str:
    """Return the token's type as a string regardless of enum/str form."""
    return token.type if isinstance(token.type, str) else token.type.name


def _is_token(child: object) -> bool:
    return isinstance(child, Token)


def _unwrap(node: ASTNode) -> ASTNode | Token:
    """Descend through single-child ``ASTNode`` wrappers until we find
    a node (or token) with actual structure.

    The precedence cascade produces a lot of chain nodes like
    ``logical_or(children=[logical_and(children=[logical_not(...)])])``
    when the expression is a plain atom. This collapses all of that
    into the first interesting node.
    """
    while isinstance(node, ASTNode) and len(node.children) == 1:
        child = node.children[0]
        if isinstance(child, ASTNode):
            node = child
        else:
            return child
    return node


# ---------------------------------------------------------------------------
# Compiler
# ---------------------------------------------------------------------------


class Compiler:
    """Walks a MACSYMA AST and emits symbolic IR.

    The class carries no mutable state; it is a namespace for the
    recursive compilation methods. Every ``_compile_*`` method returns
    an ``IRNode``.

    Set ``wrap_terminators=True`` (default ``False``) to have every
    top-level statement wrapped in either ``Display(expr)`` (for ``;``)
    or ``Suppress(expr)`` (for ``$``) so a REPL can later distinguish
    the two. The wrappers are off by default to preserve compatibility
    with consumers that drive the VM directly without a REPL — they
    can leave ``wrap_terminators=False`` and continue to receive raw
    expressions.
    """

    def __init__(self, *, wrap_terminators: bool = False) -> None:
        self._wrap_terminators = wrap_terminators

    def compile_program(self, root: ASTNode) -> list[IRNode]:
        """Compile a ``program`` AST into a list of IR statements.

        Args:
            root: The root ``ASTNode`` (must have ``rule_name='program'``).

        Returns:
            One ``IRNode`` per top-level statement.
        """
        if root.rule_name != "program":
            raise CompileError(
                f"expected 'program' at root, got {root.rule_name!r}"
            )
        statements: list[IRNode] = []
        for child in root.children:
            if isinstance(child, ASTNode) and child.rule_name == "statement":
                statements.append(self._compile_statement(child))
        return statements

    # ---- statement and expression ----------------------------------------

    def _compile_statement(self, node: ASTNode) -> IRNode:
        # `statement = expression ( SEMI | DOLLAR ) ;`
        #
        # The expression is always child 0; the terminator is child 1.
        # If ``wrap_terminators`` is enabled (REPL mode), wrap the
        # expression in ``Display(expr)`` or ``Suppress(expr)`` so the
        # REPL can decide whether to print the result. Otherwise,
        # return the inner expression directly — downstream consumers
        # that don't care about ``;`` vs ``$`` see the same shape they
        # always have.
        if not node.children:
            raise CompileError("statement has no children")
        expr_node = node.children[0]
        if not isinstance(expr_node, ASTNode):
            raise CompileError("statement's first child must be an AST node")
        inner = self._compile_node(expr_node)

        if not self._wrap_terminators:
            return inner

        if len(node.children) >= 2 and isinstance(node.children[1], Token):
            term_type = _token_type_name(node.children[1])
            if term_type == "DOLLAR":
                return IRApply(SUPPRESS, (inner,))
            if term_type == "SEMI":
                return IRApply(DISPLAY, (inner,))
        # Defensive fallback if the grammar ever lets a terminator slip
        # past — pick Display so the user sees the result.
        return IRApply(DISPLAY, (inner,))

    def _compile_node(self, node: ASTNode | Token) -> IRNode:
        """Dispatch to the handler for this node's rule or token type."""
        if isinstance(node, Token):
            return self._compile_token(node)

        unwrapped = _unwrap(node)
        if isinstance(unwrapped, Token):
            return self._compile_token(unwrapped)

        # unwrapped is an ASTNode with >1 children (or a leaf rule).
        rule = unwrapped.rule_name
        handler = self._handlers.get(rule)
        if handler is None:
            raise CompileError(f"no handler for rule {rule!r}")
        return handler(self, unwrapped)

    # ---- leaf tokens -----------------------------------------------------

    def _compile_token(self, tok: Token) -> IRNode:
        type_name = _token_type_name(tok)
        if type_name == "NUMBER":
            return self._parse_number(tok.value)
        if type_name == "NAME":
            return IRSymbol(tok.value)
        if type_name == "STRING":
            return IRString(tok.value)
        if type_name == "KEYWORD":
            # `true` and `false` are the two keywords that reach this
            # path as atoms. Others (and/or/not) are handled by their
            # parent expression rules.
            if tok.value == "true":
                return IRSymbol("True")
            if tok.value == "false":
                return IRSymbol("False")
        raise CompileError(f"unexpected leaf token {type_name}={tok.value!r}")

    @staticmethod
    def _parse_number(text: str) -> IRInteger | IRFloat:
        # The NUMBER regex accepts both ints and floats. If there's
        # a dot or exponent, it's a float; otherwise it's an integer.
        if "." in text or "e" in text or "E" in text:
            return IRFloat(float(text))
        return IRInteger(int(text))

    # ---- assign / define -------------------------------------------------

    def _compile_assign(self, node: ASTNode) -> IRNode:
        # `assign` with 3 children: lhs, COLON|COLONEQ, rhs.
        # (With 1 child it's already unwrapped elsewhere.)
        if len(node.children) == 1:
            return self._compile_node(node.children[0])  # type: ignore[arg-type]
        if len(node.children) != 3:
            raise CompileError("malformed assign node")
        lhs_ast, op_tok, rhs_ast = node.children
        if not isinstance(op_tok, Token):
            raise CompileError("assign's middle child must be a Token")
        op = _token_type_name(op_tok)
        rhs_ir = self._compile_node(rhs_ast)  # type: ignore[arg-type]
        lhs_ir = self._compile_node(lhs_ast)  # type: ignore[arg-type]

        if op == "COLONEQ":
            # `f(x) := body` — delayed function definition.
            # If lhs is `Apply(f, (params...))`, rewrite to
            # `Define(f, List(params...), body)`.
            if isinstance(lhs_ir, IRApply) and isinstance(lhs_ir.head, IRSymbol):
                return IRApply(
                    DEFINE,
                    (lhs_ir.head, IRApply(LIST, lhs_ir.args), rhs_ir),
                )
            # `x := body` — also a define, bind directly.
            return IRApply(DEFINE, (lhs_ir, IRApply(LIST, ()), rhs_ir))
        # op == "COLON": eager assignment.
        return IRApply(ASSIGN, (lhs_ir, rhs_ir))

    # ---- logical operators -----------------------------------------------

    def _compile_logical_chain(
        self, node: ASTNode, head: IRSymbol, op_text: str
    ) -> IRNode:
        """Left-associative chain: `a OP b OP c` → ``Apply(head, (a, b, c))``.

        MACSYMA's `and` and `or` keyword operators are flattened into a
        single variadic ``Apply`` rather than nested pairwise applies.
        This matches how Mathematica stores `And[a, b, c]` and makes
        rule matching easier.
        """
        if len(node.children) == 1:
            return self._compile_node(node.children[0])  # type: ignore[arg-type]
        operands: list[IRNode] = []
        for child in node.children:
            if isinstance(child, Token):
                continue  # skip the keyword tokens
            operands.append(self._compile_node(child))
        return IRApply(head, tuple(operands))

    def _compile_logical_or(self, node: ASTNode) -> IRNode:
        return self._compile_logical_chain(node, OR, "or")

    def _compile_logical_and(self, node: ASTNode) -> IRNode:
        return self._compile_logical_chain(node, AND, "and")

    def _compile_logical_not(self, node: ASTNode) -> IRNode:
        # 1 child → plain comparison; 2 children → NOT expr.
        if len(node.children) == 1:
            return self._compile_node(node.children[0])  # type: ignore[arg-type]
        return IRApply(NOT, (self._compile_node(node.children[1]),))  # type: ignore[arg-type]

    # ---- comparison ------------------------------------------------------

    def _compile_comparison(self, node: ASTNode) -> IRNode:
        if len(node.children) == 1:
            return self._compile_node(node.children[0])  # type: ignore[arg-type]
        if len(node.children) != 3:
            raise CompileError("malformed comparison node")
        lhs_ast, op_tok, rhs_ast = node.children
        if not isinstance(op_tok, Token):
            raise CompileError("comparison op must be a Token")
        head = _COMPARISON_HEADS.get(_token_type_name(op_tok))
        if head is None:
            raise CompileError(f"unknown comparison op {op_tok.value!r}")
        return IRApply(
            head,
            (self._compile_node(lhs_ast), self._compile_node(rhs_ast)),  # type: ignore[arg-type]
        )

    # ---- additive / multiplicative ---------------------------------------

    def _compile_binary_chain(self, node: ASTNode) -> IRNode:
        """Handle left-associative `operand (OP operand)*`.

        The children alternate: operand, op, operand, op, operand, ...
        We reduce pairwise from the left, producing nested ``IRApply``
        nodes. Nested is correct (not flattened) because ``Sub`` and
        ``Div`` are NOT associative — `a - b - c` must be `(a-b)-c`.
        """
        if len(node.children) == 1:
            return self._compile_node(node.children[0])  # type: ignore[arg-type]
        result: IRNode | None = None
        pending_op: IRSymbol | None = None
        for child in node.children:
            if isinstance(child, Token):
                head = _BINARY_OP_HEADS.get(_token_type_name(child))
                if head is None:
                    raise CompileError(f"unknown binary op {child.value!r}")
                pending_op = head
                continue
            value = self._compile_node(child)
            if result is None:
                result = value
            else:
                assert pending_op is not None
                result = IRApply(pending_op, (result, value))
                pending_op = None
        assert result is not None
        return result

    # ---- unary -----------------------------------------------------------

    def _compile_unary(self, node: ASTNode) -> IRNode:
        # 1 child → plain power; 2 children → (MINUS|PLUS) unary.
        if len(node.children) == 1:
            return self._compile_node(node.children[0])  # type: ignore[arg-type]
        op_tok, inner = node.children
        inner_ir = self._compile_node(inner)  # type: ignore[arg-type]
        if not isinstance(op_tok, Token):
            raise CompileError("unary op must be a Token")
        if _token_type_name(op_tok) == "MINUS":
            return IRApply(NEG, (inner_ir,))
        # Unary plus is a no-op.
        return inner_ir

    # ---- power -----------------------------------------------------------

    def _compile_power(self, node: ASTNode) -> IRNode:
        # 1 child → postfix; 3 children → base CARET|STAREQ exponent.
        if len(node.children) == 1:
            return self._compile_node(node.children[0])  # type: ignore[arg-type]
        if len(node.children) != 3:
            raise CompileError("malformed power node")
        base, _op, exp = node.children
        return IRApply(
            POW,
            (self._compile_node(base), self._compile_node(exp)),  # type: ignore[arg-type]
        )

    # ---- postfix (function call) -----------------------------------------

    def _compile_postfix(self, node: ASTNode) -> IRNode:
        # 1 child → plain atom.
        # Otherwise: atom LPAREN [arglist] RPAREN { LPAREN [arglist] RPAREN }
        if len(node.children) == 1:
            return self._compile_node(node.children[0])  # type: ignore[arg-type]

        # The first child is the atom; everything after comes in groups
        # of either (LPAREN, RPAREN) for no-arg calls or
        # (LPAREN, arglist, RPAREN) for argful calls.
        result = self._compile_node(node.children[0])  # type: ignore[arg-type]

        i = 1
        while i < len(node.children):
            child = node.children[i]
            # Expect LPAREN here.
            if not (isinstance(child, Token) and _token_type_name(child) == "LPAREN"):
                raise CompileError("expected LPAREN in postfix")
            i += 1
            # Next is either RPAREN (no args) or arglist.
            args: tuple[IRNode, ...] = ()
            next_child = node.children[i]
            if isinstance(next_child, ASTNode) and next_child.rule_name == "arglist":
                args = self._compile_arglist(next_child)
                i += 1
                # Then RPAREN.
                if not (
                    isinstance(node.children[i], Token)
                    and _token_type_name(node.children[i]) == "RPAREN"  # type: ignore[arg-type]
                ):
                    raise CompileError("expected RPAREN after arglist")
                i += 1
            else:
                # Empty arglist: next child should be RPAREN.
                if not (
                    isinstance(next_child, Token)
                    and _token_type_name(next_child) == "RPAREN"
                ):
                    raise CompileError("expected RPAREN in empty call")
                i += 1

            # Substitute well-known function names with canonical heads.
            head: IRNode = result
            if isinstance(result, IRSymbol):
                head = _STANDARD_FUNCTIONS.get(result.name, result)
            result = IRApply(head, args)
        return result

    def _compile_arglist(self, node: ASTNode) -> tuple[IRNode, ...]:
        # `arglist = expression { COMMA expression } ;`
        out: list[IRNode] = []
        for child in node.children:
            if isinstance(child, Token):
                continue
            out.append(self._compile_node(child))
        return tuple(out)

    # ---- list literal ----------------------------------------------------

    def _compile_list(self, node: ASTNode) -> IRNode:
        # `list = LBRACKET [ arglist ] RBRACKET ;`
        for child in node.children:
            if isinstance(child, ASTNode) and child.rule_name == "arglist":
                return IRApply(LIST, self._compile_arglist(child))
        return IRApply(LIST, ())

    # ---- group -----------------------------------------------------------

    def _compile_group(self, node: ASTNode) -> IRNode:
        # `group = LPAREN expression RPAREN ;` — extract the middle.
        for child in node.children:
            if isinstance(child, ASTNode):
                return self._compile_node(child)
        raise CompileError("empty group")

    # ---- atom (rarely reached at top level) ------------------------------

    def _compile_atom(self, node: ASTNode) -> IRNode:
        # `atom` should have been unwrapped already in most paths. If
        # we reach here, its only child is a token or a group/list.
        if len(node.children) != 1:
            raise CompileError("atom with non-1 children")
        return self._compile_node(node.children[0])  # type: ignore[arg-type]

    # ---- dispatch table --------------------------------------------------

    _handlers: dict[str, callable] = {  # type: ignore[type-arg]
        "assign": _compile_assign,
        "logical_or": _compile_logical_or,
        "logical_and": _compile_logical_and,
        "logical_not": _compile_logical_not,
        "comparison": _compile_comparison,
        "additive": _compile_binary_chain,
        "multiplicative": _compile_binary_chain,
        "unary": _compile_unary,
        "power": _compile_power,
        "postfix": _compile_postfix,
        "atom": _compile_atom,
        "group": _compile_group,
        "list": _compile_list,
    }


# ---------------------------------------------------------------------------
# Module-level convenience wrappers
# ---------------------------------------------------------------------------


def compile_macsyma(
    ast: ASTNode, *, wrap_terminators: bool = False
) -> list[IRNode]:
    """Compile a MACSYMA ``program`` AST to a list of IR statements.

    Args:
        ast: The root ``ASTNode`` produced by ``parse_macsyma``.
        wrap_terminators: If True, every top-level statement is wrapped
            in ``Display(expr)`` (for ``;``) or ``Suppress(expr)`` (for
            ``$``). The MACSYMA REPL passes True; consumers that drive
            the VM directly (e.g. test harnesses for substrate handlers)
            leave it False to receive raw expressions.

    Returns:
        One ``IRNode`` per top-level statement. Empty programs return
        an empty list.
    """
    return Compiler(wrap_terminators=wrap_terminators).compile_program(ast)


def compile_expression(ast: ASTNode | Token) -> IRNode:
    """Compile a single expression AST (no enclosing statement) to IR.

    Useful for tests and for callers that parse expressions
    individually rather than whole programs.
    """
    return Compiler()._compile_node(ast)
