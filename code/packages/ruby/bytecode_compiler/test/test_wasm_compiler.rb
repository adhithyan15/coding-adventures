# frozen_string_literal: true

require_relative "test_helper"

P = CodingAdventures::Parser unless defined?(P)

# Comprehensive tests for the WASM Bytecode Compiler.
#
# WASM uses uniform encoding (no short forms), so tests verify:
# i32.const with 4-byte LE, local.get/local.set, arithmetic opcodes,
# no-pop for expression statements, and end instruction.

class TestWASMNumberEncoding < Minitest::Test
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
    WASMCompiler.new.compile(program)
  end

  def i32_const_bytes(value)
    [WASM_I32_CONST].pack("C") + [value].pack("l<")
  end

  def test_i32_const_0
    code = compile_assignment(0)
    assert_equal WASM_I32_CONST, code.bytecode.getbyte(0)
    assert_equal 0, code.bytecode.byteslice(1, 4).unpack1("l<")
  end

  def test_i32_const_1
    code = compile_assignment(1)
    assert_equal WASM_I32_CONST, code.bytecode.getbyte(0)
    assert_equal 1, code.bytecode.byteslice(1, 4).unpack1("l<")
  end

  def test_i32_const_42
    code = compile_assignment(42)
    assert_equal i32_const_bytes(42), code.bytecode.byteslice(0, 5)
  end

  def test_i32_const_large
    code = compile_assignment(100_000)
    assert_equal i32_const_bytes(100_000), code.bytecode.byteslice(0, 5)
  end

  def test_i32_const_negative
    code = compile_assignment(-1)
    assert_equal WASM_I32_CONST, code.bytecode.getbyte(0)
    assert_equal(-1, code.bytecode.byteslice(1, 4).unpack1("l<"))
  end
end

class TestWASMLocalVariableEncoding < Minitest::Test
  include CodingAdventures::BytecodeCompiler

  def test_local_set_0
    program = P::Program.new(
      statements: [
        P::Assignment.new(
          target: P::Name.new(name: "x"),
          value: P::NumberLiteral.new(value: 1)
        )
      ]
    )
    code = WASMCompiler.new.compile(program)
    # i32.const 1 (5 bytes), local.set (1 byte), 0 (1 byte), end (1 byte)
    assert_equal WASM_LOCAL_SET, code.bytecode.getbyte(5)
    assert_equal 0, code.bytecode.getbyte(6)
  end

  def test_local_set_1
    program = P::Program.new(
      statements: [
        P::Assignment.new(
          target: P::Name.new(name: "x"),
          value: P::NumberLiteral.new(value: 1)
        ),
        P::Assignment.new(
          target: P::Name.new(name: "y"),
          value: P::NumberLiteral.new(value: 2)
        )
      ]
    )
    code = WASMCompiler.new.compile(program)
    # First assignment: 5 + 2 = 7 bytes, then i32.const (5 bytes), local.set
    assert_equal WASM_LOCAL_SET, code.bytecode.getbyte(12)
    assert_equal 1, code.bytecode.getbyte(13)
  end

  def test_local_get_0
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
    code = WASMCompiler.new.compile(program)
    assert_equal WASM_LOCAL_GET, code.bytecode.getbyte(7)
    assert_equal 0, code.bytecode.getbyte(8)
  end

  def test_many_locals
    program = P::Program.new(
      statements: [
        P::Assignment.new(target: P::Name.new(name: "a"), value: P::NumberLiteral.new(value: 0)),
        P::Assignment.new(target: P::Name.new(name: "b"), value: P::NumberLiteral.new(value: 1)),
        P::Assignment.new(target: P::Name.new(name: "c"), value: P::NumberLiteral.new(value: 2)),
        P::Assignment.new(target: P::Name.new(name: "d"), value: P::NumberLiteral.new(value: 3)),
        P::Assignment.new(target: P::Name.new(name: "e"), value: P::NumberLiteral.new(value: 4))
      ]
    )
    code = WASMCompiler.new.compile(program)
    # 5th variable at slot 4: 4 * 7 = 28 bytes, then i32.const(5), local.set
    assert_equal WASM_LOCAL_SET, code.bytecode.getbyte(33)
    assert_equal 4, code.bytecode.getbyte(34)
  end
