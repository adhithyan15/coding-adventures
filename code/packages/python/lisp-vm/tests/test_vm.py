"""Tests for the Lisp VM — opcodes, handlers, GC integration, and TCO.

These tests construct bytecode directly (without the compiler) to verify
that the VM correctly executes each opcode.
"""

from __future__ import annotations

from garbage_collector import ConsCell, MarkAndSweepGC, Symbol
from virtual_machine import CodeObject, Instruction

from lisp_vm import LispFunction, LispOp, NIL, create_lisp_vm


def _make_code(
    instructions: list[Instruction],
    constants: list[object] | None = None,
    names: list[str] | None = None,
) -> CodeObject:
    """Helper to create CodeObject instances."""
    return CodeObject(
        instructions=instructions,
        constants=constants or [],
        names=names or [],
    )


def _run(code: CodeObject, gc: MarkAndSweepGC | None = None) -> object:
    """Execute code and return the top of stack."""
    vm = create_lisp_vm(gc=gc)
    vm.execute(code)
    return vm.stack[-1] if vm.stack else None


# -------------------------------------------------------------------------
# Stack operations
# -------------------------------------------------------------------------


class TestStackOps:
    """Tests for LOAD_CONST, POP, LOAD_NIL, LOAD_TRUE."""

    def test_load_const(self) -> None:
        """LOAD_CONST should push a constant from the pool."""
        code = _make_code(
            [Instruction(LispOp.LOAD_CONST, 0), Instruction(LispOp.HALT)],
            constants=[42],
        )
        assert _run(code) == 42

    def test_load_nil(self) -> None:
        """LOAD_NIL should push the NIL sentinel."""
        code = _make_code(
            [Instruction(LispOp.LOAD_NIL), Instruction(LispOp.HALT)],
        )
        assert _run(code) is NIL

    def test_load_true(self) -> None:
        """LOAD_TRUE should push True."""
        code = _make_code(
            [Instruction(LispOp.LOAD_TRUE), Instruction(LispOp.HALT)],
        )
        assert _run(code) is True

    def test_pop(self) -> None:
        """POP should discard the top of stack."""
        code = _make_code(
            [
                Instruction(LispOp.LOAD_CONST, 0),
                Instruction(LispOp.LOAD_CONST, 1),
                Instruction(LispOp.POP),
                Instruction(LispOp.HALT),
            ],
            constants=[1, 2],
        )
        assert _run(code) == 1


# -------------------------------------------------------------------------
# Variable operations
# -------------------------------------------------------------------------


class TestVariableOps:
    """Tests for STORE_NAME, LOAD_NAME, STORE_LOCAL, LOAD_LOCAL."""

    def test_store_and_load_name(self) -> None:
        """STORE_NAME/LOAD_NAME should store and retrieve global variables."""
        code = _make_code(
            [
                Instruction(LispOp.LOAD_CONST, 0),
                Instruction(LispOp.STORE_NAME, 0),
                Instruction(LispOp.LOAD_NAME, 0),
                Instruction(LispOp.HALT),
            ],
            constants=[42],
            names=["x"],
        )
        assert _run(code) == 42

    def test_store_and_load_local(self) -> None:
        """STORE_LOCAL/LOAD_LOCAL should store and retrieve local slots."""
        code = _make_code(
            [
                Instruction(LispOp.LOAD_CONST, 0),
                Instruction(LispOp.STORE_LOCAL, 0),
                Instruction(LispOp.LOAD_LOCAL, 0),
                Instruction(LispOp.HALT),
            ],
            constants=[99],
        )
        assert _run(code) == 99


# -------------------------------------------------------------------------
# Arithmetic
# -------------------------------------------------------------------------


class TestArithmetic:
    """Tests for ADD, SUB, MUL, DIV."""

    def test_add(self) -> None:
        """ADD should add two numbers."""
        code = _make_code(
            [
                Instruction(LispOp.LOAD_CONST, 0),
                Instruction(LispOp.LOAD_CONST, 1),
                Instruction(LispOp.ADD),
                Instruction(LispOp.HALT),
            ],
            constants=[3, 4],
        )
        assert _run(code) == 7

    def test_sub(self) -> None:
        """SUB should subtract."""
        code = _make_code(
            [
                Instruction(LispOp.LOAD_CONST, 0),
                Instruction(LispOp.LOAD_CONST, 1),
                Instruction(LispOp.SUB),
                Instruction(LispOp.HALT),
            ],
            constants=[10, 3],
        )
        assert _run(code) == 7

    def test_mul(self) -> None:
        """MUL should multiply."""
        code = _make_code(
            [
                Instruction(LispOp.LOAD_CONST, 0),
                Instruction(LispOp.LOAD_CONST, 1),
                Instruction(LispOp.MUL),
                Instruction(LispOp.HALT),
            ],
            constants=[6, 7],
        )
        assert _run(code) == 42

    def test_div(self) -> None:
        """DIV should integer-divide."""
        code = _make_code(
            [
                Instruction(LispOp.LOAD_CONST, 0),
                Instruction(LispOp.LOAD_CONST, 1),
                Instruction(LispOp.DIV),
                Instruction(LispOp.HALT),
            ],
            constants=[10, 3],
        )
        assert _run(code) == 3


