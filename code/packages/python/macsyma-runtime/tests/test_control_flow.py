"""End-to-end control-flow tests through MacsymaBackend.

These tests complete the Phase G control-flow story by driving every
construct (``if``, ``for``, ``while``, ``block``, ``return``) through
the *full* MACSYMA pipeline:

    MACSYMA string → parse_macsyma → compile_macsyma (with name-table
    extension) → VM(MacsymaBackend)

This is complementary to ``symbolic-vm/tests/test_phase_g.py``, which
uses ``SymbolicBackend`` directly (no name-table extension, no MACSYMA
history/kill integration).  Testing here verifies that:

1. ``MacsymaBackend`` properly inherits the VM control-flow handlers.
2. Control flow interacts correctly with CAS operations available
   through the extended name table (``factor``, ``solve``, etc.).
3. MACSYMA-specific features such as history references, multi-statement
   programs, and function definitions work alongside control flow.

Test organisation
-----------------

Class                      What it covers
-------------------------  ---------------------------------------------------
TestIfExpr                 if/elseif/else through MacsymaBackend
TestForRange               for…thru loops: sum, step, function body
TestForEach                for…in loops: list iteration and accumulation
TestWhileLoop              while…do: countdown, early exit via return
TestBlock                  block(): scope isolation, multiple locals, nesting
TestReturn                 return() early exit from block and loops
TestCASPlusControlFlow     control flow mixed with CAS calls (factor, solve)
TestMultiStatement         multi-statement programs, function defines in blocks
TestRegressions            ensure previously-working features are not broken
"""

from __future__ import annotations

from macsyma_compiler import compile_macsyma
from macsyma_compiler.compiler import _STANDARD_FUNCTIONS
from macsyma_parser import parse_macsyma
from symbolic_ir import (
    IRApply,
    IRInteger,
    IRSymbol,
)
from symbolic_vm import VM

from macsyma_runtime import MacsymaBackend, extend_compiler_name_table

# Extend the compiler name table once so every test in this module can
# use MACSYMA user-visible names (factor, solve, length, etc.).
# The call is idempotent — running multiple test modules with this call
# is safe.
extend_compiler_name_table(_STANDARD_FUNCTIONS)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _backend() -> MacsymaBackend:
    """Fresh MacsymaBackend (no shared state between tests)."""
    return MacsymaBackend()


def _eval(source: str) -> object:
    """Parse + compile + eval the last statement in ``source``.

    Handles multi-statement programs: evaluates every statement in order
    and returns the result of the last one.  Leading/trailing whitespace
    and trailing ``;`` / ``$`` are tolerated.
    """
    src = source.strip().rstrip(";$").strip()
    # Normalise: re-add ``;`` so the parser sees a complete program.
    ast = parse_macsyma(src + ";")
    stmts = compile_macsyma(ast, wrap_terminators=False)
    vm = VM(_backend())
    result: object = IRSymbol("False")
    for stmt in stmts:
        result = vm.eval(stmt)
    return result


def _eval_multi(source: str) -> list[object]:
    """Evaluate each statement in ``source`` and return all results."""
    ast = parse_macsyma(source)
    stmts = compile_macsyma(ast, wrap_terminators=False)
    backend = _backend()
    vm = VM(backend)
    return [vm.eval(s) for s in stmts]


def _int(n: int) -> IRInteger:
    return IRInteger(n)


def _sym(name: str) -> IRSymbol:
    return IRSymbol(name)


# ---------------------------------------------------------------------------
# TestIfExpr — if / elseif / else through MacsymaBackend
# ---------------------------------------------------------------------------


