"""Phase G — Control-flow head handler tests.

Tests the five new VM handlers (While, ForRange, ForEach, Block, Return)
and verifies end-to-end behaviour through the Macsyma string interface.

Test organisation
-----------------

Class                       What it covers
--------------------------  -----------------------------------------------
TestWhile                   While handler: basic loop, early Return, symbolic
TestForRange                ForRange handler: basic, step, default args
TestForEach                 ForEach handler: list iteration, early Return
TestBlock                   Block handler: scope save/restore, Return, nesting
TestReturn                  Return handler: _ReturnSignal raised correctly
TestEndToEnd                parse → compile → VM using macsyma string source
TestCompilerShapes          Compiler produces correct IR for each production
TestRegressions             Previously-working tests are not broken
"""

from __future__ import annotations

import pytest
from macsyma_compiler import compile_macsyma
from macsyma_parser import parse_macsyma
from symbolic_ir import (
    ADD,
    ASSIGN,
    BLOCK,
    EQUAL,
    FOR_EACH,
    FOR_RANGE,
    IF,
    LIST,
    RETURN,
    WHILE,
    IRApply,
    IRInteger,
    IRSymbol,
)
from symbolic_ir.nodes import LESS

from symbolic_vm import VM, StrictBackend, SymbolicBackend
from symbolic_vm.handlers import _ReturnSignal

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

TRUE = IRSymbol("True")
FALSE = IRSymbol("False")
ONE = IRInteger(1)
ZERO = IRInteger(0)


def _sb() -> StrictBackend:
    """Fresh strict backend."""
    return StrictBackend()


def _yb() -> SymbolicBackend:
    """Fresh symbolic backend."""
    return SymbolicBackend()


def _compile(source: str):
    """Parse + compile MACSYMA source; return list of IR statements."""
    return compile_macsyma(parse_macsyma(source))


def _compile_one(source: str):
    """Parse + compile; return single IR statement (assertion on count)."""
    stmts = _compile(source)
    assert len(stmts) == 1, f"expected 1 statement, got {len(stmts)}"
    return stmts[0]


def _eval(source: str, backend=None):
    """Parse + compile + evaluate first statement."""
    if backend is None:
        backend = SymbolicBackend()
    vm = VM(backend)
    stmts = _compile(source)
    result = None
    for stmt in stmts:
        result = vm.eval(stmt)
    return result


# ---------------------------------------------------------------------------
# TestWhile — While(condition, body)
# ---------------------------------------------------------------------------