# -------------------------------------------------------------------------
# Comparison
# -------------------------------------------------------------------------


class TestComparison:
    """Tests for CMP_EQ, CMP_LT, CMP_GT."""

    def test_eq_true(self) -> None:
        """CMP_EQ should return 1 for equal values."""
        code = _make_code(
            [
                Instruction(LispOp.LOAD_CONST, 0),
                Instruction(LispOp.LOAD_CONST, 0),
                Instruction(LispOp.CMP_EQ),
                Instruction(LispOp.HALT),
            ],
            constants=[42],
        )
        assert _run(code) == 1

    def test_eq_false(self) -> None:
        """CMP_EQ should return 0 for unequal values."""
        code = _make_code(
            [
                Instruction(LispOp.LOAD_CONST, 0),
                Instruction(LispOp.LOAD_CONST, 1),
                Instruction(LispOp.CMP_EQ),
                Instruction(LispOp.HALT),
            ],
            constants=[1, 2],
        )
        assert _run(code) == 0

    def test_eq_nil(self) -> None:
        """CMP_EQ should handle NIL identity."""
        code = _make_code(
            [
                Instruction(LispOp.LOAD_NIL),
                Instruction(LispOp.LOAD_NIL),
                Instruction(LispOp.CMP_EQ),
                Instruction(LispOp.HALT),
            ],
        )
        assert _run(code) == 1

    def test_lt(self) -> None:
        """CMP_LT should compare correctly."""
        code = _make_code(
            [
                Instruction(LispOp.LOAD_CONST, 0),
                Instruction(LispOp.LOAD_CONST, 1),
                Instruction(LispOp.CMP_LT),
                Instruction(LispOp.HALT),
            ],
            constants=[1, 2],
        )
        assert _run(code) == 1

    def test_gt(self) -> None:
        """CMP_GT should compare correctly."""
        code = _make_code(
            [
                Instruction(LispOp.LOAD_CONST, 0),
                Instruction(LispOp.LOAD_CONST, 1),
                Instruction(LispOp.CMP_GT),
                Instruction(LispOp.HALT),
            ],
            constants=[5, 3],
        )
        assert _run(code) == 1


# -------------------------------------------------------------------------
# Control flow
# -------------------------------------------------------------------------


class TestControlFlow:
    """Tests for JUMP, JUMP_IF_FALSE, JUMP_IF_TRUE."""

    def test_jump(self) -> None:
        """JUMP should skip instructions."""
        code = _make_code(
            [
                Instruction(LispOp.JUMP, 2),        # skip next
                Instruction(LispOp.LOAD_CONST, 0),   # skipped
                Instruction(LispOp.LOAD_CONST, 1),   # landed here
                Instruction(LispOp.HALT),
            ],
            constants=[99, 42],
        )
        assert _run(code) == 42

    def test_jump_if_false_taken(self) -> None:
        """JUMP_IF_FALSE should jump when value is falsy (NIL)."""
        code = _make_code(
            [
                Instruction(LispOp.LOAD_NIL),
                Instruction(LispOp.JUMP_IF_FALSE, 3),
                Instruction(LispOp.LOAD_CONST, 0),   # skipped
                Instruction(LispOp.LOAD_CONST, 1),   # landed here
                Instruction(LispOp.HALT),
            ],
            constants=[99, 42],
        )
        assert _run(code) == 42

    def test_jump_if_false_not_taken(self) -> None:
        """JUMP_IF_FALSE should not jump when value is truthy."""
        code = _make_code(
            [
                Instruction(LispOp.LOAD_CONST, 0),
                Instruction(LispOp.JUMP_IF_FALSE, 3),
                Instruction(LispOp.LOAD_CONST, 1),   # not skipped
                Instruction(LispOp.HALT),
                Instruction(LispOp.LOAD_CONST, 0),   # would be here if jumped
                Instruction(LispOp.HALT),
            ],
            constants=[1, 42],
        )
        assert _run(code) == 42

    def test_zero_is_falsy(self) -> None:
        """0 should be falsy for JUMP_IF_FALSE."""
        code = _make_code(
            [
                Instruction(LispOp.LOAD_CONST, 0),   # push 0
                Instruction(LispOp.JUMP_IF_FALSE, 3),
                Instruction(LispOp.LOAD_CONST, 1),   # skipped
                Instruction(LispOp.LOAD_CONST, 2),   # landed here
                Instruction(LispOp.HALT),
            ],
            constants=[0, 99, 42],
        )
        assert _run(code) == 42


