"""macsyma-compiler Phase G (control flow) tests.

These tests verify that the six new MACSYMA control-flow constructs
added by ``macsyma-grammar-extensions.md`` (Phase G) compile to the
correct ``IRApply`` shapes.  Each test drives the full parse → compile
pipeline so that the grammar, lexer, parser, and compiler are all
exercised together.

Constructs covered
------------------
* ``if / elseif / else``  → :data:`~symbolic_ir.IF`
* ``for NAME in list do`` → :data:`~symbolic_ir.FOR_EACH`
* ``for NAME thru|while|unless expr do`` → :data:`~symbolic_ir.FOR_RANGE`
* ``while cond do``       → :data:`~symbolic_ir.WHILE`
* ``block(…)``            → :data:`~symbolic_ir.BLOCK`
* ``return(expr)``        → :data:`~symbolic_ir.RETURN`

Also adds edge-case tests for small uncovered paths in the existing
compiler: ``true``/``false`` boolean literals, ``not`` operator, and
the module-level :func:`~macsyma_compiler.compiler.compile_expression`
convenience wrapper.
"""

from __future__ import annotations

from macsyma_parser import parse_macsyma
from symbolic_ir import (
    ADD,
    ASSIGN,
    BLOCK,
    EQUAL,
    FOR_EACH,
    FOR_RANGE,
    GREATER,
    IF,
    LESS,
    LIST,
    NEG,
    RETURN,
    SUB,
    WHILE,
    IRApply,
    IRInteger,
    IRSymbol,
)
from symbolic_ir.nodes import NOT

from macsyma_compiler import compile_macsyma
from macsyma_compiler.compiler import compile_expression

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def compile_one(source: str):
    """Compile a one-statement MACSYMA source and return its IR node."""
    statements = compile_macsyma(parse_macsyma(source))
    assert len(statements) == 1, f"expected 1 statement, got {len(statements)}"
    return statements[0]


# ---------------------------------------------------------------------------
# if / elseif / else
# ---------------------------------------------------------------------------


class TestIfExpr:
    """IR shapes for the ``if … then … [elseif … then …] [else …]`` form."""

    def test_if_then_no_else(self) -> None:
        """``if c then t`` → ``If(c, t)``  (2-arg form; VM returns False for miss)."""
        result = compile_one("if x > 0 then x;")
        expected = IRApply(
            IF,
            (
                IRApply(GREATER, (IRSymbol("x"), IRInteger(0))),
                IRSymbol("x"),
            ),
        )
        assert result == expected

    def test_if_then_else(self) -> None:
        """``if c then t else e`` → ``If(c, t, e)``."""
        result = compile_one("if x > 0 then x else 0;")
        expected = IRApply(
            IF,
            (
                IRApply(GREATER, (IRSymbol("x"), IRInteger(0))),
                IRSymbol("x"),
                IRInteger(0),
            ),
        )
        assert result == expected

    def test_if_elseif_then_else(self) -> None:
        """``if c then t elseif c2 then t2 else e`` → right-nested ``If``."""
        result = compile_one("if x > 0 then 1 elseif x < 0 then -1 else 0;")
        inner = IRApply(
            IF,
            (
                IRApply(LESS, (IRSymbol("x"), IRInteger(0))),
                IRApply(NEG, (IRInteger(1),)),
                IRInteger(0),
            ),
        )
        expected = IRApply(
            IF,
            (
                IRApply(GREATER, (IRSymbol("x"), IRInteger(0))),
                IRInteger(1),
                inner,
            ),
        )
        assert result == expected

    def test_if_multiple_elseif_clauses(self) -> None:
        """Three conditions nest into ``If(a,1,If(b,2,If(c,3,4)))``."""
        result = compile_one("if a then 1 elseif b then 2 elseif c then 3 else 4;")
        innermost = IRApply(IF, (IRSymbol("c"), IRInteger(3), IRInteger(4)))
        mid = IRApply(IF, (IRSymbol("b"), IRInteger(2), innermost))
        expected = IRApply(IF, (IRSymbol("a"), IRInteger(1), mid))
        assert result == expected

    def test_if_elseif_no_else(self) -> None:
        """``if a then 1 elseif b then 2`` — no ``else``, innermost has 2 args."""
        result = compile_one("if a then 1 elseif b then 2;")
        inner = IRApply(IF, (IRSymbol("b"), IRInteger(2)))
        expected = IRApply(IF, (IRSymbol("a"), IRInteger(1), inner))
        assert result == expected

    def test_if_branch_is_arithmetic(self) -> None:
        """Branch bodies can be arbitrary expressions."""
        result = compile_one("if x = 1 then x + 1 else x - 1;")
        expected = IRApply(
            IF,
            (
                IRApply(EQUAL, (IRSymbol("x"), IRInteger(1))),
                IRApply(ADD, (IRSymbol("x"), IRInteger(1))),
                IRApply(SUB, (IRSymbol("x"), IRInteger(1))),
            ),
        )
        assert result == expected

    def test_if_condition_is_equality(self) -> None:
        """Equality comparison in the condition."""
        result = compile_one("if a = b then 1 else 2;")
        expected = IRApply(
            IF,
            (
                IRApply(EQUAL, (IRSymbol("a"), IRSymbol("b"))),
                IRInteger(1),
                IRInteger(2),
            ),
        )
        assert result == expected

    def test_if_body_is_assignment(self) -> None:
        """Branch bodies can be assignment statements."""
        result = compile_one("if x > 0 then y : 1 else y : -1;")
        expected = IRApply(
            IF,
            (
                IRApply(GREATER, (IRSymbol("x"), IRInteger(0))),
                IRApply(ASSIGN, (IRSymbol("y"), IRInteger(1))),
                IRApply(ASSIGN, (IRSymbol("y"), IRApply(NEG, (IRInteger(1),)))),
            ),
        )
        assert result == expected


