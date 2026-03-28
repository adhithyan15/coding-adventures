# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_lisp_compiler"

LC = CodingAdventures::LispCompiler
NIL_V = CodingAdventures::LispVm::NIL

# ================================================================
# Tests for the Lisp Compiler
# ================================================================
#
# These tests compile Lisp source code and run the resulting bytecode.
# We verify the final stack value, variable bindings, and output.
# ================================================================

class TestLispCompiler < Minitest::Test
  def run_lisp(source)
    LC.run_lisp(source)
  end

  def test_version_exists
    refute_nil LC::VERSION
  end

  # ------------------------------------------------------------------
  # Basic atoms
  # ------------------------------------------------------------------

  def test_compile_integer
    vm = run_lisp("42")
    assert_equal 42, vm.stack.last
  end

  def test_compile_nil
    vm = run_lisp("nil")
    assert_equal NIL_V, vm.stack.last
  end

  # ------------------------------------------------------------------
  # Arithmetic
  # ------------------------------------------------------------------

  def test_add
    vm = run_lisp("(+ 1 2)")
    assert_equal 3, vm.stack.last
  end

  def test_sub
    vm = run_lisp("(- 10 3)")
    assert_equal 7, vm.stack.last
  end

  def test_mul
    vm = run_lisp("(* 6 7)")
    assert_equal 42, vm.stack.last
  end

  def test_nested_arithmetic
    vm = run_lisp("(+ (* 2 3) (- 10 4))")
    assert_equal 12, vm.stack.last
  end

  # ------------------------------------------------------------------
  # define
  # ------------------------------------------------------------------

  def test_define_binds_variable
    vm = run_lisp("(define x 42)")
    assert_equal 42, vm.variables["x"]
  end

  def test_define_then_use
    vm = run_lisp("(define x 10) (+ x 5)")
    assert_equal 15, vm.stack.last
  end

  # ------------------------------------------------------------------
  # cond
  # ------------------------------------------------------------------

  def test_cond_first_branch
    vm = run_lisp("(cond ((eq 1 1) 99) ((eq 1 2) 0))")
    assert_equal 99, vm.stack.last
  end

  def test_cond_second_branch
    vm = run_lisp("(cond ((eq 1 2) 99) ((eq 1 1) 42))")
    assert_equal 42, vm.stack.last
  end

  # ------------------------------------------------------------------
  # cons / car / cdr
  # ------------------------------------------------------------------

  def test_cons_and_car
    vm = run_lisp("(car (cons 1 nil))")
    assert_equal 1, vm.stack.last
  end

  def test_cons_and_cdr
    vm = run_lisp("(cdr (cons 1 nil))")
    assert_equal NIL_V, vm.stack.last
  end

  # ------------------------------------------------------------------
  # print
  # ------------------------------------------------------------------

  def test_print_output
    vm = run_lisp("(print 42)")
    assert_equal ["42"], vm.output
  end

  # ------------------------------------------------------------------
  # lambda and call
  # ------------------------------------------------------------------

  def test_lambda_identity
    vm = run_lisp("(define id (lambda (x) x)) (id 42)")
    assert_equal 42, vm.stack.last
  end

  def test_lambda_addition
    vm = run_lisp("(define add (lambda (a b) (+ a b))) (add 3 4)")
    assert_equal 7, vm.stack.last
  end
end
