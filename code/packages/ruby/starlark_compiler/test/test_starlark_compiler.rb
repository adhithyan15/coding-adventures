# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_starlark_compiler"

SC = CodingAdventures::StarlarkCompiler

class TestStarlarkCompiler < Minitest::Test
  def test_version_exists
    refute_nil SC::VERSION
  end

  def test_compile_returns_code_object
    code = SC.compile("x = 1 + 2\n")
    refute_nil code
    refute_empty code.instructions
  end

  def test_compile_and_run_assignment
    result = SC.compile_and_run("x = 42\n")
    assert_equal 42, result[:variables]["x"]
  end

  def test_compile_and_run_arithmetic
    result = SC.compile_and_run("y = 3 * 7\n")
    assert_equal 21, result[:variables]["y"]
  end

  def test_compile_and_run_multiple_assignments
    result = SC.compile_and_run("a = 10\nb = 20\n")
    assert_equal 10, result[:variables]["a"]
    assert_equal 20, result[:variables]["b"]
  end

  def test_evaluate_returns_variables
    vars = SC.evaluate("z = 100\n")
    assert_equal 100, vars["z"]
  end

  def test_compile_produces_halt
    code = SC.compile("x = 1\n")
    opcodes = code.instructions.map(&:opcode)
    # The compiled code must end with HALT (0xFF)
    assert_equal 0xFF, opcodes.last
  end
end
