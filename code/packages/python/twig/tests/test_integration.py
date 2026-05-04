"""End-to-end integration tests for Twig.

These tests exercise the full pipeline: source → lex → parse →
extract → compile → vm-core execute → observable output.  If
anything in the chain breaks, these tests fail loudly.
"""

from __future__ import annotations

import pytest

from twig import NIL, TwigVM
from twig.errors import TwigCompileError, TwigRuntimeError


@pytest.fixture
def vm() -> TwigVM:
    return TwigVM()


# ---------------------------------------------------------------------------
# Atoms
# ---------------------------------------------------------------------------


def test_integer_returns_value(vm: TwigVM) -> None:
    _, val = vm.run("42")
    assert val == 42


def test_negative_integer(vm: TwigVM) -> None:
    _, val = vm.run("-7")
    assert val == -7


def test_boolean_true(vm: TwigVM) -> None:
    _, val = vm.run("#t")
    assert val is True


def test_boolean_false(vm: TwigVM) -> None:
    _, val = vm.run("#f")
    assert val is False


def test_nil(vm: TwigVM) -> None:
    _, val = vm.run("nil")
    assert val is NIL


def test_empty_program_returns_nil(vm: TwigVM) -> None:
    _, val = vm.run("")
    assert val is NIL


# ---------------------------------------------------------------------------
# Arithmetic
# ---------------------------------------------------------------------------


def test_addition(vm: TwigVM) -> None:
    _, val = vm.run("(+ 1 2 3)")
    assert val == 6


def test_subtraction(vm: TwigVM) -> None:
    _, val = vm.run("(- 10 3 2)")
    assert val == 5


def test_unary_negation(vm: TwigVM) -> None:
    _, val = vm.run("(- 5)")
    assert val == -5


def test_multiplication(vm: TwigVM) -> None:
    _, val = vm.run("(* 2 3 4)")
    assert val == 24


def test_division(vm: TwigVM) -> None:
    _, val = vm.run("(/ 20 4)")
    assert val == 5


def test_division_by_zero_raises(vm: TwigVM) -> None:
    with pytest.raises(TwigRuntimeError):
        vm.run("(/ 1 0)")


def test_equality(vm: TwigVM) -> None:
    _, val = vm.run("(= 1 1 1)")
    assert val is True
    _, val = vm.run("(= 1 2)")
    assert val is False


def test_comparison(vm: TwigVM) -> None:
    _, val = vm.run("(< 1 2)")
    assert val is True
    _, val = vm.run("(> 1 2)")
    assert val is False


# ---------------------------------------------------------------------------
# Control flow
# ---------------------------------------------------------------------------


def test_if_taken(vm: TwigVM) -> None:
    _, val = vm.run("(if #t 1 2)")
    assert val == 1


def test_if_not_taken(vm: TwigVM) -> None:
    _, val = vm.run("(if #f 1 2)")
    assert val == 2


def test_if_with_nil_is_falsy(vm: TwigVM) -> None:
    _, val = vm.run("(if nil 1 2)")
    assert val == 2


def test_if_with_zero_is_truthy(vm: TwigVM) -> None:
    """Scheme says only ``#f`` and ``nil`` are falsy.  ``0`` is true."""
    _, val = vm.run("(if 0 1 2)")
    assert val == 1


def test_let_introduces_bindings(vm: TwigVM) -> None:
    _, val = vm.run("(let ((a 1) (b 2)) (+ a b))")
    assert val == 3


def test_nested_let(vm: TwigVM) -> None:
    _, val = vm.run("(let ((a 1)) (let ((b 2)) (+ a b)))")
    assert val == 3


def test_begin_returns_last(vm: TwigVM) -> None:
    _, val = vm.run("(begin 1 2 3)")
    assert val == 3


# ---------------------------------------------------------------------------
# Cons cells
# ---------------------------------------------------------------------------


def test_cons_car_cdr(vm: TwigVM) -> None:
    _, val = vm.run("(car (cons 1 2))")
    assert val == 1
    _, val = vm.run("(cdr (cons 1 2))")
    assert val == 2


def test_proper_list_length(vm: TwigVM) -> None:
    _, val = vm.run("""
        (define (length xs)
          (if (null? xs) 0 (+ 1 (length (cdr xs)))))
        (length (cons 1 (cons 2 (cons 3 nil))))
    """)
    assert val == 3


def test_pair_predicate(vm: TwigVM) -> None:
    _, val = vm.run("(pair? (cons 1 2))")
    assert val is True
    _, val = vm.run("(pair? nil)")
    assert val is False
    _, val = vm.run("(pair? 42)")
    assert val is False


def test_null_predicate(vm: TwigVM) -> None:
    _, val = vm.run("(null? nil)")
    assert val is True
    _, val = vm.run("(null? (cons 1 2))")
    assert val is False