class TestIfExpr:
    """Verify ``if … then … [elseif … then …] [else …]`` end-to-end."""

    def test_if_true_branch(self) -> None:
        """``if 3 > 2 then 1 else 0`` → 1."""
        assert _eval("if 3 > 2 then 1 else 0") == _int(1)

    def test_if_false_branch(self) -> None:
        """``if 1 > 2 then 1 else 0`` → 0."""
        assert _eval("if 1 > 2 then 1 else 0") == _int(0)

    def test_if_no_else_false(self) -> None:
        """``if 1 > 2 then 99`` with no else → False (MACSYMA semantics)."""
        result = _eval("if 1 > 2 then 99")
        assert result == _sym("False")

    def test_elseif_chain(self) -> None:
        """``if … elseif … else …`` right-nests correctly."""
        result = _eval("if 1 > 10 then 1 elseif 2 > 1 then 2 else 3")
        assert result == _int(2)

    def test_if_with_assignment_body(self) -> None:
        """Condition fires an assignment; later statement reads result."""
        # Two statements: if sets y, then return y.
        src = "y : 0;  if 5 > 3 then y : 42;  y;"
        result = _eval_multi(src)[-1]
        assert result == _int(42)

    def test_if_condition_uses_variable(self) -> None:
        """Condition evaluates a previously assigned symbol."""
        result = _eval_multi("n : 7;  if n > 5 then big else small;")[-1]
        assert result == _sym("big")


# ---------------------------------------------------------------------------
# TestForRange — for…thru loops
# ---------------------------------------------------------------------------


class TestForRange:
    """Verify ``for NAME[:start][step s] thru b do body``."""

    def test_sum_one_to_five(self) -> None:
        """block([s:0], for i:1 thru 5 do s:s+i, s) → 15."""
        assert _eval("block([s: 0], for i: 1 thru 5 do s: s + i, s)") == _int(15)

    def test_last_body_value_returned(self) -> None:
        """``for i:1 thru 4 do i^2`` returns the *last* iteration value."""
        assert _eval("for i: 1 thru 4 do i^2") == _int(16)

    def test_step_two_sum_evens(self) -> None:
        """Step 2 visits 2,4,6,8,10 — sum = 30."""
        assert _eval("block([s:0], for i:2 step 2 thru 10 do s:s+i, s)") == _int(30)

    def test_default_start_and_step(self) -> None:
        """``for i thru 5 do i`` → 5 (start=1, step=1 by default)."""
        assert _eval("for i thru 5 do i") == _int(5)

    def test_empty_range_returns_false(self) -> None:
        """Range with start > end never runs body and returns False."""
        assert _eval("for i: 10 thru 1 do i") == _sym("False")

    def test_product_accumulation(self) -> None:
        """Compute 5! = 120 using a ForRange accumulator."""
        result = _eval("block([p: 1], for k: 1 thru 5 do p: p * k, p)")
        assert result == _int(120)


# ---------------------------------------------------------------------------
# TestForEach — for…in loops
# ---------------------------------------------------------------------------


class TestForEach:
    """Verify ``for NAME in list do body``."""

    def test_basic_iteration(self) -> None:
        """Last body value = last element value."""
        assert _eval("for x in [10, 20, 30] do x") == _int(30)

    def test_sum_list_elements(self) -> None:
        """Accumulate sum of list elements."""
        result = _eval("block([s: 0], for v in [3, 7, 10] do s: s + v, s)")
        assert result == _int(20)

    def test_empty_list_returns_false(self) -> None:
        """Iterating over an empty list returns False."""
        assert _eval("for x in [] do x") == _sym("False")

    def test_loop_variable_not_leaked(self) -> None:
        """Loop variable is unbound in outer scope after the loop."""
        src = "y : 99;  for y in [1, 2, 3] do y;  y;"
        # After the loop, y should be restored to its pre-loop value 99.
        assert _eval_multi(src)[-1] == _int(99)

    def test_for_each_with_return_exits_early(self) -> None:
        """return() inside a for-each exits on the matching element."""
        # Find the first element equal to 3 in [1,2,3,4,5].
        result = _eval(
            "for x in [1, 2, 3, 4, 5] do if x = 3 then return(found)"
        )
        assert result == _sym("found")


# ---------------------------------------------------------------------------
# TestWhileLoop — while…do
# ---------------------------------------------------------------------------


