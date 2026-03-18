# frozen_string_literal: true

require_relative "test_helper"

P = CodingAdventures::Parser unless defined?(P)

# Comprehensive tests for the CLR IL Compiler.
#
# Tests verify the tiered encoding (ldc.i4.N, ldc.i4.s, ldc.i4),
# local variable short/long forms, arithmetic opcodes, and end-to-end
# bytecode sequences.

class TestCLRNumberEncoding < Minitest::Test
  include CodingAdventures::BytecodeCompiler

  def compile_assignment(value)
    program = P::Program.new(
      statements: [
        P::Assignment.new(
          target: P::Name.new(name: "x"),
          value: P::NumberLiteral.new(value: value)
        )
      ]
    )
    CLRCompiler.new.compile(program)
  end

  def test_ldc_i4_0
    code = compile_assignment(0)
    assert_equal CLR_LDC_I4_0, code.bytecode.getbyte(0)
  end

  def test_ldc_i4_1
    code = compile_assignment(1)
    assert_equal CLR_LDC_I4_1, code.bytecode.getbyte(0)
  end

  def test_ldc_i4_8
    code = compile_assignment(8)
    assert_equal CLR_LDC_I4_8, code.bytecode.getbyte(0)
  end

  def test_ldc_i4_s_for_9
    code = compile_assignment(9)
    assert_equal CLR_LDC_I4_S, code.bytecode.getbyte(0)
    assert_equal 9, code.bytecode.getbyte(1)
  end

  def test_ldc_i4_s_for_100
    code = compile_assignment(100)
    assert_equal CLR_LDC_I4_S, code.bytecode.getbyte(0)
    assert_equal 100, code.bytecode.getbyte(1)
  end

  def test_ldc_i4_s_for_127
    code = compile_assignment(127)
    assert_equal CLR_LDC_I4_S, code.bytecode.getbyte(0)
    assert_equal 127, code.bytecode.getbyte(1)
  end

  def test_ldc_i4_s_for_negative
    code = compile_assignment(-1)
    assert_equal CLR_LDC_I4_S, code.bytecode.getbyte(0)
    assert_equal 0xFF, code.bytecode.getbyte(1)
  end

  def test_ldc_i4_for_128
    code = compile_assignment(128)
    assert_equal CLR_LDC_I4, code.bytecode.getbyte(0)
    value_bytes = code.bytecode.byteslice(1, 4)
    assert_equal 128, value_bytes.unpack1("l<")
  end

  def test_ldc_i4_for_large_number
    code = compile_assignment(100_000)
    assert_equal CLR_LDC_I4, code.bytecode.getbyte(0)
    value_bytes = code.bytecode.byteslice(1, 4)
    assert_equal 100_000, value_bytes.unpack1("l<")
  end

  def test_ldc_i4_for_negative_129
    code = compile_assignment(-129)
    assert_equal CLR_LDC_I4, code.bytecode.getbyte(0)
    value_bytes = code.bytecode.byteslice(1, 4)
    assert_equal(-129, value_bytes.unpack1("l<"))
  end
end

class TestCLRLocalVariableEncoding < Minitest::Test
  include CodingAdventures::BytecodeCompiler

  def test_stloc_0
    program = P::Program.new(
      statements: [
        P::Assignment.new(
          target: P::Name.new(name: "x"),
          value: P::NumberLiteral.new(value: 1)
        )
      ]
    )
    code = CLRCompiler.new.compile(program)
    assert_equal CLR_STLOC_0, code.bytecode.getbyte(1)
  end

  def test_stloc_s_for_slot_4
    program = P::Program.new(
      statements: [
        P::Assignment.new(target: P::Name.new(name: "a"), value: P::NumberLiteral.new(value: 0)),
        P::Assignment.new(target: P::Name.new(name: "b"), value: P::NumberLiteral.new(value: 1)),
        P::Assignment.new(target: P::Name.new(name: "c"), value: P::NumberLiteral.new(value: 2)),
        P::Assignment.new(target: P::Name.new(name: "d"), value: P::NumberLiteral.new(value: 3)),
        P::Assignment.new(target: P::Name.new(name: "e"), value: P::NumberLiteral.new(value: 4))
      ]
    )
    code = CLRCompiler.new.compile(program)
    assert_equal CLR_STLOC_S, code.bytecode.getbyte(9)
    assert_equal 4, code.bytecode.getbyte(10)
  end

  def test_ldloc_0
    program = P::Program.new(
      statements: [
        P::Assignment.new(
          target: P::Name.new(name: "x"),
          value: P::NumberLiteral.new(value: 1)
        ),
        P::Assignment.new(
          target: P::Name.new(name: "y"),
          value: P::Name.new(name: "x")
        )
      ]
    )
    code = CLRCompiler.new.compile(program)
    assert_equal CLR_LDLOC_0, code.bytecode.getbyte(2)
  end

  def test_ldloc_s_for_slot_4
    program = P::Program.new(
      statements: [
        P::Assignment.new(target: P::Name.new(name: "a"), value: P::NumberLiteral.new(value: 0)),
        P::Assignment.new(target: P::Name.new(name: "b"), value: P::NumberLiteral.new(value: 1)),
        P::Assignment.new(target: P::Name.new(name: "c"), value: P::NumberLiteral.new(value: 2)),
        P::Assignment.new(target: P::Name.new(name: "d"), value: P::NumberLiteral.new(value: 3)),
        P::Assignment.new(target: P::Name.new(name: "e"), value: P::NumberLiteral.new(value: 4)),
        P::Assignment.new(target: P::Name.new(name: "f"), value: P::Name.new(name: "e"))
      ]
    )
    code = CLRCompiler.new.compile(program)
    assert_equal CLR_LDLOC_S, code.bytecode.getbyte(11)
    assert_equal 4, code.bytecode.getbyte(12)
  end