# -------------------------------------------------------------------------
# Cons cells
# -------------------------------------------------------------------------


class TestConsCells:
    """Tests for CONS, CAR, CDR."""

    def test_cons(self) -> None:
        """CONS should create a cons cell on the heap."""
        gc = MarkAndSweepGC()
        code = _make_code(
            [
                Instruction(LispOp.LOAD_CONST, 0),   # cdr = 2
                Instruction(LispOp.LOAD_CONST, 1),   # car = 1
                Instruction(LispOp.CONS),
                Instruction(LispOp.HALT),
            ],
            constants=[2, 1],
        )
        addr = _run(code, gc=gc)
        assert isinstance(addr, int)
        cell = gc.deref(addr)
        assert isinstance(cell, ConsCell)
        assert cell.car == 1
        assert cell.cdr == 2

    def test_car(self) -> None:
        """CAR should extract the first element."""
        code = _make_code(
            [
                Instruction(LispOp.LOAD_CONST, 0),   # cdr
                Instruction(LispOp.LOAD_CONST, 1),   # car
                Instruction(LispOp.CONS),
                Instruction(LispOp.CAR),
                Instruction(LispOp.HALT),
            ],
            constants=[2, 1],
        )
        assert _run(code) == 1

    def test_cdr(self) -> None:
        """CDR should extract the second element."""
        code = _make_code(
            [
                Instruction(LispOp.LOAD_CONST, 0),   # cdr
                Instruction(LispOp.LOAD_CONST, 1),   # car
                Instruction(LispOp.CONS),
                Instruction(LispOp.CDR),
                Instruction(LispOp.HALT),
            ],
            constants=[2, 1],
        )
        assert _run(code) == 2

    def test_nested_cons(self) -> None:
        """Nested cons cells should work (building a list)."""
        # Build (1 . (2 . NIL)) = (1 2)
        code = _make_code(
            [
                Instruction(LispOp.LOAD_NIL),         # NIL (end of list)
                Instruction(LispOp.LOAD_CONST, 0),    # 2
                Instruction(LispOp.CONS),             # (2 . NIL)
                Instruction(LispOp.LOAD_CONST, 1),    # 1
                Instruction(LispOp.CONS),             # (1 . (2 . NIL))
                Instruction(LispOp.CAR),              # get 1
                Instruction(LispOp.HALT),
            ],
            constants=[2, 1],
        )
        assert _run(code) == 1


# -------------------------------------------------------------------------
# Symbols
# -------------------------------------------------------------------------


class TestSymbols:
    """Tests for MAKE_SYMBOL and symbol interning."""

    def test_make_symbol(self) -> None:
        """MAKE_SYMBOL should intern a symbol and push its address."""
        gc = MarkAndSweepGC()
        code = _make_code(
            [
                Instruction(LispOp.MAKE_SYMBOL, 0),
                Instruction(LispOp.HALT),
            ],
            constants=["foo"],
        )
        addr = _run(code, gc=gc)
        assert isinstance(addr, int)
        obj = gc.deref(addr)
        assert isinstance(obj, Symbol)
        assert obj.name == "foo"

    def test_symbol_interning(self) -> None:
        """Two MAKE_SYMBOL for the same name should return the same address."""
        gc = MarkAndSweepGC()
        code = _make_code(
            [
                Instruction(LispOp.MAKE_SYMBOL, 0),  # intern "foo"
                Instruction(LispOp.MAKE_SYMBOL, 0),  # intern "foo" again
                Instruction(LispOp.CMP_EQ),           # should be equal
                Instruction(LispOp.HALT),
            ],
            constants=["foo"],
        )
        assert _run(code, gc=gc) == 1