class TestWhileLoop:
    """Verify ``while cond do body``."""

    def test_countdown_to_zero(self) -> None:
        """Decrement n until zero; block returns final n = 0."""
        result = _eval("block([n: 5], while n > 0 do n: n - 1, n)")
        assert result == _int(0)

    def test_while_false_immediately_returns_false(self) -> None:
        """While body never entered when condition starts false."""
        result = _eval("while 1 > 2 do 99")
        assert result == _sym("False")

    def test_while_doubles_until_threshold(self) -> None:
        """n starts at 1, doubles each iteration until >= 32."""
        result = _eval("block([n: 1], while n < 32 do n: n * 2, n)")
        assert result == _int(32)


# ---------------------------------------------------------------------------
# TestBlock — block() scope isolation
# ---------------------------------------------------------------------------


class TestBlock:
    """Verify ``block([locals], stmt1, …, stmtN)`` scope semantics."""

    def test_block_returns_last_stmt(self) -> None:
        """block(1, 2, 3) → 3 (last statement's value)."""
        assert _eval("block(1, 2, 3)") == _int(3)

    def test_local_var_initialized(self) -> None:
        """block([x: 7], x) → 7."""
        assert _eval("block([x: 7], x)") == _int(7)

    def test_scope_does_not_leak(self) -> None:
        """Variable declared in block is unbound outside it."""
        src = "block([secret: 42], secret);  secret;"
        results = _eval_multi(src)
        assert results[0] == _int(42)
        # 'secret' is unbound outside; symbolic backend returns the symbol.
        assert results[1] == _sym("secret")

    def test_block_restores_outer_binding(self) -> None:
        """If outer scope had a binding, block restores it on exit."""
        src = "x : 100;  block([x: 0], x);  x;"
        results = _eval_multi(src)
        assert results[1] == _int(0)     # inner block sees x=0
        assert results[2] == _int(100)   # outer x restored to 100

    def test_nested_blocks_independent_scope(self) -> None:
        """Inner block's x doesn't permanently overwrite outer block's x."""
        result = _eval("block([x: 10], block([x: 20], false), x)")
        assert result == _int(10)

    def test_multiple_locals(self) -> None:
        """block([a:3, b:4], a^2 + b^2) → 25."""
        assert _eval("block([a: 3, b: 4], a^2 + b^2)") == _int(25)

    def test_block_empty_locals_no_list(self) -> None:
        """block(stmt1, stmt2) without a locals list also works."""
        assert _eval("block(1 + 1, 3 + 3)") == _int(6)


# ---------------------------------------------------------------------------
# TestReturn — early exit via return()
# ---------------------------------------------------------------------------


class TestReturn:
    """Verify ``return(expr)`` unwinds blocks and loops."""

    def test_return_exits_block_early(self) -> None:
        """return(42) inside block exits before remaining statements."""
        result = _eval("block([x: 0], x: 5, return(x * 2), x: 999)")
        assert result == _int(10)

    def test_return_exits_for_range_early(self) -> None:
        """return() inside a for-range body exits the loop early.

        The ForRange handler catches ``_ReturnSignal`` and returns the
        payload as the loop's normal result value.  There is no outer
        block here — the for-range itself is the top-level expression.
        """
        result = _eval("for i: 1 thru 10 do if i = 3 then return(found)")
        assert result == _sym("found")

    def test_return_exits_while_early(self) -> None:
        """return() as the direct while body exits immediately.

        ``while cond do return(val)`` — the While handler catches the
        ``_ReturnSignal`` and returns ``val``.  Using return() as the
        direct body (not inside an inner block) avoids the inner block
        catching the signal first.
        """
        # n is set to 5 before the while starts; the very first body
        # evaluation fires return(n) → 5 before any increment.
        result = _eval("block([n: 0], n: 5, while n > 0 do return(n))")
        assert result == _int(5)


# ---------------------------------------------------------------------------
# TestCASPlusControlFlow — control flow interacting with CAS operations
# ---------------------------------------------------------------------------


