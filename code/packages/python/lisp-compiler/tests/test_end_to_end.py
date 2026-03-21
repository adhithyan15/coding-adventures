"""End-to-end tests — From Lisp source to execution results.

These tests compile AND execute Lisp code, verifying that the entire
pipeline works: lexer → parser → compiler → VM → result.

This is where we prove that McCarthy's 1960 Lisp actually runs.
"""

from __future__ import annotations

import pytest
from lisp_compiler import run_lisp
from lisp_vm import NIL


class TestArithmetic:
    """Test arithmetic operations end-to-end."""

    def test_add(self) -> None:
        assert run_lisp("(+ 1 2)") == 3

    def test_subtract(self) -> None:
        assert run_lisp("(- 10 3)") == 7

    def test_multiply(self) -> None:
        assert run_lisp("(* 4 5)") == 20

    def test_divide(self) -> None:
        assert run_lisp("(/ 10 2)") == 5

    def test_nested_arithmetic(self) -> None:
        assert run_lisp("(+ (* 2 3) (- 10 4))") == 12

    def test_deeply_nested(self) -> None:
        assert run_lisp("(* (+ 1 2) (+ 3 4))") == 21


class TestComparisons:
    """Test comparison operations end-to-end."""

    def test_eq_true(self) -> None:
        assert run_lisp("(eq 1 1)") == 1

    def test_eq_false(self) -> None:
        assert run_lisp("(eq 1 2)") == 0

    def test_less_than(self) -> None:
        assert run_lisp("(< 1 2)") == 1

    def test_greater_than(self) -> None:
        assert run_lisp("(> 3 2)") == 1


class TestDefine:
    """Test variable definitions."""

    def test_define_and_use(self) -> None:
        assert run_lisp("(define x 42) x") == 42

    def test_define_expression(self) -> None:
        assert run_lisp("(define x (+ 1 2)) x") == 3

    def test_multiple_defines(self) -> None:
        assert run_lisp("(define x 10) (define y 20) (+ x y)") == 30


class TestCond:
    """Test conditional expressions."""

    def test_cond_true_branch(self) -> None:
        assert run_lisp("(cond ((eq 1 1) 42) (t 0))") == 42

    def test_cond_false_branch(self) -> None:
        assert run_lisp("(cond ((eq 1 2) 42) (t 99))") == 99

    def test_cond_multiple_branches(self) -> None:
        result = run_lisp("""
            (define x 2)
            (cond ((eq x 1) 10)
                  ((eq x 2) 20)
                  (t 30))
        """)
        assert result == 20

    def test_cond_else_only(self) -> None:
        assert run_lisp("(cond (t 42))") == 42


class TestLambda:
    """Test lambda expressions and function calls."""

    def test_identity_function(self) -> None:
        assert run_lisp("((lambda (x) x) 42)") == 42

    def test_simple_function(self) -> None:
        assert run_lisp("((lambda (x) (+ x 1)) 41)") == 42

    def test_two_args(self) -> None:
        assert run_lisp("((lambda (x y) (+ x y)) 10 20)") == 30

    def test_named_function(self) -> None:
        result = run_lisp("""
            (define double (lambda (x) (* x 2)))
            (double 21)
        """)
        assert result == 42

    def test_closure_captures_env(self) -> None:
        """Closures should capture the environment at creation time."""
        result = run_lisp("""
            (define y 10)
            (define add-y (lambda (x) (+ x y)))
            (add-y 32)
        """)
        assert result == 42


class TestConsCells:
    """Test cons cell operations."""

    def test_cons_car(self) -> None:
        assert run_lisp("(car (cons 1 2))") == 1

    def test_cons_cdr(self) -> None:
        assert run_lisp("(cdr (cons 1 2))") == 2

    def test_nested_cons(self) -> None:
        """(car (cdr (cons 1 (cons 2 3)))) should be 2."""
        assert run_lisp("(car (cdr (cons 1 (cons 2 3))))") == 2


class TestQuote:
    """Test quoted data construction."""

    def test_quote_number(self) -> None:
        assert run_lisp("(quote 42)") == 42

    def test_quote_nil(self) -> None:
        assert run_lisp("(quote nil)") is NIL

    def test_shorthand_quote_number(self) -> None:
        assert run_lisp("'42") == 42

    def test_quote_list_car(self) -> None:
        """(car (quote (1 2 3))) should be 1."""
        assert run_lisp("(car (quote (1 2 3)))") == 1

    def test_quote_list_cdr_car(self) -> None:
        """(car (cdr (quote (1 2 3)))) should be 2."""
        assert run_lisp("(car (cdr (quote (1 2 3))))") == 2

    def test_quote_empty_list(self) -> None:
        """(quote ()) is NIL."""
        assert run_lisp("(quote ())") is NIL