# -------------------------------------------------------------------------
# Predicates
# -------------------------------------------------------------------------


class TestPredicates:
    """Tests for IS_ATOM and IS_NIL."""

    def test_is_atom_number(self) -> None:
        """Numbers should be atoms."""
        code = _make_code(
            [
                Instruction(LispOp.LOAD_CONST, 0),
                Instruction(LispOp.IS_ATOM),
                Instruction(LispOp.HALT),
            ],
            constants=[42],
        )
        assert _run(code) == 1

    def test_is_atom_nil(self) -> None:
        """NIL should be an atom."""
        code = _make_code(
            [
                Instruction(LispOp.LOAD_NIL),
                Instruction(LispOp.IS_ATOM),
                Instruction(LispOp.HALT),
            ],
        )
        assert _run(code) == 1

    def test_is_atom_cons(self) -> None:
        """A cons cell should NOT be an atom."""
        code = _make_code(
            [
                Instruction(LispOp.LOAD_CONST, 0),
                Instruction(LispOp.LOAD_CONST, 1),
                Instruction(LispOp.CONS),
                Instruction(LispOp.IS_ATOM),
                Instruction(LispOp.HALT),
            ],
            constants=[2, 1],
        )
        assert _run(code) == 0

    def test_is_nil_true(self) -> None:
        """IS_NIL should return 1 for NIL."""
        code = _make_code(
            [
                Instruction(LispOp.LOAD_NIL),
                Instruction(LispOp.IS_NIL),
                Instruction(LispOp.HALT),
            ],
        )
        assert _run(code) == 1

    def test_is_nil_false(self) -> None:
        """IS_NIL should return 0 for non-NIL."""
        code = _make_code(
            [
                Instruction(LispOp.LOAD_CONST, 0),
                Instruction(LispOp.IS_NIL),
                Instruction(LispOp.HALT),
            ],
            constants=[42],
        )
        assert _run(code) == 0


# -------------------------------------------------------------------------
# Functions and closures
# -------------------------------------------------------------------------


class TestFunctions:
    """Tests for MAKE_CLOSURE, CALL_FUNCTION, RETURN."""

    def test_simple_function(self) -> None:
        """A simple function that returns a constant should work."""
        # Function body: push 42, return
        func_code = _make_code(
            [
                Instruction(LispOp.LOAD_CONST, 0),
                Instruction(LispOp.RETURN),
            ],
            constants=[42],
        )

        # Main: create closure, call it
        main_code = _make_code(
            [
                Instruction(LispOp.LOAD_CONST, 0),   # push func CodeObject
                Instruction(LispOp.MAKE_CLOSURE, 0),  # 0 params
                Instruction(LispOp.CALL_FUNCTION, 0), # call with 0 args
                Instruction(LispOp.HALT),
            ],
            constants=[func_code],
        )
        assert _run(main_code) == 42

    def test_function_with_args(self) -> None:
        """A function that uses its arguments should work."""
        # Function body: load local 0, load local 1, add, return
        func_code = _make_code(
            [
                Instruction(LispOp.LOAD_LOCAL, 0),
                Instruction(LispOp.LOAD_LOCAL, 1),
                Instruction(LispOp.ADD),
                Instruction(LispOp.RETURN),
            ],
        )

        # Main: push args, create closure, call
        main_code = _make_code(
            [
                Instruction(LispOp.LOAD_CONST, 0),   # arg 1 = 3
                Instruction(LispOp.LOAD_CONST, 1),   # arg 2 = 4
                Instruction(LispOp.LOAD_CONST, 2),   # func CodeObject
                Instruction(LispOp.MAKE_CLOSURE, 2),  # 2 params
                Instruction(LispOp.CALL_FUNCTION, 2), # call with 2 args
                Instruction(LispOp.HALT),
            ],
            constants=[3, 4, func_code],
        )
        assert _run(main_code) == 7

    def test_recursive_function(self) -> None:
        """A recursive function (factorial) should work.

        Hand-compiled factorial:
        (define factorial (lambda (n) (cond ((eq n 0) 1) (t (* n (factorial (- n 1)))))))
        (factorial 5)
        """
        # Function body for factorial(n):
        # LOAD_LOCAL 0 (n)
        # LOAD_CONST 0 (0)
        # CMP_EQ
        # JUMP_IF_FALSE 6
        # LOAD_CONST 1 (1)     <- return 1
        # RETURN
        # LOAD_LOCAL 0 (n)     <- else branch
        # LOAD_LOCAL 0 (n)
        # LOAD_CONST 1 (1)
        # SUB                   <- n - 1
        # LOAD_NAME 0 (factorial)
        # CALL_FUNCTION 1       <- factorial(n-1)
        # MUL                   <- n * factorial(n-1)
        # RETURN
        func_code = _make_code(
            [
                Instruction(LispOp.LOAD_LOCAL, 0),     # 0: n
                Instruction(LispOp.LOAD_CONST, 0),     # 1: 0
                Instruction(LispOp.CMP_EQ),            # 2: n == 0?
                Instruction(LispOp.JUMP_IF_FALSE, 6),  # 3: if not, goto 6
                Instruction(LispOp.LOAD_CONST, 1),     # 4: push 1
                Instruction(LispOp.RETURN),             # 5: return 1
                Instruction(LispOp.LOAD_LOCAL, 0),     # 6: n
                Instruction(LispOp.LOAD_LOCAL, 0),     # 7: n
                Instruction(LispOp.LOAD_CONST, 1),     # 8: 1
                Instruction(LispOp.SUB),                # 9: n - 1
                Instruction(LispOp.LOAD_NAME, 0),      # 10: factorial
                Instruction(LispOp.CALL_FUNCTION, 1),  # 11: factorial(n-1)
                Instruction(LispOp.MUL),                # 12: n * result
                Instruction(LispOp.RETURN),             # 13: return
            ],
            constants=[0, 1],
            names=["factorial"],
        )

        # Main: define factorial, call factorial(5)
        main_code = _make_code(
            [
                Instruction(LispOp.LOAD_CONST, 0),    # func code
                Instruction(LispOp.MAKE_CLOSURE, 1),   # 1 param
                Instruction(LispOp.STORE_NAME, 0),     # define "factorial"
                Instruction(LispOp.LOAD_CONST, 1),     # push 5
                Instruction(LispOp.LOAD_NAME, 0),      # push factorial
                Instruction(LispOp.CALL_FUNCTION, 1),  # call factorial(5)
                Instruction(LispOp.HALT),
            ],
            constants=[func_code, 5],
            names=["factorial"],
        )
        assert _run(main_code) == 120