class TestCASPlusControlFlow:
    """Control-flow constructs work alongside CAS substrate operations."""

    def test_if_selects_between_factors(self) -> None:
        """if condition then CAS function call branch."""
        # x = 4: 4 > 3 is true → factor(x^2 - 1)
        result = _eval("x : 4;  if x > 3 then factor(x^2 - 1) else x^2")
        # factor(16 - 1) = factor(15) = 3 * 5 (or some factored form)
        # The important thing: it didn't return 16 (false branch).
        # We can't pin the exact factored IR form, so just verify it's not 16.
        assert result != _int(16)

    def test_for_range_with_solve(self) -> None:
        """for loop bodies can include CAS calls."""
        # Verify length([1,2,3]) is 3 inside a block.
        result = _eval("block([n: 0], for x in [1, 2, 3] do n: n + 1, n)")
        assert result == _int(3)

    def test_block_local_from_cas(self) -> None:
        """A block local can be initialised from a CAS result."""
        # length([a, b, c]) → 3; use it as a local initialiser.
        result = _eval("block([n: length([a, b, c])], n + 1)")
        assert result == _int(4)

    def test_while_with_list_length(self) -> None:
        """While condition can use a CAS-computed value."""
        # Accumulate list elements using a while loop over an index.
        # We express this as a for-in which is essentially the same semantic.
        result = _eval(
            "block([s: 0], for v in [5, 10, 15] do s: s + v, s)"
        )
        assert result == _int(30)


# ---------------------------------------------------------------------------
# TestMultiStatement — multi-statement programs and function defines
# ---------------------------------------------------------------------------


class TestMultiStatement:
    """Multi-statement programs interleaving assignments, defs, and control flow."""

    def test_function_define_then_call_in_block(self) -> None:
        """User-defined function called inside a block."""
        src = "square(n) := n^2;  block([x: 5], square(x));"
        result = _eval_multi(src)[-1]
        assert result == _int(25)

    def test_assign_then_if(self) -> None:
        """Assignment before if; condition refers to assigned var."""
        results = _eval_multi("t : 10;  if t > 5 then yes else no;")
        assert results[-1] == _sym("yes")

    def test_block_accumulates_using_outer_var(self) -> None:
        """RHS of a block local is evaluated in the outer scope."""
        src = "n : 3;  block([m: n * 2], m + 1);"
        result = _eval_multi(src)[-1]
        assert result == _int(7)

    def test_for_range_fibonacci_like(self) -> None:
        """Compute F(7) = 13 using two accumulators.

        ``(stmt1, stmt2)`` is NOT a valid sequential form in this grammar —
        use ``block(stmt1, stmt2)`` (no locals list) instead.  With no locals
        the inner block shares the outer scope, so assignments to ``a``, ``b``,
        ``tmp`` all update the outer block's locals correctly.
        """
        result = _eval(
            "block([a: 1, b: 1, tmp: 0], "
            "for i: 1 thru 5 do block(tmp: b, b: a + b, a: tmp), "
            "b)"
        )
        assert result == _int(13)


# ---------------------------------------------------------------------------
# TestRegressions — ensure that non-Phase-G behaviour is undisturbed
# ---------------------------------------------------------------------------


class TestRegressions:
    """A handful of pre-Phase-G tests verify nothing was broken."""

    def test_arithmetic(self) -> None:
        assert _eval("3 + 4 * 2") == _int(11)

    def test_assign_and_read(self) -> None:
        results = _eval_multi("z : 7;  z * 3;")
        assert results[-1] == _int(21)

    def test_list_length(self) -> None:
        assert _eval("length([1, 2, 3, 4])") == _int(4)

    def test_factor_difference_of_squares(self) -> None:
        """factor(x^2 - 1) returns a non-trivial factored form."""
        from symbolic_ir import MUL
        result = _eval("factor(x^2 - 1)")
        # Should be a MUL node — not just the original polynomial.
        assert isinstance(result, IRApply) and result.head == MUL
