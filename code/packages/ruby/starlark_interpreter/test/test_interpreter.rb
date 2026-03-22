# frozen_string_literal: true

# ==========================================================================
# Tests for the Starlark Interpreter
# ==========================================================================
#
# These tests verify the full Starlark interpretation pipeline, including
# the load() statement support. Tests progress from simple to complex:
#
#   1. Basic interpretation (variables, arithmetic)
#   2. Functions
#   3. Control flow
#   4. Collections
#   5. Print capture
#   6. Load with dictionary resolver
#   7. Load caching
#   8. Load with imported functions
#   9. Load error handling
#  10. File interpretation with temporary files
#  11. BUILD file simulation
# ==========================================================================

require "minitest/autorun"
require "tempfile"
require "coding_adventures_starlark_interpreter"

class TestStarlarkInterpreter < Minitest::Test
  # Helper: interpret source via the module-level convenience method.
  def interpret(source, file_resolver: nil)
    CodingAdventures::StarlarkInterpreter.interpret(
      source,
      file_resolver: file_resolver
    )
  end

  # ================================================================
  # Basic Interpretation
  # ================================================================

  def test_basic_assignment
    result = interpret("x = 42\n")
    assert_equal 42, result.variables["x"]
  end

  def test_arithmetic
    result = interpret("x = 2 + 3 * 4\n")
    assert_equal 14, result.variables["x"]
  end

  def test_string_assignment
    result = interpret("name = \"hello\"\n")
    assert_equal "hello", result.variables["name"]
  end

  def test_multiple_assignments
    result = interpret("x = 1\ny = 2\nz = x + y\n")
    assert_equal 3, result.variables["z"]
  end

  def test_boolean_assignment
    result = interpret("x = True\ny = False\n")
    assert_equal true, result.variables["x"]
    assert_equal false, result.variables["y"]
  end

  def test_none_assignment
    result = interpret("x = None\n")
    assert_nil result.variables["x"]
  end

  # ================================================================
  # Functions
  # ================================================================

  def test_simple_function
    source = <<~STARLARK
      def add(a, c):
          return a + c
      result = add(3, 4)
    STARLARK
    result = interpret(source)
    assert_equal 7, result.variables["result"]
  end

  def test_function_with_defaults
    source = <<~STARLARK
      def greet(name, greeting = "Hello"):
          return greeting + " " + name
      result = greet("world")
    STARLARK
    result = interpret(source)
    assert_equal "Hello world", result.variables["result"]
  end

  def test_recursive_function
    source = <<~STARLARK
      def factorial(n):
          if n <= 1:
              return 1
          return n * factorial(n - 1)
      result = factorial(5)
    STARLARK
    result = interpret(source)
    assert_equal 120, result.variables["result"]
  end

  # ================================================================
  # Control Flow
  # ================================================================

  def test_if_else
    source = <<~STARLARK
      x = 10
      if x > 5:
          y = "large"
      else:
          y = "small"
    STARLARK
    result = interpret(source)
    assert_equal "large", result.variables["y"]
  end

  def test_for_loop
    source = <<~STARLARK
      total = 0
      for i in [1, 2, 3, 4, 5]:
          total = total + i
    STARLARK
    result = interpret(source)
    assert_equal 15, result.variables["total"]
  end

  def test_for_loop_with_range
    source = <<~STARLARK
      total = 0
      for i in range(10):
          total = total + i
    STARLARK
    result = interpret(source)
    assert_equal 45, result.variables["total"]
  end

  # ================================================================
  # Collections
  # ================================================================

  def test_list_operations
    source = <<~STARLARK
      x = [1, 2, 3]
      y = len(x)
    STARLARK
    result = interpret(source)
    assert_equal [1, 2, 3], result.variables["x"]
    assert_equal 3, result.variables["y"]
  end

  def test_dict_operations
    source = <<~STARLARK
      x = {"name": "Alice", "age": 30}
      y = x["name"]
    STARLARK
    result = interpret(source)
    assert_equal "Alice", result.variables["y"]
  end

  def test_tuple_operations
    result = interpret("x = (1, 2, 3)\n")
    assert_equal [1, 2, 3], result.variables["x"]
  end

  # ================================================================
  # Print Capture
  # ================================================================

  def test_print_capture
    result = interpret("print(\"hello world\")\n")
    assert_equal ["hello world"], result.output
  end

  def test_print_multiple_lines
    source = <<~STARLARK
      print("line 1")
      print("line 2")
      print("line 3")
    STARLARK
    result = interpret(source)
    assert_equal ["line 1", "line 2", "line 3"], result.output
  end

  def test_print_with_variables
    source = <<~STARLARK
      x = 42
      print(x)
    STARLARK
    result = interpret(source)
    assert_equal ["42"], result.output
  end

  # ================================================================
  # Load with Dictionary Resolver
  # ================================================================

  def test_load_simple_variable
    resolver = ->(label) {
      files = {"//constants.star" => "PI = 3\n"}
      files[label]
    }
    source = <<~STARLARK
      load("//constants.star", "PI")
      x = PI
    STARLARK
    result = interpret(source, file_resolver: resolver)
    assert_equal 3, result.variables["x"]
  end

  def test_load_function
    resolver = ->(label) {
      files = {
        "//math.star" => "def double(n):\n    return n * 2\n"
      }
      files[label]
    }
    source = <<~STARLARK
      load("//math.star", "double")
      result = double(21)
    STARLARK
    result = interpret(source, file_resolver: resolver)
    assert_equal 42, result.variables["result"]
  end

  def test_load_multiple_symbols
    resolver = ->(label) {
      files = {
        "//defs.star" => "X = 10\nY = 20\n"
      }
      files[label]
    }
    source = <<~STARLARK
      load("//defs.star", "X", "Y")
      result = X + Y
    STARLARK
    result = interpret(source, file_resolver: resolver)
    assert_equal 30, result.variables["result"]
  end

  # ================================================================
  # Load Caching
  # ================================================================

  def test_load_caching
    # Track how many times the resolver is called
    call_count = 0
    resolver = ->(label) {
      call_count += 1
      files = {"//shared.star" => "VALUE = 42\n"}
      files[label]
    }
    # Create an interpreter instance to share the cache
    interp = CodingAdventures::StarlarkInterpreter::Interpreter.new(
      file_resolver: resolver
    )
    # First interpretation loads the module
    result1 = interp.interpret("load(\"//shared.star\", \"VALUE\")\nx = VALUE\n")
    assert_equal 42, result1.variables["x"]
    # Second interpretation should use the cache
    result2 = interp.interpret("load(\"//shared.star\", \"VALUE\")\ny = VALUE\n")
    assert_equal 42, result2.variables["y"]
    # The resolver should have been called only once
    assert_equal 1, call_count
  end

  # ================================================================
  # Load with Functions from Loaded Module
  # ================================================================

  def test_load_and_call_function
    resolver = ->(label) {
      files = {
        "//helpers.star" => <<~STAR
          def add(a, c):
              return a + c
          def mul(a, c):
              return a * c
        STAR
      }
      files[label]
    }
    source = <<~STARLARK
      load("//helpers.star", "add", "mul")
      x = add(3, 4)
      y = mul(5, 6)
    STARLARK
    result = interpret(source, file_resolver: resolver)
    assert_equal 7, result.variables["x"]
    assert_equal 30, result.variables["y"]
  end

  # ================================================================
  # Load Error Handling
  # ================================================================

  def test_load_without_resolver
    assert_raises(RuntimeError) do
      interpret("load(\"//missing.star\", \"X\")\n")
    end
  end

  def test_load_file_not_found
    resolver = ->(_label) { nil }
    assert_raises(RuntimeError) do
      interpret("load(\"//nonexistent.star\", \"X\")\n", file_resolver: resolver)
    end
  end

  # ================================================================
  # File Interpretation
  # ================================================================

  def test_interpret_file
    tmpfile = Tempfile.new(["test", ".star"])
    begin
      tmpfile.write("x = 100\ny = 200\n")
      tmpfile.close

      result = CodingAdventures::StarlarkInterpreter.interpret_file(tmpfile.path)
      assert_equal 100, result.variables["x"]
      assert_equal 200, result.variables["y"]
    ensure
      tmpfile.unlink
    end
  end

  def test_interpret_file_with_function
    tmpfile = Tempfile.new(["test", ".star"])
    begin
      tmpfile.write("def square(n):\n    return n * n\nresult = square(7)\n")
      tmpfile.close

      result = CodingAdventures::StarlarkInterpreter.interpret_file(tmpfile.path)
      assert_equal 49, result.variables["result"]
    ensure
      tmpfile.unlink
    end
  end

  def test_interpret_file_adds_trailing_newline
    tmpfile = Tempfile.new(["test", ".star"])
    begin
      tmpfile.write("x = 42")  # No trailing newline
      tmpfile.close

      result = CodingAdventures::StarlarkInterpreter.interpret_file(tmpfile.path)
      assert_equal 42, result.variables["x"]
    ensure
      tmpfile.unlink
    end
  end

  # ================================================================
  # BUILD File Simulation
  # ================================================================
  #
  # This test simulates a simplified Bazel/Buck BUILD file scenario
  # where a BUILD file loads rules from a .star file and uses them.

  def test_build_file_simulation
    resolver = ->(label) {
      files = {
        "//rules.star" => <<~STAR
          def cc_library(name, srcs):
              return {"name": name, "srcs": srcs, "type": "cc_library"}
        STAR
      }
      files[label]
    }
    source = <<~STARLARK
      load("//rules.star", "cc_library")
      lib = cc_library("mylib", ["main.cc", "util.cc"])
    STARLARK
    result = interpret(source, file_resolver: resolver)
    lib = result.variables["lib"]
    assert_equal "mylib", lib["name"]
    assert_equal ["main.cc", "util.cc"], lib["srcs"]
    assert_equal "cc_library", lib["type"]
  end

  # ================================================================
  # Interpreter Instance
  # ================================================================

  def test_interpreter_instance_creation
    interp = CodingAdventures::StarlarkInterpreter::Interpreter.new
    assert_nil interp.file_resolver
    assert_equal 200, interp.max_recursion_depth
  end

  def test_interpreter_custom_recursion_depth
    interp = CodingAdventures::StarlarkInterpreter::Interpreter.new(
      max_recursion_depth: 50
    )
    assert_equal 50, interp.max_recursion_depth
  end

  def test_interpreter_with_resolver
    resolver = ->(label) { "X = 1\n" }
    interp = CodingAdventures::StarlarkInterpreter::Interpreter.new(
      file_resolver: resolver
    )
    refute_nil interp.file_resolver
  end

  # ================================================================
  # Result Structure
  # ================================================================

  def test_result_has_variables
    result = interpret("x = 42\n")
    assert_instance_of Hash, result.variables
    assert_equal 42, result.variables["x"]
  end

  def test_result_has_output
    result = interpret("print(\"test\")\n")
    assert_instance_of Array, result.output
    assert_equal ["test"], result.output
  end

  def test_result_has_traces
    result = interpret("x = 42\n")
    assert_instance_of Array, result.traces
    refute_empty result.traces
  end

  # ================================================================
  # Integration: Complex Programs
  # ================================================================

  def test_complex_program
    source = <<~STARLARK
      def fizz(n):
          result = []
          for i in range(1, n + 1):
              if i % 15 == 0:
                  result.append("FizzFuzz")
              elif i % 3 == 0:
                  result.append("Fizz")
              elif i % 5 == 0:
                  result.append("Fuzz")
              else:
                  result.append(i)
          return result
      output = fizz(15)
    STARLARK
    result = interpret(source)
    output = result.variables["output"]
    assert_equal 15, output.length
    assert_equal "Fizz", output[2]     # 3
    assert_equal "Fuzz", output[4]     # 5
    assert_equal "Fizz", output[5]     # 6
    assert_equal "FizzFuzz", output[14] # 15
    assert_equal 1, output[0]          # 1
    assert_equal 2, output[1]          # 2
  end

  def test_list_manipulation
    source = <<~STARLARK
      items = [3, 1, 4, 1, 5, 9]
      s = sorted(items)
      r = reversed(items)
      total = 0
      for x in items:
          total = total + x
    STARLARK
    result = interpret(source)
    assert_equal [1, 1, 3, 4, 5, 9], result.variables["s"]
    assert_equal [9, 5, 1, 4, 1, 3], result.variables["r"]
    assert_equal 23, result.variables["total"]
  end
end