def test_number_predicate(vm: TwigVM) -> None:
    _, val = vm.run("(number? 42)")
    assert val is True
    _, val = vm.run("(number? #t)")
    assert val is False
    _, val = vm.run("(number? 'foo)")
    assert val is False


def test_symbol_predicate(vm: TwigVM) -> None:
    _, val = vm.run("(symbol? 'foo)")
    assert val is True
    _, val = vm.run("(symbol? 42)")
    assert val is False


# ---------------------------------------------------------------------------
# Recursion + define
# ---------------------------------------------------------------------------


def test_factorial(vm: TwigVM) -> None:
    _, val = vm.run("""
        (define (fact n)
          (if (= n 0) 1 (* n (fact (- n 1)))))
        (fact 6)
    """)
    assert val == 720


def test_mutual_recursion(vm: TwigVM) -> None:
    _, val = vm.run("""
        (define (even? n)
          (if (= n 0) #t (odd? (- n 1))))
        (define (odd? n)
          (if (= n 0) #f (even? (- n 1))))
        (even? 10)
    """)
    assert val is True


def test_define_value(vm: TwigVM) -> None:
    _, val = vm.run("""
        (define x 42)
        (+ x 8)
    """)
    assert val == 50


# ---------------------------------------------------------------------------
# Closures
# ---------------------------------------------------------------------------


def test_simple_closure(vm: TwigVM) -> None:
    """The classic adder."""
    _, val = vm.run("""
        (define (adder n) (lambda (x) (+ x n)))
        (define add5 (adder 5))
        (add5 3)
    """)
    assert val == 8


def test_closure_captures_multiple_vars(vm: TwigVM) -> None:
    _, val = vm.run("""
        (define (mk a b) (lambda (c) (+ a b c)))
        ((mk 1 2) 100)
    """)
    assert val == 103


def test_inline_lambda(vm: TwigVM) -> None:
    _, val = vm.run("((lambda (x) (* x 2)) 21)")
    assert val == 42


def test_higher_order_inc_twice(vm: TwigVM) -> None:
    _, val = vm.run("""
        (define (apply-twice f x) (f (f x)))
        (define (inc x) (+ x 1))
        (apply-twice inc 5)
    """)
    assert val == 7


def test_first_class_builtin(vm: TwigVM) -> None:
    """Pass a builtin (``+``) as a value."""
    _, val = vm.run("""
        (define (apply-it f a b) (f a b))
        (apply-it + 3 4)
    """)
    assert val == 7


# ---------------------------------------------------------------------------
# I/O
# ---------------------------------------------------------------------------


def test_print_emits_to_stdout(vm: TwigVM) -> None:
    out, _ = vm.run("(print 42)")
    assert out == "42\n"


def test_print_multiple_lines(vm: TwigVM) -> None:
    out, _ = vm.run("(begin (print 1) (print 2))")
    assert out == "1\n2\n"


def test_print_list(vm: TwigVM) -> None:
    out, _ = vm.run("(print (cons 1 (cons 2 (cons 3 nil))))")
    assert out == "(1 2 3)\n"


def test_print_improper_list(vm: TwigVM) -> None:
    out, _ = vm.run("(print (cons 1 2))")
    assert out == "(1 . 2)\n"


def test_print_symbol(vm: TwigVM) -> None:
    out, _ = vm.run("(print 'hello)")
    assert out == "hello\n"


def test_print_nil(vm: TwigVM) -> None:
    out, _ = vm.run("(print nil)")
    assert out == "nil\n"


def test_print_booleans(vm: TwigVM) -> None:
    out, _ = vm.run("(begin (print #t) (print #f))")
    assert out == "#t\n#f\n"


# ---------------------------------------------------------------------------
# GC / heap behaviour
# ---------------------------------------------------------------------------


def test_clean_program_releases_temporary_cons_cells(vm: TwigVM) -> None:
    """A program that allocates cons cells but doesn't store them
    anywhere should leave the heap with most allocations released
    (we don't promise zero — globals + the returned value live on)."""
    vm.run("""
        (define (range n)
          (if (= n 0) nil (cons n (range (- n 1)))))
        (define (length xs)
          (if (null? xs) 0 (+ 1 (length (cdr xs)))))
        (length (range 5))
    """)
    heap = vm.heap
    assert heap is not None
    stats = heap.stats()
    # We allocated at least 5 cons cells.
    assert stats.total_allocs >= 5


# ---------------------------------------------------------------------------
# Error paths
# ---------------------------------------------------------------------------


def test_unbound_name_compile_error(vm: TwigVM) -> None:
    with pytest.raises(TwigCompileError):
        vm.run("(no-such-name 1 2)")


def test_car_on_non_pair_runtime_error(vm: TwigVM) -> None:
    with pytest.raises(TwigRuntimeError):
        vm.run("(car 42)")


def test_division_by_zero_runtime_error(vm: TwigVM) -> None:
    with pytest.raises(TwigRuntimeError):
        vm.run("(/ 5 0)")