# ---------------------------------------------------------------------------
# for NAME in list do body  (ForEach)
# ---------------------------------------------------------------------------


class TestForEachExpr:
    """IR shapes for ``for NAME in list do body``."""

    def test_basic_for_each(self) -> None:
        """``for i in [1,2,3] do i`` → ``ForEach(i, List(1,2,3), i)``."""
        result = compile_one("for i in [1, 2, 3] do i;")
        expected = IRApply(
            FOR_EACH,
            (
                IRSymbol("i"),
                IRApply(LIST, (IRInteger(1), IRInteger(2), IRInteger(3))),
                IRSymbol("i"),
            ),
        )
        assert result == expected

    def test_for_each_symbol_list(self) -> None:
        """List can be a symbol (evaluated at runtime)."""
        result = compile_one("for x in xs do x;")
        expected = IRApply(FOR_EACH, (IRSymbol("x"), IRSymbol("xs"), IRSymbol("x")))
        assert result == expected

    def test_for_each_empty_list(self) -> None:
        """Empty literal list is handled gracefully."""
        result = compile_one("for i in [] do i;")
        expected = IRApply(
            FOR_EACH,
            (IRSymbol("i"), IRApply(LIST, ()), IRSymbol("i")),
        )
        assert result == expected

    def test_for_each_assignment_body(self) -> None:
        """Body can be an assignment."""
        result = compile_one("for x in xs do s : s + x;")
        expected = IRApply(
            FOR_EACH,
            (
                IRSymbol("x"),
                IRSymbol("xs"),
                IRApply(
                    ASSIGN,
                    (
                        IRSymbol("s"),
                        IRApply(ADD, (IRSymbol("s"), IRSymbol("x"))),
                    ),
                ),
            ),
        )
        assert result == expected

    def test_for_each_return_in_body(self) -> None:
        """Return inside a for-each body."""
        result = compile_one("for i in items do return(i);")
        expected = IRApply(
            FOR_EACH,
            (
                IRSymbol("i"),
                IRSymbol("items"),
                IRApply(RETURN, (IRSymbol("i"),)),
            ),
        )
        assert result == expected


# ---------------------------------------------------------------------------
# for NAME [start] [step] thru|while|unless end do body  (ForRange)
# ---------------------------------------------------------------------------