end

class TestWASMArithmeticOps < Minitest::Test
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
    WASMCompiler.new.compile(program)
  end

  def test_i32_add
    code = compile_binop("+", 1, 2)
    assert_includes code.bytecode.bytes, WASM_I32_ADD
  end

  def test_i32_sub
    code = compile_binop("-", 5, 3)
    assert_includes code.bytecode.bytes, WASM_I32_SUB
  end

  def test_i32_mul
    code = compile_binop("*", 4, 3)
    assert_includes code.bytecode.bytes, WASM_I32_MUL
  end

  def test_i32_div_s
    code = compile_binop("/", 10, 2)
    assert_includes code.bytecode.bytes, WASM_I32_DIV_S
  end
end

class TestWASMEndToEnd < Minitest::Test
  include CodingAdventures::BytecodeCompiler

  def i32_const_bytes(value)
    [WASM_I32_CONST].pack("C") + [value].pack("l<")
  end

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
    code = WASMCompiler.new.compile(program)

    expected = i32_const_bytes(1) + i32_const_bytes(2) +
      [WASM_I32_ADD].pack("C") + [WASM_LOCAL_SET, 0].pack("CC") + [WASM_END].pack("C")
    assert_equal expected, code.bytecode
  end

  def test_no_pop_for_expression_statement
    program = P::Program.new(
      statements: [
        P::BinaryOp.new(
          left: P::NumberLiteral.new(value: 1),
          op: "+",
          right: P::NumberLiteral.new(value: 2)
        )
      ]
    )
    code = WASMCompiler.new.compile(program)

    expected = i32_const_bytes(1) + i32_const_bytes(2) +
      [WASM_I32_ADD].pack("C") + [WASM_END].pack("C")
    assert_equal expected, code.bytecode
  end

  def test_empty_program
    program = P::Program.new(statements: [])
    code = WASMCompiler.new.compile(program)
    assert_equal [WASM_END].pack("C").b, code.bytecode
    assert_equal 0, code.num_locals
    assert_equal [], code.local_names
  end

  def test_ends_with_end
    program = P::Program.new(
      statements: [
        P::Assignment.new(
          target: P::Name.new(name: "x"),
          value: P::NumberLiteral.new(value: 42)
        )
      ]
    )
    code = WASMCompiler.new.compile(program)
    assert_equal WASM_END, code.bytecode.getbyte(code.bytecode.bytesize - 1)
  end

  def test_variable_load_and_store
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
    code = WASMCompiler.new.compile(program)

    expected = i32_const_bytes(1) + [WASM_LOCAL_SET, 0].pack("CC") +
      [WASM_LOCAL_GET, 0].pack("CC") + [WASM_LOCAL_SET, 1].pack("CC") +
      [WASM_END].pack("C")
    assert_equal expected, code.bytecode
  end
end

class TestWASMLocalNames < Minitest::Test
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
    code = WASMCompiler.new.compile(program)
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
    code = WASMCompiler.new.compile(program)
    assert_equal ["x"], code.local_names
    assert_equal 1, code.num_locals
  end
end

class TestWASMReturnType < Minitest::Test
  include CodingAdventures::BytecodeCompiler

  def test_returns_wasm_code_object
    program = P::Program.new(
      statements: [P::NumberLiteral.new(value: 1)]
    )
    code = WASMCompiler.new.compile(program)
    assert_instance_of WASMCodeObject, code
  end

  def test_bytecode_is_string
    program = P::Program.new(
      statements: [P::NumberLiteral.new(value: 1)]
    )
    code = WASMCompiler.new.compile(program)
    assert_instance_of String, code.bytecode
    assert_predicate code.bytecode, :frozen?
  end
end

class TestWASMErrorHandling < Minitest::Test
  include CodingAdventures::BytecodeCompiler

  def test_unknown_expression_raises_type_error
    compiler = WASMCompiler.new
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
    assert_raises(TypeError) { WASMCompiler.new.compile(program) }
  end
end