end

class TestCLRArithmeticOps < Minitest::Test
  include CodingAdventures::BytecodeCompiler

  def compile_binop(op, left_val, right_val)
    program = P::Program.new(
      statements: [
        P::Assignment.new(
          target: P::Name.new(name: "x"),
          value: P::BinaryOp.new(
            left: P::NumberLiteral.new(value: left_val),
            op: op,
            right: P::NumberLiteral.new(value: right_val)
          )
        )
      ]
    )
    CLRCompiler.new.compile(program)
  end

  def test_add
    code = compile_binop("+", 1, 2)
    assert_includes code.bytecode.bytes, CLR_ADD
  end

  def test_sub
    code = compile_binop("-", 5, 3)
    assert_includes code.bytecode.bytes, CLR_SUB
  end

  def test_mul
    code = compile_binop("*", 4, 3)
    assert_includes code.bytecode.bytes, CLR_MUL
  end

  def test_div
    code = compile_binop("/", 10, 2)
    assert_includes code.bytecode.bytes, CLR_DIV
  end
end

class TestCLREndToEnd < Minitest::Test
  include CodingAdventures::BytecodeCompiler

  def test_x_equals_1_plus_2
    program = P::Program.new(
      statements: [
        P::Assignment.new(
          target: P::Name.new(name: "x"),
          value: P::BinaryOp.new(
            left: P::NumberLiteral.new(value: 1),
            op: "+",
            right: P::NumberLiteral.new(value: 2)
          )
        )
      ]
    )
    code = CLRCompiler.new.compile(program)

    expected = [CLR_LDC_I4_1, CLR_LDC_I4_2, CLR_ADD, CLR_STLOC_0, CLR_RET].pack("C*").b
    assert_equal expected, code.bytecode
  end

  def test_empty_program
    program = P::Program.new(statements: [])
    code = CLRCompiler.new.compile(program)
    assert_equal [CLR_RET].pack("C*").b, code.bytecode
    assert_equal 0, code.num_locals
  end

  def test_ends_with_ret
    program = P::Program.new(
      statements: [
        P::Assignment.new(
          target: P::Name.new(name: "x"),
          value: P::NumberLiteral.new(value: 42)
        )
      ]
    )
    code = CLRCompiler.new.compile(program)
    assert_equal CLR_RET, code.bytecode.getbyte(code.bytecode.bytesize - 1)
  end

  def test_expression_statement_emits_pop
    program = P::Program.new(
      statements: [
        P::BinaryOp.new(
          left: P::NumberLiteral.new(value: 1),
          op: "+",
          right: P::NumberLiteral.new(value: 2)
        )
      ]
    )
    code = CLRCompiler.new.compile(program)
    expected = [CLR_LDC_I4_1, CLR_LDC_I4_2, CLR_ADD, CLR_POP, CLR_RET].pack("C*").b
    assert_equal expected, code.bytecode
  end
end

class TestCLRErrorHandling < Minitest::Test
  include CodingAdventures::BytecodeCompiler

  def test_unknown_expression_raises_type_error
    compiler = CLRCompiler.new
    assert_raises(TypeError) { compiler.compile_expression("not_an_ast_node") }
  end

  def test_string_literal_raises_type_error
    program = P::Program.new(
      statements: [
        P::Assignment.new(
          target: P::Name.new(name: "x"),
          value: P::StringLiteral.new(value: "hello")
        )
      ]
    )
    assert_raises(TypeError) { CLRCompiler.new.compile(program) }
  end
end

class TestCLRLocalNames < Minitest::Test
  include CodingAdventures::BytecodeCompiler

  def test_single_variable
    program = P::Program.new(
      statements: [
        P::Assignment.new(
          target: P::Name.new(name: "x"),
          value: P::NumberLiteral.new(value: 1)
        )
      ]
    )
    code = CLRCompiler.new.compile(program)
    assert_equal ["x"], code.local_names
    assert_equal 1, code.num_locals
  end

  def test_reassignment_reuses_slot
    program = P::Program.new(
      statements: [
        P::Assignment.new(
          target: P::Name.new(name: "x"),
          value: P::NumberLiteral.new(value: 1)
        ),
        P::Assignment.new(
          target: P::Name.new(name: "x"),
          value: P::NumberLiteral.new(value: 2)
        )
      ]
    )
    code = CLRCompiler.new.compile(program)
    assert_equal ["x"], code.local_names
    assert_equal 1, code.num_locals
  end
end
