# frozen_string_literal: true

# ==========================================================================
# Tests for GenericCompiler — the Pluggable AST-to-Bytecode Compiler
# ==========================================================================
#
# These tests verify that GenericCompiler dispatches to registered rule
# handlers, manages constant/name pools, handles jump patching, supports
# scopes, and compiles nested CodeObjects.
#
# We use a simple "toy AST" (OpenStruct nodes with rule_name and children)
# to test the compiler without depending on any real parser.
# ==========================================================================

require_relative "test_helper"
require "ostruct"

class TestGenericCompiler < Minitest::Test
  GC = CodingAdventures::BytecodeCompiler::GenericCompiler
  Inst = CodingAdventures::VirtualMachine::Instruction
  CO = CodingAdventures::VirtualMachine::CodeObject

  # -- Toy opcode constants --
  OP_LOAD_CONST = 0x01
  OP_STORE_NAME = 0x02
  OP_ADD        = 0x20
  OP_JUMP       = 0x30
  OP_JUMP_FALSE = 0x31
  OP_HALT       = 0xFF

  # Helper: create a toy AST node
  def make_node(rule_name, children = [])
    OpenStruct.new(rule_name: rule_name, children: children)
  end

  # Helper: create a toy token (leaf node)
  def make_token(type, value)
    OpenStruct.new(type: type, value: value)
  end

  # =====================================================================
  # Plugin registration and dispatch
  # =====================================================================

  def test_register_and_dispatch_rule
    compiler = GC.new
    called = false
    compiler.register_rule("test_rule", ->(c, node) { called = true })

    node = make_node("test_rule")
    compiler.compile_node(node)
    assert called
  end

  def test_pass_through_single_child
    compiler = GC.new
    inner_called = false
    compiler.register_rule("inner", ->(c, node) { inner_called = true })

    # Outer has one child (inner) and no handler → passes through
    inner = make_node("inner")
    outer = make_node("wrapper", [inner])
    compiler.compile_node(outer)
    assert inner_called
  end

  def test_unhandled_multi_child_raises
    compiler = GC.new
    node = make_node("complex", [make_node("a"), make_node("b")])
    assert_raises(CodingAdventures::BytecodeCompiler::UnhandledRuleError) do
      compiler.compile_node(node)
    end
  end

  def test_token_pass_through
    compiler = GC.new
    # Tokens are no-ops by default (structural tokens like NEWLINE)
    token = make_token("NEWLINE", "\n")
    compiler.compile_node(token)
    assert_empty compiler.instructions
  end

  # =====================================================================
  # Instruction emission
  # =====================================================================

  def test_emit_instruction
    compiler = GC.new
    idx = compiler.emit(OP_LOAD_CONST, 0)
    assert_equal 0, idx
    assert_equal 1, compiler.instructions.length
    assert_equal OP_LOAD_CONST, compiler.instructions[0].opcode
    assert_equal 0, compiler.instructions[0].operand
  end

  def test_emit_no_operand
    compiler = GC.new
    idx = compiler.emit(OP_ADD)
    assert_equal 0, idx
    assert_nil compiler.instructions[0].operand
  end

  def test_emit_returns_sequential_indices
    compiler = GC.new
    assert_equal 0, compiler.emit(OP_LOAD_CONST, 0)
    assert_equal 1, compiler.emit(OP_LOAD_CONST, 1)
    assert_equal 2, compiler.emit(OP_ADD)
  end

  def test_current_offset
    compiler = GC.new
    assert_equal 0, compiler.current_offset
    compiler.emit(OP_LOAD_CONST, 0)
    assert_equal 1, compiler.current_offset
    compiler.emit(OP_ADD)
    assert_equal 2, compiler.current_offset
  end

  # =====================================================================
  # Jump patching
  # =====================================================================

  def test_emit_jump_placeholder
    compiler = GC.new
    idx = compiler.emit_jump(OP_JUMP_FALSE)
    assert_equal 0, compiler.instructions[idx].operand  # placeholder
  end

  def test_patch_jump_explicit_target
    compiler = GC.new
    idx = compiler.emit_jump(OP_JUMP_FALSE)
    compiler.emit(OP_LOAD_CONST, 0)
    compiler.emit(OP_ADD)
    compiler.patch_jump(idx, 5)
    assert_equal 5, compiler.instructions[idx].operand
  end

  def test_patch_jump_default_target
    compiler = GC.new
    idx = compiler.emit_jump(OP_JUMP_FALSE)
    compiler.emit(OP_LOAD_CONST, 0)
    compiler.emit(OP_ADD)
    compiler.patch_jump(idx)  # defaults to current_offset = 3
    assert_equal 3, compiler.instructions[idx].operand
  end

  def test_patch_preserves_opcode
    compiler = GC.new
    idx = compiler.emit_jump(OP_JUMP_FALSE)
    compiler.patch_jump(idx, 10)
    assert_equal OP_JUMP_FALSE, compiler.instructions[idx].opcode
  end

  # =====================================================================
  # Constant and name pool management
  # =====================================================================

  def test_add_constant
    compiler = GC.new
    idx = compiler.add_constant(42)
    assert_equal 0, idx
    assert_equal [42], compiler.constants
  end

  def test_add_constant_deduplicates
    compiler = GC.new
    idx1 = compiler.add_constant(42)
    idx2 = compiler.add_constant(42)
    assert_equal idx1, idx2
    assert_equal [42], compiler.constants
  end

  def test_add_multiple_constants
    compiler = GC.new
    assert_equal 0, compiler.add_constant(1)
    assert_equal 1, compiler.add_constant(2)
    assert_equal 2, compiler.add_constant(3)
    assert_equal [1, 2, 3], compiler.constants
  end

  def test_add_name
    compiler = GC.new
    idx = compiler.add_name("x")
    assert_equal 0, idx
    assert_equal ["x"], compiler.names
  end

  def test_add_name_deduplicates
    compiler = GC.new
    idx1 = compiler.add_name("x")
    idx2 = compiler.add_name("x")
    assert_equal idx1, idx2
    assert_equal ["x"], compiler.names
  end

  def test_add_multiple_names
    compiler = GC.new
    assert_equal 0, compiler.add_name("x")
    assert_equal 1, compiler.add_name("y")
    assert_equal 2, compiler.add_name("z")
    assert_equal %w[x y z], compiler.names
  end

  # =====================================================================
  # Scope management
  # =====================================================================

  def test_enter_scope
    compiler = GC.new
    assert_nil compiler.scope
    scope = compiler.enter_scope
    refute_nil compiler.scope
    assert_equal scope, compiler.scope
  end

  def test_enter_scope_with_params
    compiler = GC.new
    scope = compiler.enter_scope(["a", "b"])
    assert_equal 0, scope.get_local("a")
    assert_equal 1, scope.get_local("b")
    assert_equal 2, scope.num_locals
  end

  def test_exit_scope
    compiler = GC.new
    compiler.enter_scope
    old = compiler.exit_scope
    assert_nil compiler.scope
    refute_nil old
  end

  def test_nested_scopes
    compiler = GC.new
    outer = compiler.enter_scope(["x"])
    inner = compiler.enter_scope(["y"])
    assert_equal inner, compiler.scope
    assert_equal outer, inner.parent
    compiler.exit_scope
    assert_equal outer, compiler.scope
  end

  def test_exit_scope_when_not_in_scope_raises
    compiler = GC.new
    assert_raises(CodingAdventures::BytecodeCompiler::CompilerError) do
      compiler.exit_scope
    end
  end

  # =====================================================================
  # CompilerScope
  # =====================================================================

  def test_scope_add_local
    scope = CodingAdventures::BytecodeCompiler::CompilerScope.new
    assert_equal 0, scope.add_local("x")
    assert_equal 1, scope.add_local("y")
  end

  def test_scope_add_local_deduplicates
    scope = CodingAdventures::BytecodeCompiler::CompilerScope.new
    idx1 = scope.add_local("x")
    idx2 = scope.add_local("x")
    assert_equal idx1, idx2
  end

  def test_scope_get_local
    scope = CodingAdventures::BytecodeCompiler::CompilerScope.new
    scope.add_local("x")
    assert_equal 0, scope.get_local("x")
    assert_nil scope.get_local("missing")
  end

  def test_scope_num_locals
    scope = CodingAdventures::BytecodeCompiler::CompilerScope.new
    assert_equal 0, scope.num_locals
    scope.add_local("x")
    scope.add_local("y")
    assert_equal 2, scope.num_locals
  end

  # =====================================================================
  # Nested code object compilation
  # =====================================================================

  def test_compile_nested
    compiler = GC.new
    # Set up a handler that emits instructions
    compiler.register_rule("body", ->(c, node) {
      c.emit(OP_LOAD_CONST, c.add_constant(42))
    })

    # Emit something in the outer context first
    compiler.emit(OP_LOAD_CONST, compiler.add_constant(1))

    # Compile a nested CodeObject
    nested_node = make_node("body")
    nested_code = compiler.compile_nested(nested_node)

    # Nested CodeObject should have its own instructions
    assert_equal 1, nested_code.instructions.length
    assert_equal [42], nested_code.constants

    # Outer context should be preserved
    assert_equal 1, compiler.instructions.length
    assert_equal [1], compiler.constants
  end

  # =====================================================================
  # Top-level compile
  # =====================================================================

  def test_compile_appends_halt
    compiler = GC.new
    compiler.register_rule("program", ->(c, node) {
      c.emit(OP_LOAD_CONST, c.add_constant(42))
    })

    code = compiler.compile(make_node("program"))
    assert_equal 2, code.instructions.length
    assert_equal OP_HALT, code.instructions.last.opcode
  end

  def test_compile_custom_halt_opcode
    compiler = GC.new
    compiler.register_rule("program", ->(c, node) {
      c.emit(OP_LOAD_CONST, 0)
    })

    code = compiler.compile(make_node("program"), halt_opcode: 0xFE)
    assert_equal 0xFE, code.instructions.last.opcode
  end

  def test_compile_returns_code_object
    compiler = GC.new
    compiler.register_rule("program", ->(c, node) {
      c.emit(OP_LOAD_CONST, c.add_constant(42))
    })

    code = compiler.compile(make_node("program"))
    assert_instance_of CodingAdventures::VirtualMachine::CodeObject, code
    assert_equal [42], code.constants
  end

  # =====================================================================
  # Integration: simple "expression" compilation
  # =====================================================================

  def test_compile_addition_expression
    # Simulate compiling "1 + 2" with toy handlers
    compiler = GC.new

    compiler.register_rule("expr", ->(c, node) {
      # Children: [number_node, "+", number_node]
      c.compile_node(node.children[0])
      c.compile_node(node.children[2])
      c.emit(OP_ADD)
    })

    compiler.register_rule("number", ->(c, node) {
      value = node.children[0].value.to_i
      c.emit(OP_LOAD_CONST, c.add_constant(value))
    })

    # AST: expr(number("1"), "+", number("2"))
    ast = make_node("expr", [
      make_node("number", [make_token("INT", "1")]),
      make_token("PLUS", "+"),
      make_node("number", [make_token("INT", "2")])
    ])

    code = compiler.compile(ast)

    # Should be: LOAD_CONST 0, LOAD_CONST 1, ADD, HALT
    assert_equal 4, code.instructions.length
    assert_equal [1, 2], code.constants
    assert_equal OP_LOAD_CONST, code.instructions[0].opcode
    assert_equal OP_LOAD_CONST, code.instructions[1].opcode
    assert_equal OP_ADD, code.instructions[2].opcode
    assert_equal OP_HALT, code.instructions[3].opcode
  end
end