class TestWhile:
    """VM-level tests for the While handler."""

    def test_basic_loop_counts_up(self) -> None:
        """While keeps running until condition is false."""
        # Equivalent to:  i : 0;  while i < 3 do i : i + 1;
        # We drive this through the VM directly.
        backend = _sb()
        vm = VM(backend)
        backend.bind("i", ZERO)

        # condition: i < 3
        cond = IRApply(LESS, (IRSymbol("i"), IRInteger(3)))
        # body: i : i + 1
        body = IRApply(ASSIGN, (IRSymbol("i"), IRApply(ADD, (IRSymbol("i"), ONE))))
        expr = IRApply(WHILE, (cond, body))
        result = vm.eval(expr)
        assert result == IRInteger(3)
        assert backend.lookup("i") == IRInteger(3)

    def test_loop_never_entered(self) -> None:
        """While body is never evaluated when condition is already false."""
        backend = _sb()
        vm = VM(backend)
        backend.bind("i", IRInteger(5))
        cond = IRApply(LESS, (IRSymbol("i"), IRInteger(3)))
        body = IRApply(ASSIGN, (IRSymbol("i"), IRInteger(0)))  # would reset i
        result = vm.eval(IRApply(WHILE, (cond, body)))
        # Condition is false from the start; returns False and doesn't run body.
        assert result == FALSE
        assert backend.lookup("i") == IRInteger(5)

    def test_loop_with_return(self) -> None:
        """Return fires directly in the While body, exiting the loop.

        ``return`` exits the nearest enclosing control-flow construct.
        When the return expression is the *direct* body of a While (not
        wrapped in a nested Block), the ``while_`` handler catches the
        ``_ReturnSignal`` and returns the payload.

        If the while body were a ``Block(…)``, the block handler would
        catch the signal first — that is tested separately in
        ``TestBlock.test_block_with_return``.
        """
        backend = _yb()
        vm = VM(backend)
        # Bind i = 2 so the very first iteration immediately fires the return.
        backend.bind("i", IRInteger(2))

        # condition: i < 10  (always True for i=2)
        cond = IRApply(LESS, (IRSymbol("i"), IRInteger(10)))
        # body: if True then return(99)  (no nested block — return propagates to while_)
        body = IRApply(IF, (TRUE, IRApply(RETURN, (IRInteger(99),))))
        result = vm.eval(IRApply(WHILE, (cond, body)))
        assert result == IRInteger(99)

    def test_symbolic_condition_leaves_unevaluated(self) -> None:
        """Symbolic condition causes While to return itself unevaluated."""
        vm = VM(_yb())
        x = IRSymbol("x")
        cond = IRApply(LESS, (x, IRInteger(10)))  # x is unbound → symbolic
        body = IRInteger(1)
        expr = IRApply(WHILE, (cond, body))
        result = vm.eval(expr)
        assert result == expr

    def test_while_wrong_arity_raises(self) -> None:
        vm = VM(_sb())
        with pytest.raises(TypeError, match="While expects 2 arguments"):
            vm.eval(IRApply(WHILE, (TRUE,)))

    def test_while_exceeds_iteration_limit(self) -> None:
        """While raises RuntimeError when MAX_LOOP_ITERATIONS is exceeded."""
        import symbolic_vm.handlers as _hmod
        old_limit = _hmod.MAX_LOOP_ITERATIONS
        try:
            _hmod.MAX_LOOP_ITERATIONS = 5  # tiny limit for the test
            vm = VM(_yb())
            # `while true do 1` — infinite loop, hits the cap at 5 iters
            expr = IRApply(WHILE, (TRUE, ONE))
            with pytest.raises(RuntimeError, match="While loop exceeded"):
                vm.eval(expr)
        finally:
            _hmod.MAX_LOOP_ITERATIONS = old_limit


# ---------------------------------------------------------------------------
# TestForRange — ForRange(var, start, step, end, body)
# ---------------------------------------------------------------------------