class TestForRangeExpr:
    """IR shapes for the range-based ``for`` loop.

    ``ForRange(var, start, step, end, body)`` — start and step default
    to ``IRInteger(1)`` when the optional grammar clauses are absent.
    """

    def test_basic_for_thru(self) -> None:
        """``for i thru 10 do i`` → default start=1, step=1."""
        result = compile_one("for i thru 10 do i;")
        expected = IRApply(
            FOR_RANGE,
            (
                IRSymbol("i"),
                IRInteger(1),
                IRInteger(1),
                IRInteger(10),
                IRSymbol("i"),
            ),
        )
        assert result == expected

    def test_for_with_explicit_start(self) -> None:
        """``for i:5 thru 10 do i`` — explicit start, default step."""
        result = compile_one("for i:5 thru 10 do i;")
        expected = IRApply(
            FOR_RANGE,
            (
                IRSymbol("i"),
                IRInteger(5),
                IRInteger(1),
                IRInteger(10),
                IRSymbol("i"),
            ),
        )
        assert result == expected

    def test_for_with_step_only(self) -> None:
        """``for i step 2 thru 10 do i`` — default start, explicit step."""
        result = compile_one("for i step 2 thru 10 do i;")
        expected = IRApply(
            FOR_RANGE,
            (
                IRSymbol("i"),
                IRInteger(1),
                IRInteger(2),
                IRInteger(10),
                IRSymbol("i"),
            ),
        )
        assert result == expected

    def test_for_with_start_and_step(self) -> None:
        """``for i:1 step 2 thru 10 do i`` — both start and step explicit."""
        result = compile_one("for i:1 step 2 thru 10 do i;")
        expected = IRApply(
            FOR_RANGE,
            (
                IRSymbol("i"),
                IRInteger(1),
                IRInteger(2),
                IRInteger(10),
                IRSymbol("i"),
            ),
        )
        assert result == expected

    def test_for_while_terminator(self) -> None:
        """``while`` terminator in a range-for stores condition as end."""
        result = compile_one("for i:1 while i < 10 do i;")
        expected = IRApply(
            FOR_RANGE,
            (
                IRSymbol("i"),
                IRInteger(1),
                IRInteger(1),
                IRApply(LESS, (IRSymbol("i"), IRInteger(10))),
                IRSymbol("i"),
            ),
        )
        assert result == expected

    def test_for_unless_terminator(self) -> None:
        """``unless`` terminator in a range-for."""
        result = compile_one("for i:1 unless i > 10 do i;")
        expected = IRApply(
            FOR_RANGE,
            (
                IRSymbol("i"),
                IRInteger(1),
                IRInteger(1),
                IRApply(GREATER, (IRSymbol("i"), IRInteger(10))),
                IRSymbol("i"),
            ),
        )
        assert result == expected

    def test_for_body_is_assign(self) -> None:
        """``for i thru 5 do s : s + i``."""
        result = compile_one("for i thru 5 do s : s + i;")
        expected = IRApply(
            FOR_RANGE,
            (
                IRSymbol("i"),
                IRInteger(1),
                IRInteger(1),
                IRInteger(5),
                IRApply(
                    ASSIGN,
                    (
                        IRSymbol("s"),
                        IRApply(ADD, (IRSymbol("s"), IRSymbol("i"))),
                    ),
                ),
            ),
        )
        assert result == expected

    def test_for_body_has_return(self) -> None:
        """Return inside a range-for body."""
        result = compile_one("for i thru 10 do return(i);")
        expected = IRApply(
            FOR_RANGE,
            (
                IRSymbol("i"),
                IRInteger(1),
                IRInteger(1),
                IRInteger(10),
                IRApply(RETURN, (IRSymbol("i"),)),
            ),
        )
        assert result == expected


# ---------------------------------------------------------------------------
# while cond do body  (While)
# ---------------------------------------------------------------------------