# -------------------------------------------------------------------------
# Tail Call Optimization
# -------------------------------------------------------------------------


class TestTailCall:
    """Tests for TAIL_CALL opcode."""

    def test_tail_recursive_factorial(self) -> None:
        """Tail-recursive factorial should work with TAIL_CALL.

        (define factorial-iter (lambda (n acc)
          (cond ((eq n 0) acc)
                (t (factorial-iter (- n 1) (* n acc))))))
        """
        func_code = _make_code(
            [
                Instruction(LispOp.LOAD_LOCAL, 0),     # 0: n
                Instruction(LispOp.LOAD_CONST, 0),     # 1: 0
                Instruction(LispOp.CMP_EQ),            # 2: n == 0?
                Instruction(LispOp.JUMP_IF_FALSE, 6),  # 3: if not, goto 6
                Instruction(LispOp.LOAD_LOCAL, 1),     # 4: acc
                Instruction(LispOp.RETURN),             # 5: return acc
                # Compute args for tail call
                Instruction(LispOp.LOAD_LOCAL, 0),     # 6: n
                Instruction(LispOp.LOAD_CONST, 1),     # 7: 1
                Instruction(LispOp.SUB),                # 8: n - 1
                Instruction(LispOp.LOAD_LOCAL, 0),     # 9: n
                Instruction(LispOp.LOAD_LOCAL, 1),     # 10: acc
                Instruction(LispOp.MUL),                # 11: n * acc
                Instruction(LispOp.LOAD_NAME, 0),      # 12: factorial-iter
                Instruction(LispOp.TAIL_CALL, 2),      # 13: tail call with 2 args
            ],
            constants=[0, 1],
            names=["factorial-iter"],
        )

        main_code = _make_code(
            [
                Instruction(LispOp.LOAD_CONST, 0),    # func code
                Instruction(LispOp.MAKE_CLOSURE, 2),   # 2 params
                Instruction(LispOp.STORE_NAME, 0),     # define "factorial-iter"
                Instruction(LispOp.LOAD_CONST, 1),     # push 10
                Instruction(LispOp.LOAD_CONST, 2),     # push 1 (initial acc)
                Instruction(LispOp.LOAD_NAME, 0),      # push factorial-iter
                Instruction(LispOp.CALL_FUNCTION, 2),  # call
                Instruction(LispOp.HALT),
            ],
            constants=[func_code, 10, 1],
            names=["factorial-iter"],
        )
        assert _run(main_code) == 3628800  # 10!

    def test_tail_call_no_stack_overflow(self) -> None:
        """TAIL_CALL should handle deep recursion without stack overflow.

        A simple countdown: (define count (lambda (n) (cond ((eq n 0) 0) (t (count (- n 1))))))
        With regular CALL_FUNCTION this would overflow for large N.
        With TAIL_CALL it uses O(1) stack.
        """
        func_code = _make_code(
            [
                Instruction(LispOp.LOAD_LOCAL, 0),     # n
                Instruction(LispOp.LOAD_CONST, 0),     # 0
                Instruction(LispOp.CMP_EQ),            # n == 0?
                Instruction(LispOp.JUMP_IF_FALSE, 6),
                Instruction(LispOp.LOAD_CONST, 0),     # return 0
                Instruction(LispOp.RETURN),
                Instruction(LispOp.LOAD_LOCAL, 0),     # n
                Instruction(LispOp.LOAD_CONST, 1),     # 1
                Instruction(LispOp.SUB),                # n - 1
                Instruction(LispOp.LOAD_NAME, 0),      # count
                Instruction(LispOp.TAIL_CALL, 1),      # tail call
            ],
            constants=[0, 1],
            names=["count"],
        )

        main_code = _make_code(
            [
                Instruction(LispOp.LOAD_CONST, 0),
                Instruction(LispOp.MAKE_CLOSURE, 1),
                Instruction(LispOp.STORE_NAME, 0),
                Instruction(LispOp.LOAD_CONST, 1),     # count(10000)
                Instruction(LispOp.LOAD_NAME, 0),
                Instruction(LispOp.CALL_FUNCTION, 1),
                Instruction(LispOp.HALT),
            ],
            constants=[func_code, 10000],
            names=["count"],
        )
        # Should complete without RecursionError
        assert _run(main_code) == 0