class TestForRange:
    """VM-level tests for the ForRange handler."""

    def test_basic_range_returns_last_body_value(self) -> None:
        """for i: 1 thru 5 do i^2 returns 25 (the last value)."""
        from symbolic_ir import POW
        vm = VM(_yb())
        var = IRSymbol("i")
        body = IRApply(POW, (IRSymbol("i"), IRInteger(2)))
        expr = IRApply(FOR_RANGE, (var, ONE, ONE, IRInteger(5), body))
        result = vm.eval(expr)
        assert result == IRInteger(25)

    def test_sum_inside_loop(self) -> None:
        """Accumulate a sum via assignment inside a ForRange loop."""
        backend = _yb()
        vm = VM(backend)
        backend.bind("s", ZERO)

        var = IRSymbol("i")
        body = IRApply(ASSIGN, (
            IRSymbol("s"),
            IRApply(ADD, (IRSymbol("s"), IRSymbol("i"))),
        ))
        expr = IRApply(FOR_RANGE, (var, ONE, ONE, IRInteger(5), body))
        vm.eval(expr)
        assert backend.lookup("s") == IRInteger(15)

    def test_loop_variable_restored_after(self) -> None:
        """ForRange restores the loop variable's binding on exit."""
        backend = _yb()
        vm = VM(backend)
        backend.bind("i", IRInteger(42))

        var = IRSymbol("i")
        body = IRSymbol("i")
        vm.eval(IRApply(FOR_RANGE, (var, ONE, ONE, IRInteger(3), body)))
        # Binding should be restored to 42.
        assert backend.lookup("i") == IRInteger(42)

    def test_loop_variable_unbound_after_if_not_set_before(self) -> None:
        """ForRange unbinds the loop variable if it was unbound before."""
        backend = _yb()
        vm = VM(backend)
        # 'j' is not bound initially.
        assert backend.lookup("j") is None

        var = IRSymbol("j")
        body = IRSymbol("j")
        vm.eval(IRApply(FOR_RANGE, (var, ONE, ONE, IRInteger(3), body)))
        # Should be unbound again.
        assert backend.lookup("j") is None

    def test_step_two(self) -> None:
        """ForRange with step=2 visits only odd values 1,3,5."""
        vm = VM(_yb())
        var = IRSymbol("k")
        body = IRSymbol("k")  # returns current k
        expr = IRApply(FOR_RANGE, (var, ONE, IRInteger(2), IRInteger(5), body))
        result = vm.eval(expr)
        assert result == IRInteger(5)  # last value visited

    def test_empty_range_returns_false(self) -> None:
        """ForRange with start > end returns False (loop never entered)."""
        vm = VM(_yb())
        var = IRSymbol("m")
        body = IRInteger(999)
        expr = IRApply(FOR_RANGE, (var, IRInteger(5), ONE, IRInteger(3), body))
        assert vm.eval(expr) == FALSE

    def test_for_range_with_return(self) -> None:
        """Return inside ForRange exits early."""
        vm = VM(_yb())
        var = IRSymbol("n")
        # body: if n = 3 then return(n) else false
        body = IRApply(IF, (
            IRApply(EQUAL, (IRSymbol("n"), IRInteger(3))),
            IRApply(RETURN, (IRSymbol("n"),)),
        ))
        expr = IRApply(FOR_RANGE, (var, ONE, ONE, IRInteger(10), body))
        assert vm.eval(expr) == IRInteger(3)

    def test_symbolic_bounds_leave_unevaluated(self) -> None:
        """Symbolic start/step/end causes ForRange to return unevaluated."""
        vm = VM(_yb())
        x = IRSymbol("x")
        var = IRSymbol("i")
        body = IRInteger(1)
        expr = IRApply(FOR_RANGE, (var, x, ONE, IRInteger(5), body))
        assert vm.eval(expr) == expr

    def test_for_range_wrong_arity_raises(self) -> None:
        vm = VM(_sb())
        with pytest.raises(TypeError, match="ForRange expects 5 arguments"):
            vm.eval(IRApply(FOR_RANGE, (IRSymbol("i"), ONE)))

    def test_for_range_exceeds_iteration_limit(self) -> None:
        """ForRange raises RuntimeError when MAX_LOOP_ITERATIONS is exceeded."""
        import symbolic_vm.handlers as _hmod
        old_limit = _hmod.MAX_LOOP_ITERATIONS
        try:
            _hmod.MAX_LOOP_ITERATIONS = 3
            vm = VM(_yb())
            # for i: 1 thru 1000 do i — 1000 iterations, cap is 3
            var = IRSymbol("i")
            body = IRSymbol("i")
            expr = IRApply(FOR_RANGE, (var, ONE, ONE, IRInteger(1000), body))
            with pytest.raises(RuntimeError, match="ForRange loop exceeded"):
                vm.eval(expr)
        finally:
            _hmod.MAX_LOOP_ITERATIONS = old_limit


# ---------------------------------------------------------------------------
# TestForEach — ForEach(var, list, body)
# ---------------------------------------------------------------------------