class TestWhileExpr:
    """IR shapes for ``while cond do body``."""

    def test_basic_while(self) -> None:
        """``while x > 0 do x : x - 1``."""
        result = compile_one("while x > 0 do x : x - 1;")
        expected = IRApply(
            WHILE,
            (
                IRApply(GREATER, (IRSymbol("x"), IRInteger(0))),
                IRApply(
                    ASSIGN,
                    (IRSymbol("x"), IRApply(SUB, (IRSymbol("x"), IRInteger(1)))),
                ),
            ),
        )
        assert result == expected

    def test_while_with_true_literal(self) -> None:
        """Infinite loop: ``while true do 1``."""
        result = compile_one("while true do 1;")
        expected = IRApply(WHILE, (IRSymbol("True"), IRInteger(1)))
        assert result == expected

    def test_while_body_contains_return(self) -> None:
        """Return inside a while body."""
        result = compile_one("while x > 0 do return(x);")
        expected = IRApply(
            WHILE,
            (
                IRApply(GREATER, (IRSymbol("x"), IRInteger(0))),
                IRApply(RETURN, (IRSymbol("x"),)),
            ),
        )
        assert result == expected

    def test_while_condition_is_equality(self) -> None:
        """Equality condition."""
        result = compile_one("while a = b do a : a + 1;")
        expected = IRApply(
            WHILE,
            (
                IRApply(EQUAL, (IRSymbol("a"), IRSymbol("b"))),
                IRApply(
                    ASSIGN,
                    (IRSymbol("a"), IRApply(ADD, (IRSymbol("a"), IRInteger(1)))),
                ),
            ),
        )
        assert result == expected


# ---------------------------------------------------------------------------
# block([locals], stmts…)  (Block)
# ---------------------------------------------------------------------------


class TestBlockExpr:
    """IR shapes for ``block([locals], stmt1, stmt2, …)``."""

    def test_block_empty_locals_with_stmt(self) -> None:
        """``block([s], s : 0, s)`` → ``Block(List(s), Assign(s,0), s)``."""
        result = compile_one("block([s], s : 0, s);")
        expected = IRApply(
            BLOCK,
            (
                IRApply(LIST, (IRSymbol("s"),)),
                IRApply(ASSIGN, (IRSymbol("s"), IRInteger(0))),
                IRSymbol("s"),
            ),
        )
        assert result == expected

    def test_block_no_locals_list(self) -> None:
        """``block(x, y)`` — no ``[…]`` first arg; compiler prepends ``List()``."""
        result = compile_one("block(x, y);")
        expected = IRApply(
            BLOCK,
            (
                IRApply(LIST, ()),
                IRSymbol("x"),
                IRSymbol("y"),
            ),
        )
        assert result == expected

    def test_block_empty_locals_list_only(self) -> None:
        """``block([])`` → ``Block(List())``, no statements."""
        result = compile_one("block([]);")
        expected = IRApply(BLOCK, (IRApply(LIST, ()),))
        assert result == expected

    def test_block_local_with_initializer(self) -> None:
        """``block([x:0], x)`` — local declared with an initial value."""
        result = compile_one("block([x:0], x);")
        expected = IRApply(
            BLOCK,
            (
                IRApply(LIST, (IRApply(ASSIGN, (IRSymbol("x"), IRInteger(0))),)),
                IRSymbol("x"),
            ),
        )
        assert result == expected

    def test_block_multiple_locals(self) -> None:
        """``block([x, y], x + y)``."""
        result = compile_one("block([x, y], x + y);")
        expected = IRApply(
            BLOCK,
            (
                IRApply(LIST, (IRSymbol("x"), IRSymbol("y"))),
                IRApply(ADD, (IRSymbol("x"), IRSymbol("y"))),
            ),
        )
        assert result == expected

    def test_block_nested(self) -> None:
        """``block([x], block([y], x + y))`` — nested blocks compile correctly."""
        result = compile_one("block([x], block([y], x + y));")
        inner = IRApply(
            BLOCK,
            (
                IRApply(LIST, (IRSymbol("y"),)),
                IRApply(ADD, (IRSymbol("x"), IRSymbol("y"))),
            ),
        )
        expected = IRApply(
            BLOCK,
            (
                IRApply(LIST, (IRSymbol("x"),)),
                inner,
            ),
        )
        assert result == expected

    def test_block_body_has_return(self) -> None:
        """``block([s], s : 1, return(s))``."""
        result = compile_one("block([s], s : 1, return(s));")
        expected = IRApply(
            BLOCK,
            (
                IRApply(LIST, (IRSymbol("s"),)),
                IRApply(ASSIGN, (IRSymbol("s"), IRInteger(1))),
                IRApply(RETURN, (IRSymbol("s"),)),
            ),
        )
        assert result == expected


