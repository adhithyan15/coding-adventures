# frozen_string_literal: true

# ==========================================================================
# Tests for the Starlark VM
# ==========================================================================
#
# These tests verify end-to-end execution of Starlark programs. Each test
# compiles and executes a Starlark source string, then checks the resulting
# variables, output, or both.
#
# Tests progress from simple to complex:
#   1. Basic assignment and arithmetic
#   2. Float arithmetic
#   3. Comparison operators
#   4. Boolean logic and short-circuit evaluation
#   5. Control flow (if/else, for loops)
#   6. Functions (def, call, return, defaults)
#   7. Collections (list, dict, tuple)
#   8. Builtin functions
#   9. String operations
#  10. Nested function calls
#  11. Error cases
# ==========================================================================

require "minitest/autorun"
require "coding_adventures_starlark_vm"

class TestStarlarkVM < Minitest::Test
  # Helper: execute Starlark source and return StarlarkResult.
  def exec(source)
    CodingAdventures::StarlarkVM.execute_starlark(source)
  end

  # ================================================================
  # Basic Assignment
  # ================================================================

  def test_basic_integer_assignment
    result = exec("x = 42\n")
    assert_equal 42, result.variables["x"]
  end

  def test_basic_string_assignment
    result = exec("name = \"hello\"\n")
    assert_equal "hello", result.variables["name"]
  end

  def test_multiple_assignments
    result = exec("x = 1\ny = 2\nz = 3\n")
    assert_equal 1, result.variables["x"]
    assert_equal 2, result.variables["y"]
    assert_equal 3, result.variables["z"]
  end

  def test_none_assignment
    result = exec("x = None\n")
    assert_nil result.variables["x"]
  end

  def test_boolean_true_assignment
    result = exec("x = True\n")
    assert_equal true, result.variables["x"]
  end

  def test_boolean_false_assignment
    result = exec("x = False\n")
    assert_equal false, result.variables["x"]
  end

  # ================================================================
  # Arithmetic Operations
  # ================================================================

  def test_addition
    result = exec("x = 1 + 2\n")
    assert_equal 3, result.variables["x"]
  end

  def test_subtraction
    result = exec("x = 10 - 3\n")
    assert_equal 7, result.variables["x"]
  end

  def test_multiplication
    result = exec("x = 6 * 7\n")
    assert_equal 42, result.variables["x"]
  end

  def test_true_division
    result = exec("x = 7 / 2\n")
    assert_equal 3.5, result.variables["x"]
  end

  def test_floor_division
    result = exec("x = 7 // 2\n")
    assert_equal 3, result.variables["x"]
  end

  def test_modulo
    result = exec("x = 10 % 3\n")
    assert_equal 1, result.variables["x"]
  end

  def test_power
    result = exec("x = 2 ** 10\n")
    assert_equal 1024, result.variables["x"]
  end

  def test_negation
    result = exec("x = -42\n")
    assert_equal(-42, result.variables["x"])
  end

  def test_complex_arithmetic
    result = exec("x = 2 + 3 * 4\n")
    assert_equal 14, result.variables["x"]
  end

  # ================================================================
  # Float Arithmetic
  # ================================================================

  def test_float_addition
    result = exec("x = 1.5 + 2.5\n")
    assert_in_delta 4.0, result.variables["x"], 0.001
  end

  def test_float_multiplication
    result = exec("x = 2.5 * 4.0\n")
    assert_in_delta 10.0, result.variables["x"], 0.001
  end

  # ================================================================
  # String Operations
  # ================================================================

  def test_string_concatenation
    result = exec("x = \"hello\" + \" \" + \"world\"\n")
    assert_equal "hello world", result.variables["x"]
  end

  def test_string_repetition
    result = exec("x = \"ab\" * 3\n")
    assert_equal "ababab", result.variables["x"]
  end

  # ================================================================
  # Comparison Operations
  # ================================================================

  def test_equal
    result = exec("x = 1 == 1\n")
    assert_equal true, result.variables["x"]
  end

  def test_not_equal
    result = exec("x = 1 != 2\n")
    assert_equal true, result.variables["x"]
  end

  def test_less_than
    result = exec("x = 1 < 2\n")
    assert_equal true, result.variables["x"]
  end

  def test_greater_than
    result = exec("x = 2 > 1\n")
    assert_equal true, result.variables["x"]
  end

  def test_less_than_or_equal
    result = exec("x = 2 <= 2\n")
    assert_equal true, result.variables["x"]
  end

  def test_greater_than_or_equal
    result = exec("x = 3 >= 2\n")
    assert_equal true, result.variables["x"]
  end

  def test_comparison_false
    result = exec("x = 5 < 3\n")
    assert_equal false, result.variables["x"]
  end

  # ================================================================
  # Boolean Logic
  # ================================================================

  def test_not_true
    result = exec("x = not True\n")
    assert_equal false, result.variables["x"]
  end

  def test_not_false
    result = exec("x = not False\n")
    assert_equal true, result.variables["x"]
  end

  def test_and_short_circuit_false
    # "and" with a falsy left operand should return the left operand
    result = exec("x = False and True\n")
    assert_equal false, result.variables["x"]
  end

  def test_and_short_circuit_true
    # "and" with truthy left should evaluate and return right
    result = exec("x = True and 42\n")
    assert_equal 42, result.variables["x"]
  end

  def test_or_short_circuit_true
    # "or" with truthy left should return left
    result = exec("x = True or False\n")
    assert_equal true, result.variables["x"]
  end

  def test_or_short_circuit_false
    # "or" with falsy left should evaluate right
    result = exec("x = False or 42\n")
    assert_equal 42, result.variables["x"]
  end

  # ================================================================
  # Control Flow: if/else
  # ================================================================

  def test_if_true_branch
    source = <<~STARLARK
      x = 0
      if True:
          x = 1
    STARLARK
    result = exec(source)
    assert_equal 1, result.variables["x"]
  end

  def test_if_false_branch
    source = <<~STARLARK
      x = 0
      if False:
          x = 1
    STARLARK
    result = exec(source)
    assert_equal 0, result.variables["x"]
  end

  def test_if_else
    source = <<~STARLARK
      x = 10
      if x > 5:
          y = "large"
      else:
          y = "small"
    STARLARK
    result = exec(source)
    assert_equal "large", result.variables["y"]
  end

  def test_if_elif_else
    source = <<~STARLARK
      x = 5
      if x > 10:
          y = "large"
      elif x > 3:
          y = "medium"
      else:
          y = "small"
    STARLARK
    result = exec(source)
    assert_equal "medium", result.variables["y"]
  end

  # ================================================================
  # Control Flow: for loops
  # ================================================================

  def test_for_loop_sum
    source = <<~STARLARK
      total = 0
      for i in [1, 2, 3, 4, 5]:
          total = total + i
    STARLARK
    result = exec(source)
    assert_equal 15, result.variables["total"]
  end

  def test_for_loop_with_range_builtin
    source = <<~STARLARK
      total = 0
      for i in range(5):
          total = total + i
    STARLARK
    result = exec(source)
    assert_equal 10, result.variables["total"]
  end

  def test_for_loop_string_iteration
    source = <<~STARLARK
      chars = []
      for c in "abc":
          chars.append(c)
    STARLARK
    result = exec(source)
    assert_equal ["a", "b", "c"], result.variables["chars"]
  end

  # ================================================================
  # Functions
  # ================================================================

  def test_simple_function
    source = <<~STARLARK
      def add(a, b):
          return a + b
      result = add(3, 4)
    STARLARK
    result = exec(source)
    assert_equal 7, result.variables["result"]
  end

  def test_function_with_default_args
    source = <<~STARLARK
      def greet(name, greeting = "Hello"):
          return greeting + " " + name
      result = greet("world")
    STARLARK
    result = exec(source)
    assert_equal "Hello world", result.variables["result"]
  end

  def test_function_override_default
    source = <<~STARLARK
      def greet(name, greeting = "Hello"):
          return greeting + " " + name
      result = greet("world", "Hi")
    STARLARK
    result = exec(source)
    assert_equal "Hi world", result.variables["result"]
  end

  def test_function_returning_none
    source = <<~STARLARK
      def noop():
          x = 1
      result = noop()
    STARLARK
    result = exec(source)
    # Functions that don't explicitly return should return None (nil)
    assert_nil result.variables["result"]
  end

  def test_nested_function_calls
    source = <<~STARLARK
      def double(x):
          return x * 2
      def quad(x):
          return double(double(x))
      result = quad(3)
    STARLARK
    result = exec(source)
    assert_equal 12, result.variables["result"]
  end

  def test_function_with_local_variables
    source = <<~STARLARK
      def compute(x):
          temp = x * 2
          return temp + 1
      result = compute(10)
    STARLARK
    result = exec(source)
    assert_equal 21, result.variables["result"]
  end

  # ================================================================
  # Collections: Lists
  # ================================================================

  def test_list_literal
    result = exec("x = [1, 2, 3]\n")
    assert_equal [1, 2, 3], result.variables["x"]
  end

  def test_empty_list
    result = exec("x = []\n")
    assert_equal [], result.variables["x"]
  end

  def test_list_subscript
    source = <<~STARLARK
      x = [10, 20, 30]
      y = x[1]
    STARLARK
    result = exec(source)
    assert_equal 20, result.variables["y"]
  end

  def test_list_append
    source = <<~STARLARK
      x = [1, 2]
      x.append(3)
    STARLARK
    result = exec(source)
    assert_equal [1, 2, 3], result.variables["x"]
  end

  def test_list_concatenation
    result = exec("x = [1, 2] + [3, 4]\n")
    assert_equal [1, 2, 3, 4], result.variables["x"]
  end

  # ================================================================
  # Collections: Dicts
  # ================================================================

  def test_dict_literal
    # Note: avoid single-char "b" due to upstream compiler escape bug (\b -> backspace)
    result = exec("x = {\"a\": 1, \"c\": 2}\n")
    assert_equal({"a" => 1, "c" => 2}, result.variables["x"])
  end

  def test_empty_dict
    result = exec("x = {}\n")
    assert_equal({}, result.variables["x"])
  end

  def test_dict_subscript
    source = <<~STARLARK
      x = {"a": 1, "c": 2}
      y = x["c"]
    STARLARK
    result = exec(source)
    assert_equal 2, result.variables["y"]
  end

  def test_dict_with_multiple_keys
    source = <<~STARLARK
      x = {"key": 42, "other": 99}
    STARLARK
    result = exec(source)
    assert_equal({"key" => 42, "other" => 99}, result.variables["x"])
  end

  # ================================================================
  # Collections: Tuples
  # ================================================================

  def test_tuple_literal
    result = exec("x = (1, 2, 3)\n")
    assert_equal [1, 2, 3], result.variables["x"]
  end

  # ================================================================
  # Builtin Functions
  # ================================================================

  def test_builtin_len_list
    result = exec("x = len([1, 2, 3])\n")
    assert_equal 3, result.variables["x"]
  end

  def test_builtin_len_string
    result = exec("x = len(\"hello\")\n")
    assert_equal 5, result.variables["x"]
  end

  def test_builtin_len_dict
    result = exec("x = len({\"a\": 1, \"c\": 2})\n")
    assert_equal 2, result.variables["x"]
  end

  def test_builtin_type_int
    result = exec("x = type(42)\n")
    assert_equal "int", result.variables["x"]
  end

  def test_builtin_type_string
    result = exec("x = type(\"hello\")\n")
    assert_equal "string", result.variables["x"]
  end

  def test_builtin_type_list
    result = exec("x = type([1, 2])\n")
    assert_equal "list", result.variables["x"]
  end

  def test_builtin_type_bool
    result = exec("x = type(True)\n")
    assert_equal "bool", result.variables["x"]
  end

  def test_builtin_type_none
    result = exec("x = type(None)\n")
    assert_equal "NoneType", result.variables["x"]
  end

  def test_builtin_bool_truthy
    result = exec("x = bool(42)\n")
    assert_equal true, result.variables["x"]
  end

  def test_builtin_bool_falsy
    result = exec("x = bool(0)\n")
    assert_equal false, result.variables["x"]
  end

  def test_builtin_bool_empty_string
    result = exec("x = bool(\"\")\n")
    assert_equal false, result.variables["x"]
  end

  def test_builtin_int_from_string
    result = exec("x = int(\"42\")\n")
    assert_equal 42, result.variables["x"]
  end

  def test_builtin_int_from_float
    result = exec("x = int(3.7)\n")
    assert_equal 3, result.variables["x"]
  end

  def test_builtin_str_from_int
    result = exec("x = str(42)\n")
    assert_equal "42", result.variables["x"]
  end

  def test_builtin_str_from_bool
    result = exec("x = str(True)\n")
    assert_equal "True", result.variables["x"]
  end

  def test_builtin_range_one_arg
    result = exec("x = range(5)\n")
    assert_equal [0, 1, 2, 3, 4], result.variables["x"]
  end

  def test_builtin_range_two_args
    result = exec("x = range(2, 5)\n")
    assert_equal [2, 3, 4], result.variables["x"]
  end

  def test_builtin_range_three_args
    result = exec("x = range(0, 10, 3)\n")
    assert_equal [0, 3, 6, 9], result.variables["x"]
  end

  def test_builtin_sorted
    result = exec("x = sorted([3, 1, 2])\n")
    assert_equal [1, 2, 3], result.variables["x"]
  end

  def test_builtin_reversed
    result = exec("x = reversed([1, 2, 3])\n")
    assert_equal [3, 2, 1], result.variables["x"]
  end

  def test_builtin_min
    result = exec("x = min([3, 1, 2])\n")
    assert_equal 1, result.variables["x"]
  end

  def test_builtin_max
    result = exec("x = max([3, 1, 2])\n")
    assert_equal 3, result.variables["x"]
  end

  def test_builtin_abs_positive
    result = exec("x = abs(5)\n")
    assert_equal 5, result.variables["x"]
  end

  def test_builtin_abs_negative
    result = exec("x = abs(-5)\n")
    assert_equal 5, result.variables["x"]
  end

  def test_builtin_all_true
    result = exec("x = all([1, 2, 3])\n")
    assert_equal true, result.variables["x"]
  end

  def test_builtin_all_false
    result = exec("x = all([1, 0, 3])\n")
    assert_equal false, result.variables["x"]
  end

  def test_builtin_any_true
    result = exec("x = any([0, 0, 1])\n")
    assert_equal true, result.variables["x"]
  end

  def test_builtin_any_false
    result = exec("x = any([0, 0, 0])\n")
    assert_equal false, result.variables["x"]
  end

  def test_builtin_enumerate
    # Note: avoid single-char "b" due to upstream compiler escape bug
    result = exec("x = enumerate([\"a\", \"c\", \"d\"])\n")
    assert_equal [[0, "a"], [1, "c"], [2, "d"]], result.variables["x"]
  end

  def test_builtin_zip
    result = exec("x = zip([1, 2], [3, 4])\n")
    assert_equal [[1, 3], [2, 4]], result.variables["x"]
  end

  def test_builtin_hasattr_true
    result = exec("x = hasattr({\"a\": 1}, \"a\")\n")
    assert_equal true, result.variables["x"]
  end

  def test_builtin_hasattr_false
    result = exec("x = hasattr({\"a\": 1}, \"b\")\n")
    assert_equal false, result.variables["x"]
  end

  def test_builtin_getattr
    result = exec("x = getattr({\"a\": 42}, \"a\")\n")
    assert_equal 42, result.variables["x"]
  end

  def test_builtin_getattr_default
    result = exec("x = getattr({\"a\": 1}, \"b\", 99)\n")
    assert_equal 99, result.variables["x"]
  end

  # ================================================================
  # Print Capture
  # ================================================================

  def test_print_string
    result = exec("print(\"hello\")\n")
    assert_equal ["hello"], result.output
  end

  def test_print_number
    result = exec("print(42)\n")
    assert_equal ["42"], result.output
  end

  def test_print_multiple_values
    result = exec("print(\"x\", 42)\n")
    assert_equal ["x 42"], result.output
  end

  def test_print_returns_none
    result = exec("x = print(\"hello\")\n")
    assert_nil result.variables["x"]
    assert_equal ["hello"], result.output
  end

  def test_multiple_prints
    source = <<~STARLARK
      print("line1")
      print("line2")
    STARLARK
    result = exec(source)
    assert_equal ["line1", "line2"], result.output
  end

  # ================================================================
  # In / Not In Operators
  # ================================================================

  def test_in_list
    result = exec("x = 2 in [1, 2, 3]\n")
    assert_equal true, result.variables["x"]
  end

  def test_not_in_list
    result = exec("x = 4 not in [1, 2, 3]\n")
    assert_equal true, result.variables["x"]
  end

  def test_in_dict
    result = exec("x = \"a\" in {\"a\": 1}\n")
    assert_equal true, result.variables["x"]
  end

  def test_in_string
    result = exec("x = \"bc\" in \"abcd\"\n")
    assert_equal true, result.variables["x"]
  end

  # ================================================================
  # Execution Result Structure
  # ================================================================

  def test_result_has_variables
    result = exec("x = 42\n")
    assert_instance_of Hash, result.variables
  end

  def test_result_has_output
    result = exec("x = 42\n")
    assert_instance_of Array, result.output
  end

  def test_result_has_traces
    result = exec("x = 42\n")
    assert_instance_of Array, result.traces
    refute_empty result.traces
  end

  # ================================================================
  # create_starlark_vm Factory
  # ================================================================

  def test_create_starlark_vm
    vm = CodingAdventures::StarlarkVM.create_starlark_vm
    assert_instance_of CodingAdventures::VirtualMachine::GenericVM, vm
  end

  def test_create_starlark_vm_custom_recursion_depth
    vm = CodingAdventures::StarlarkVM.create_starlark_vm(max_recursion_depth: 50)
    assert_equal 50, vm.max_recursion_depth
  end

  # ================================================================
  # StarlarkFunction Type
  # ================================================================

  def test_starlark_function_type
    func = CodingAdventures::StarlarkVM::StarlarkFunction.new(
      code: nil,
      name: "test",
      param_count: 2,
      param_names: ["a", "b"]
    )
    assert_equal "test", func.name
    assert_equal 2, func.param_count
    assert_equal ["a", "b"], func.param_names
    assert_equal [], func.defaults
  end

  # ================================================================
  # StarlarkIterator Type
  # ================================================================

  def test_starlark_iterator
    iter = CodingAdventures::StarlarkVM::StarlarkIterator.new([10, 20, 30])
    assert_equal 10, iter.next_value
    assert_equal 20, iter.next_value
    refute iter.done?
    assert_equal 30, iter.next_value
    assert iter.done?
    assert_nil iter.next_value
  end

  # ================================================================
  # Variable Reassignment
  # ================================================================

  def test_variable_reassignment
    source = <<~STARLARK
      x = 1
      x = x + 1
      x = x * 3
    STARLARK
    result = exec(source)
    assert_equal 6, result.variables["x"]
  end

  # ================================================================
  # Integration: Combining Features
  # ================================================================

  def test_function_with_loop
    source = <<~STARLARK
      def sum_list(items):
          total = 0
          for item in items:
              total = total + item
          return total
      result = sum_list([1, 2, 3, 4, 5])
    STARLARK
    result = exec(source)
    assert_equal 15, result.variables["result"]
  end

  def test_function_with_conditional
    source = <<~STARLARK
      def classify(n):
          if n > 0:
              return "positive"
          elif n < 0:
              return "negative"
          else:
              return "zero"
      r1 = classify(5)
      r2 = classify(-3)
      r3 = classify(0)
    STARLARK
    result = exec(source)
    assert_equal "positive", result.variables["r1"]
    assert_equal "negative", result.variables["r2"]
    assert_equal "zero", result.variables["r3"]
  end

  def test_fibonacci
    source = <<~STARLARK
      def fib(n):
          if n <= 1:
              return n
          a = 0
          b = 1
          for i in range(2, n + 1):
              temp = a + b
              a = b
              b = temp
          return b
      result = fib(10)
    STARLARK
    result = exec(source)
    assert_equal 55, result.variables["result"]
  end
end