class TestForEach:
    """VM-level tests for the ForEach handler."""

    def test_basic_iteration(self) -> None:
        """ForEach returns the last body value."""
        vm = VM(_yb())
        var = IRSymbol("x")
        lst = IRApply(LIST, (IRInteger(1), IRInteger(2), IRInteger(3)))
        body = IRSymbol("x")
        result = vm.eval(IRApply(FOR_EACH, (var, lst, body)))
        assert result == IRInteger(3)

    def test_empty_list_returns_false(self) -> None:
        vm = VM(_yb())
        var = IRSymbol("x")
        lst = IRApply(LIST, ())
        body = IRInteger(99)
        assert vm.eval(IRApply(FOR_EACH, (var, lst, body))) == FALSE

    def test_loop_variable_restored(self) -> None:
        """ForEach restores the loop variable binding on exit."""
        backend = _yb()
        vm = VM(backend)
        backend.bind("x", IRInteger(7))
        var = IRSymbol("x")
        lst = IRApply(LIST, (ONE, IRInteger(2), IRInteger(3)))
        body = IRSymbol("x")
        vm.eval(IRApply(FOR_EACH, (var, lst, body)))
        assert backend.lookup("x") == IRInteger(7)

    def test_loop_variable_unbound_after_if_not_set_before(self) -> None:
        """ForEach unbinds the loop variable if it was unbound before."""
        backend = _yb()
        vm = VM(backend)
        assert backend.lookup("elem") is None
        var = IRSymbol("elem")
        lst = IRApply(LIST, (ONE, IRInteger(2)))
        body = IRSymbol("elem")
        vm.eval(IRApply(FOR_EACH, (var, lst, body)))
        assert backend.lookup("elem") is None

    def test_accumulate_sum(self) -> None:
        """Sum a list using ForEach + Assign."""
        backend = _yb()
        vm = VM(backend)
        backend.bind("total", ZERO)
        var = IRSymbol("v")
        lst = IRApply(LIST, (IRInteger(10), IRInteger(20), IRInteger(30)))
        body = IRApply(ASSIGN, (
            IRSymbol("total"),
            IRApply(ADD, (IRSymbol("total"), IRSymbol("v"))),
        ))
        vm.eval(IRApply(FOR_EACH, (var, lst, body)))
        assert backend.lookup("total") == IRInteger(60)

    def test_for_each_with_return(self) -> None:
        """Return inside ForEach exits early."""
        vm = VM(_yb())
        var = IRSymbol("v")
        lst = IRApply(LIST, (IRInteger(1), IRInteger(2), IRInteger(3), IRInteger(4)))
        body = IRApply(IF, (
            IRApply(EQUAL, (IRSymbol("v"), IRInteger(2))),
            IRApply(RETURN, (IRInteger(99),)),
        ))
        assert vm.eval(IRApply(FOR_EACH, (var, lst, body))) == IRInteger(99)

    def test_non_list_value_leaves_unevaluated(self) -> None:
        """ForEach with a symbolic list stays unevaluated."""
        vm = VM(_yb())
        var = IRSymbol("x")
        lst = IRSymbol("mylist")  # unbound symbol
        body = IRSymbol("x")
        expr = IRApply(FOR_EACH, (var, lst, body))
        assert vm.eval(expr) == expr

    def test_for_each_wrong_arity_raises(self) -> None:
        vm = VM(_sb())
        with pytest.raises(TypeError, match="ForEach expects 3 arguments"):
            vm.eval(IRApply(FOR_EACH, (IRSymbol("x"),)))


# ---------------------------------------------------------------------------
# TestBlock — Block(List(locals), stmt1, …)
# ---------------------------------------------------------------------------