# -------------------------------------------------------------------------
# PRINT
# -------------------------------------------------------------------------


class TestPrint:
    """Tests for PRINT output."""

    def _get_output(self, traces: list[object]) -> str:
        """Extract printed output from VM traces."""
        parts = []
        for trace in traces:
            if hasattr(trace, "output") and trace.output is not None:
                parts.append(trace.output)
        return "\n".join(parts)

    def test_print_number(self) -> None:
        """PRINT should format numbers."""
        vm = create_lisp_vm()
        code = _make_code(
            [
                Instruction(LispOp.LOAD_CONST, 0),
                Instruction(LispOp.PRINT),
                Instruction(LispOp.HALT),
            ],
            constants=[42],
        )
        traces = vm.execute(code)
        assert "42" in self._get_output(traces)

    def test_print_nil(self) -> None:
        """PRINT should format NIL as 'nil'."""
        vm = create_lisp_vm()
        code = _make_code(
            [
                Instruction(LispOp.LOAD_NIL),
                Instruction(LispOp.PRINT),
                Instruction(LispOp.HALT),
            ],
        )
        traces = vm.execute(code)
        assert "nil" in self._get_output(traces)

    def test_print_cons(self) -> None:
        """PRINT should format cons cells as lists."""
        vm = create_lisp_vm()
        # Build (1 . (2 . NIL)) = (1 2)
        code = _make_code(
            [
                Instruction(LispOp.LOAD_NIL),
                Instruction(LispOp.LOAD_CONST, 0),   # 2
                Instruction(LispOp.CONS),
                Instruction(LispOp.LOAD_CONST, 1),   # 1
                Instruction(LispOp.CONS),
                Instruction(LispOp.PRINT),
                Instruction(LispOp.HALT),
            ],
            constants=[2, 1],
        )
        traces = vm.execute(code)
        assert "(1 2)" in self._get_output(traces)


# -------------------------------------------------------------------------
# NIL sentinel
# -------------------------------------------------------------------------


class TestNIL:
    """Tests for the NIL sentinel behavior."""

    def test_nil_is_falsy(self) -> None:
        """NIL should be falsy."""
        assert not NIL

    def test_nil_repr(self) -> None:
        """NIL should repr as 'NIL'."""
        assert repr(NIL) == "NIL"

    def test_nil_is_singleton(self) -> None:
        """NIL should be a singleton (identity check works)."""
        from lisp_vm.handlers import NIL as NIL2
        assert NIL is NIL2