# ---------------------------------------------------------------------------
# return(expr)  (Return)
# ---------------------------------------------------------------------------


class TestReturnExpr:
    """IR shapes for ``return(expr)``."""

    def test_return_integer(self) -> None:
        """``return(42)`` → ``Return(42)``."""
        result = compile_one("return(42);")
        assert result == IRApply(RETURN, (IRInteger(42),))

    def test_return_symbol(self) -> None:
        """``return(x)`` → ``Return(x)``."""
        result = compile_one("return(x);")
        assert result == IRApply(RETURN, (IRSymbol("x"),))

    def test_return_expression(self) -> None:
        """``return(x + 1)`` → ``Return(Add(x, 1))``."""
        result = compile_one("return(x + 1);")
        expected = IRApply(
            RETURN,
            (IRApply(ADD, (IRSymbol("x"), IRInteger(1))),),
        )
        assert result == expected


# ---------------------------------------------------------------------------
# Additional coverage for pre-existing code paths
# ---------------------------------------------------------------------------


class TestBooleanLiterals:
    """Cover the ``true`` / ``false`` keyword token paths in ``_compile_token``."""

    def test_true_literal(self) -> None:
        """``true`` → ``IRSymbol("True")``."""
        assert compile_one("true;") == IRSymbol("True")

    def test_false_literal(self) -> None:
        """``false`` → ``IRSymbol("False")``."""
        assert compile_one("false;") == IRSymbol("False")


class TestLogicalNot:
    """Cover the ``not expr`` path in ``_compile_logical_not``."""

    def test_not_expression(self) -> None:
        """``not x`` → ``IRApply(NOT, (x,))``."""
        result = compile_one("not x;")
        assert result == IRApply(NOT, (IRSymbol("x"),))

    def test_not_comparison(self) -> None:
        """``not (a = b)``."""
        result = compile_one("not (a = b);")
        assert result == IRApply(
            NOT,
            (IRApply(EQUAL, (IRSymbol("a"), IRSymbol("b"))),),
        )


class TestCompileExpressionWrapper:
    """Cover the module-level ``compile_expression`` convenience function (line 824)."""

    def test_compile_expression_returns_single_node(self) -> None:
        """``compile_expression`` compiles a single expression node to IR."""
        ast = parse_macsyma("x + 1;")
        # ast.children[0] is the statement; .children[0] is the expression
        expr_node = ast.children[0].children[0]
        result = compile_expression(expr_node)
        assert result == IRApply(ADD, (IRSymbol("x"), IRInteger(1)))

    def test_compile_expression_atom(self) -> None:
        """``compile_expression`` on a bare symbol token."""
        ast = parse_macsyma("x;")
        expr_node = ast.children[0].children[0]
        result = compile_expression(expr_node)
        assert result == IRSymbol("x")


# ---------------------------------------------------------------------------
# Canonical end-to-end: block + for-range accumulation
# ---------------------------------------------------------------------------


class TestCanonicalBlockForRange:
    """Block + ForRange: compute sum 1..N via ``block``."""

    def test_sum_one_to_five(self) -> None:
        """
        ``block([s:0], for i thru 5 do s : s + i, s)``
        compiles to the IR shape the VM will evaluate to 15.
        """
        result = compile_one("block([s:0], for i thru 5 do s : s + i, s);")
        for_body = IRApply(
            ASSIGN,
            (
                IRSymbol("s"),
                IRApply(ADD, (IRSymbol("s"), IRSymbol("i"))),
            ),
        )
        for_loop = IRApply(
            FOR_RANGE,
            (
                IRSymbol("i"),
                IRInteger(1),
                IRInteger(1),
                IRInteger(5),
                for_body,
            ),
        )
        expected = IRApply(
            BLOCK,
            (
                IRApply(LIST, (IRApply(ASSIGN, (IRSymbol("s"), IRInteger(0))),)),
                for_loop,
                IRSymbol("s"),
            ),
        )
        assert result == expected