class TestBlock:
    """VM-level tests for the Block handler."""

    def test_empty_block_returns_false(self) -> None:
        vm = VM(_yb())
        assert vm.eval(IRApply(BLOCK, (IRApply(LIST, ()),))) == FALSE

    def test_single_stmt_returns_its_value(self) -> None:
        vm = VM(_yb())
        expr = IRApply(BLOCK, (IRApply(LIST, ()), IRInteger(42)))
        assert vm.eval(expr) == IRInteger(42)

    def test_last_stmt_is_return_value(self) -> None:
        vm = VM(_yb())
        expr = IRApply(
            BLOCK,
            (IRApply(LIST, ()), IRInteger(1), IRInteger(2), IRInteger(3)),
        )
        assert vm.eval(expr) == IRInteger(3)

    def test_local_init_to_false(self) -> None:
        """Local variable declared without init defaults to False."""
        backend = _yb()
        vm = VM(backend)
        # block([x], x)  → x is initialized to False; returns False.
        expr = IRApply(BLOCK, (IRApply(LIST, (IRSymbol("x"),)), IRSymbol("x")))
        assert vm.eval(expr) == FALSE

    def test_local_init_with_assign(self) -> None:
        """Local variable initialized via Assign in the locals list."""
        vm = VM(_yb())
        # block([x: 5], x)  → x = 5; returns 5.
        local = IRApply(ASSIGN, (IRSymbol("x"), IRInteger(5)))
        expr = IRApply(BLOCK, (IRApply(LIST, (local,)), IRSymbol("x")))
        assert vm.eval(expr) == IRInteger(5)

    def test_scope_does_not_leak(self) -> None:
        """Block variable does not pollute the outer scope."""
        backend = _yb()
        vm = VM(backend)
        assert backend.lookup("x") is None

        local = IRApply(ASSIGN, (IRSymbol("x"), IRInteger(99)))
        inner = IRApply(BLOCK, (IRApply(LIST, (local,)), IRSymbol("x")))
        result = vm.eval(inner)
        assert result == IRInteger(99)
        # x must be unbound in the outer scope.
        assert backend.lookup("x") is None

    def test_scope_restores_outer_binding(self) -> None:
        """Block restores an outer binding after the block exits."""
        backend = _yb()
        vm = VM(backend)
        backend.bind("x", IRInteger(7))

        local = IRApply(ASSIGN, (IRSymbol("x"), IRInteger(0)))
        inner = IRApply(BLOCK, (IRApply(LIST, (local,)), IRSymbol("x")))
        result = vm.eval(inner)
        assert result == ZERO
        # Outer x restored.
        assert backend.lookup("x") == IRInteger(7)

    def test_multiple_locals(self) -> None:
        """Block with multiple locals initialises and uses all of them."""
        vm = VM(_yb())
        local_a = IRApply(ASSIGN, (IRSymbol("a"), IRInteger(3)))
        local_b = IRApply(ASSIGN, (IRSymbol("b"), IRInteger(4)))
        # block([a: 3, b: 4], a + b)  → 7
        stmt = IRApply(ADD, (IRSymbol("a"), IRSymbol("b")))
        expr = IRApply(BLOCK, (IRApply(LIST, (local_a, local_b)), stmt))
        assert vm.eval(expr) == IRInteger(7)

    def test_block_with_return(self) -> None:
        """Return inside Block exits early and returns the given value."""
        vm = VM(_yb())
        ret = IRApply(RETURN, (IRInteger(99),))
        never_reached = IRInteger(0)
        expr = IRApply(BLOCK, (IRApply(LIST, ()), ret, never_reached))
        assert vm.eval(expr) == IRInteger(99)

    def test_nested_blocks_scope(self) -> None:
        """Inner block's local doesn't shadow outer block's local permanently."""
        vm = VM(_yb())
        inner_local = IRApply(ASSIGN, (IRSymbol("x"), IRInteger(0)))
        # inner block: block([x: 0], x)  → 0
        inner = IRApply(BLOCK, (IRApply(LIST, (inner_local,)), IRSymbol("x")))
        outer_local = IRApply(ASSIGN, (IRSymbol("x"), IRInteger(99)))
        # outer block: block([x: 99], inner_block, x)  → 99
        outer = IRApply(BLOCK, (
            IRApply(LIST, (outer_local,)),
            inner,
            IRSymbol("x"),
        ))
        assert vm.eval(outer) == IRInteger(99)

    def test_block_local_init_uses_outer_scope_rhs(self) -> None:
        """RHS of a local init is evaluated in the enclosing scope."""
        backend = _yb()
        vm = VM(backend)
        backend.bind("n", IRInteger(10))
        # block([x: n], x)  → x = 10 (n looked up in outer scope before x is bound).
        local = IRApply(ASSIGN, (IRSymbol("x"), IRSymbol("n")))
        expr = IRApply(BLOCK, (IRApply(LIST, (local,)), IRSymbol("x")))
        assert vm.eval(expr) == IRInteger(10)

    def test_block_without_explicit_locals(self) -> None:
        """Block without a List first arg treats all args as stmts."""
        vm = VM(_yb())
        # Block(1, 2, 3)  — no explicit locals list.
        expr = IRApply(BLOCK, (ONE, IRInteger(2), IRInteger(3)))
        assert vm.eval(expr) == IRInteger(3)

    def test_sum_via_block_and_for_range(self) -> None:
        """Canonical test: block([s: 0], for i: 1 thru 5 do s: s+i, s) → 15."""
        vm = VM(_yb())
        s_local = IRApply(ASSIGN, (IRSymbol("s"), ZERO))
        var = IRSymbol("i")
        add_body = IRApply(ASSIGN, (
            IRSymbol("s"),
            IRApply(ADD, (IRSymbol("s"), IRSymbol("i"))),
        ))
        for_loop = IRApply(FOR_RANGE, (var, ONE, ONE, IRInteger(5), add_body))
        expr = IRApply(BLOCK, (
            IRApply(LIST, (s_local,)),
            for_loop,
            IRSymbol("s"),
        ))
        assert vm.eval(expr) == IRInteger(15)

    def test_block_invalid_local_raises(self) -> None:
        """Block raises TypeError for a non-symbol, non-Assign local entry."""
        vm = VM(_yb())
        bad_local = IRInteger(5)  # not a symbol or Assign
        expr = IRApply(BLOCK, (IRApply(LIST, (bad_local,)), IRInteger(1)))
        with pytest.raises(TypeError, match="Block: invalid local declaration"):
            vm.eval(expr)