class TestPredicates:
    """Test predicate operations."""

    def test_atom_number(self) -> None:
        assert run_lisp("(atom 42)") == 1

    def test_atom_cons(self) -> None:
        assert run_lisp("(atom (cons 1 2))") == 0

    def test_is_nil_true(self) -> None:
        assert run_lisp("(is-nil nil)") == 1

    def test_is_nil_false(self) -> None:
        assert run_lisp("(is-nil 42)") == 0


class TestRecursion:
    """Test recursive functions — the crown jewel."""

    def test_factorial(self) -> None:
        """The rite of passage: (factorial 5) = 120."""
        result = run_lisp("""
            (define factorial
              (lambda (n)
                (cond ((eq n 0) 1)
                      (t (* n (factorial (- n 1)))))))
            (factorial 5)
        """)
        assert result == 120

    def test_factorial_10(self) -> None:
        result = run_lisp("""
            (define factorial
              (lambda (n)
                (cond ((eq n 0) 1)
                      (t (* n (factorial (- n 1)))))))
            (factorial 10)
        """)
        assert result == 3628800

    def test_fibonacci(self) -> None:
        """Fibonacci — doubly recursive."""
        result = run_lisp("""
            (define fib
              (lambda (n)
                (cond ((eq n 0) 0)
                      ((eq n 1) 1)
                      (t (+ (fib (- n 1)) (fib (- n 2)))))))
            (fib 10)
        """)
        assert result == 55


class TestTailCallOptimization:
    """Test that tail-recursive functions don't overflow the stack."""

    def test_tail_recursive_factorial(self) -> None:
        """Factorial with accumulator — should use TAIL_CALL."""
        result = run_lisp("""
            (define factorial-iter
              (lambda (n acc)
                (cond ((eq n 0) acc)
                      (t (factorial-iter (- n 1) (* n acc))))))
            (factorial-iter 10 1)
        """)
        assert result == 3628800

    def test_tail_call_large_n(self) -> None:
        """Tail-recursive count-down — should handle large N without
        stack overflow thanks to TAIL_CALL optimization."""
        result = run_lisp("""
            (define countdown
              (lambda (n)
                (cond ((eq n 0) 0)
                      (t (countdown (- n 1))))))
            (countdown 10000)
        """)
        assert result == 0

    def test_tail_recursive_factorial_100(self) -> None:
        """Factorial(100) — a very large number, enabled by TCO."""
        result = run_lisp("""
            (define factorial-iter
              (lambda (n acc)
                (cond ((eq n 0) acc)
                      (t (factorial-iter (- n 1) (* n acc))))))
            (factorial-iter 100 1)
        """)
        expected = 93326215443944152681699238856266700490715968264381621468592963895217599993229915608941463976156518286253697920827223758251185210916864000000000000000000000000
        assert result == expected


class TestSymbols:
    """Test symbol interning via quote."""

    def test_eq_symbols(self) -> None:
        """Two quoted symbols with the same name should be eq."""
        assert run_lisp("(eq (quote foo) (quote foo))") == 1

    def test_neq_symbols(self) -> None:
        """Two quoted symbols with different names should not be eq."""
        assert run_lisp("(eq (quote foo) (quote bar))") == 0


class TestEdgeCases:
    """Test edge cases and interesting combinations."""

    def test_nil_literal(self) -> None:
        assert run_lisp("nil") is NIL

    def test_t_literal(self) -> None:
        assert run_lisp("t") is True

    def test_empty_list_is_nil(self) -> None:
        assert run_lisp("()") is NIL

    def test_higher_order_function(self) -> None:
        """Passing a function as an argument."""
        result = run_lisp("""
            (define apply-to-5 (lambda (f) (f 5)))
            (define double (lambda (x) (* x 2)))
            (apply-to-5 double)
        """)
        assert result == 10

    def test_currying(self) -> None:
        """A function that returns a function."""
        result = run_lisp("""
            (define make-adder (lambda (x) (lambda (y) (+ x y))))
            (define add-10 (make-adder 10))
            (add-10 32)
        """)
        assert result == 42