# ---------------------------------------------------------------------------
# TestReturn — Return(value)
# ---------------------------------------------------------------------------


class TestReturn:
    """Return handler raises _ReturnSignal (standalone, no enclosing block)."""

    def test_return_raises_signal_directly(self) -> None:
        """Return raises _ReturnSignal when evaluated outside a block."""
        vm = VM(_yb())
        expr = IRApply(RETURN, (IRInteger(42),))
        with pytest.raises(_ReturnSignal) as exc_info:
            vm.eval(expr)
        assert exc_info.value.value == IRInteger(42)

    def test_return_wrong_arity_raises(self) -> None:
        vm = VM(_sb())
        with pytest.raises(TypeError, match="Return expects 1 argument"):
            vm.eval(IRApply(RETURN, ()))

    def test_return_value_is_evaluated_before_signal(self) -> None:
        """The return-value expression is evaluated before _ReturnSignal fires."""
        vm = VM(_yb())
        # Return(1 + 1)  →  _ReturnSignal(2)
        expr = IRApply(RETURN, (IRApply(ADD, (ONE, ONE)),))
        with pytest.raises(_ReturnSignal) as exc_info:
            vm.eval(expr)
        assert exc_info.value.value == IRInteger(2)


# ---------------------------------------------------------------------------
# TestCompilerShapes — IR shape from parse+compile
# ---------------------------------------------------------------------------


class TestCompilerShapes:
    """Verify that parse+compile produces the expected IR structure."""

    def test_if_simple(self) -> None:
        ir = _compile_one("if x > 0 then x;")
        assert isinstance(ir, IRApply) and ir.head == IF
        assert len(ir.args) == 2

    def test_if_with_else(self) -> None:
        ir = _compile_one("if x > 0 then x else 0;")
        assert isinstance(ir, IRApply) and ir.head == IF
        assert len(ir.args) == 3

    def test_if_elseif_else(self) -> None:
        ir = _compile_one("if x > 0 then 1 elseif x < 0 then -1 else 0;")
        # Outer If
        assert isinstance(ir, IRApply) and ir.head == IF
        # Else branch is nested If
        assert isinstance(ir.args[2], IRApply) and ir.args[2].head == IF

    def test_for_each_shape(self) -> None:
        ir = _compile_one("for x in [1, 2, 3] do x;")
        assert isinstance(ir, IRApply) and ir.head == FOR_EACH
        assert len(ir.args) == 3
        assert ir.args[0] == IRSymbol("x")  # loop variable

    def test_for_range_full_shape(self) -> None:
        ir = _compile_one("for i: 1 step 2 thru 9 do i;")
        assert isinstance(ir, IRApply) and ir.head == FOR_RANGE
        assert len(ir.args) == 5
        var, start, step, end, body = ir.args
        assert var == IRSymbol("i")
        assert start == ONE
        assert step == IRInteger(2)
        assert end == IRInteger(9)

    def test_for_range_default_step(self) -> None:
        ir = _compile_one("for i: 1 thru 5 do i;")
        assert ir.head == FOR_RANGE
        var, start, step, end, body = ir.args
        assert step == ONE  # default step = 1

    def test_for_range_default_start_and_step(self) -> None:
        ir = _compile_one("for i thru 5 do i;")
        assert ir.head == FOR_RANGE
        var, start, step, end, body = ir.args
        assert start == ONE  # default start = 1
        assert step == ONE   # default step = 1

    def test_while_shape(self) -> None:
        ir = _compile_one("while x < 10 do x;")
        assert isinstance(ir, IRApply) and ir.head == WHILE
        assert len(ir.args) == 2

    def test_block_with_locals_shape(self) -> None:
        ir = _compile_one("block([x: 0, y], x + y);")
        assert isinstance(ir, IRApply) and ir.head == BLOCK
        # First arg is List(Assign(x,0), y)
        locals_list = ir.args[0]
        assert isinstance(locals_list, IRApply) and locals_list.head == LIST
        assert len(locals_list.args) == 2

    def test_block_without_locals_shape(self) -> None:
        ir = _compile_one("block(x + 1, x + 2);")
        assert ir.head == BLOCK
        # Compiler prepends empty List()
        assert ir.args[0] == IRApply(LIST, ())
        assert len(ir.args) == 3  # List(), x+1, x+2

    def test_return_shape(self) -> None:
        ir = _compile_one("return(42);")
        assert isinstance(ir, IRApply) and ir.head == RETURN
        assert ir.args[0] == IRInteger(42)

    def test_for_range_while_keyword(self) -> None:
        """for…while compiles to ForRange (while is the terminator keyword)."""
        ir = _compile_one("for i: 1 while i < 10 do i;")
        assert ir.head == FOR_RANGE
        assert len(ir.args) == 5

    def test_for_range_unless_keyword(self) -> None:
        """for…unless compiles to ForRange."""
        ir = _compile_one("for i: 1 unless i > 10 do i;")
        assert ir.head == FOR_RANGE
        assert len(ir.args) == 5


# ---------------------------------------------------------------------------
# TestEndToEnd — parse → compile → VM evaluation
# ---------------------------------------------------------------------------


class TestEndToEnd:
    """End-to-end evaluation through the string interface."""

    def test_if_true_branch(self) -> None:
        assert _eval("if 1 < 2 then 3 else 4;") == IRInteger(3)

    def test_if_false_branch(self) -> None:
        assert _eval("if 2 < 1 then 3 else 4;") == IRInteger(4)

    def test_if_elseif_chain(self) -> None:
        # x = -5 → second branch
        result = _eval("x : -5;  if x > 0 then 1 elseif x < 0 then -1 else 0;")
        assert result == IRInteger(-1)

    def test_for_range_basic(self) -> None:
        """for i: 1 thru 5 do i^2 returns 25 (the last iteration value)."""
        result = _eval("for i: 1 thru 5 do i^2;")
        assert result == IRInteger(25)

    def test_for_range_sum_in_block(self) -> None:
        """block([s: 0], for i: 1 thru 5 do s: s + i, s) → 15."""
        result = _eval("block([s: 0], for i: 1 thru 5 do s: s + i, s);")
        assert result == IRInteger(15)

    def test_for_each_basic(self) -> None:
        result = _eval("for x in [10, 20, 30] do x;")
        assert result == IRInteger(30)

    def test_for_each_in_block(self) -> None:
        result = _eval("block([s: 0], for x in [1, 2, 3, 4] do s: s + x, s);")
        assert result == IRInteger(10)

    def test_while_countdown(self) -> None:
        result = _eval("n : 5;  while n > 0 do n: n - 1;")
        assert result == ZERO

    def test_block_scope_does_not_leak(self) -> None:
        """Block local doesn't survive after block exits."""
        backend = _yb()
        vm = VM(backend)
        stmts = _compile("block([secret: 42], secret);")
        for s in stmts:
            vm.eval(s)
        # `secret` should not be bound in the outer environment.
        assert backend.lookup("secret") is None

    def test_block_returns_last_stmt(self) -> None:
        assert _eval("block(1 + 1, 2 + 2, 3 + 3);") == IRInteger(6)

    def test_nested_if_in_block(self) -> None:
        result = _eval("block([x: 7], if x > 5 then x * 2 else x);")
        assert result == IRInteger(14)

    def test_for_range_step(self) -> None:
        """Accumulate sum of even numbers 2, 4, 6, 8, 10."""
        result = _eval("block([s: 0], for i: 2 step 2 thru 10 do s: s + i, s);")
        assert result == IRInteger(30)

    def test_return_from_block(self) -> None:
        result = _eval("block([x: 0], x: 5, return(x * 2), x: 99);")
        assert result == IRInteger(10)

    def test_for_each_return_early(self) -> None:
        result = _eval("for x in [1, 2, 3, 4, 5] do if x = 3 then return(found);")
        assert result == IRSymbol("found")

    def test_for_range_no_start(self) -> None:
        """for i thru 3 do i → default start=1, returns 3."""
        result = _eval("for i thru 3 do i;")
        assert result == IRInteger(3)

    def test_block_multiple_locals(self) -> None:
        result = _eval("block([a: 3, b: 4], a^2 + b^2);")
        assert result == IRInteger(25)

    def test_nested_blocks_independent_scope(self) -> None:
        """Inner block's x doesn't permanently shadow outer block's x.

        ``block_expr`` sits at the top of the ``expression`` precedence
        hierarchy, so ``block(…) + x`` would need explicit parentheses to
        work as arithmetic.  This test instead uses a multi-statement outer
        block to verify scope restore without arithmetic on the block result.
        """
        # block([x: 10], block([x: 20], false), x) → 10
        # After the inner block exits, outer x is restored to 10.
        result = _eval("block([x: 10], block([x: 20], false), x);")
        assert result == IRInteger(10)


# ---------------------------------------------------------------------------
# TestRegressions — ensure prior behaviour is not broken by Phase G
# ---------------------------------------------------------------------------


class TestRegressions:
    """A selection of tests that exercises pre-Phase-G functionality."""

    def test_arithmetic(self) -> None:
        assert _eval("3 + 4 * 2;") == IRInteger(11)

    def test_assign_and_use(self) -> None:
        assert _eval("z : 7;  z * 3;") == IRInteger(21)

    def test_function_define_and_call(self) -> None:
        assert _eval("f(n) := n^2;  f(5);") == IRInteger(25)

    def test_list_literal(self) -> None:
        result = _eval("[1, 2, 3];")
        assert isinstance(result, IRApply) and result.head == LIST

    def test_logical_and(self) -> None:
        assert _eval("true and false;") == FALSE

    def test_comparison_equal(self) -> None:
        assert _eval("3 = 3;") == TRUE
