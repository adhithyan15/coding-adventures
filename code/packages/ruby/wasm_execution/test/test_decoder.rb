# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_wasm_execution"

# ==========================================================================
# Tests for the WASM Bytecode Decoder
# ==========================================================================
#
# The decoder bridges variable-length WASM bytecodes and the GenericVM's
# fixed-format Instruction objects. It also builds the control flow map
# that maps block/loop/if starts to their matching ends.
#
# Tests cover:
#   1. decode_function_body: various instruction sequences
#   2. build_control_flow_map: block/loop/if/else/end nesting
#   3. to_vm_instructions: conversion to GenericVM format
#   4. decode_signed_64: LEB128 for 64-bit signed integers
# ==========================================================================

class TestDecoder < Minitest::Test
  Decoder = CodingAdventures::WasmExecution::Decoder
  TrapError = CodingAdventures::WasmExecution::TrapError
  FunctionBody = CodingAdventures::WasmTypes::FunctionBody

  # Helper to create a FunctionBody from raw bytes.
  def make_body(bytes)
    FunctionBody.new([], bytes.pack("C*"))
  end

  # ── decode_function_body ───────────────────────────────────────────

  def test_decode_empty_body
    body = make_body([0x0B]) # just 'end'
    result = Decoder.decode_function_body(body)
    assert_equal 1, result.length
    assert_equal 0x0B, result[0].opcode
  end

  def test_decode_i32_const_42
    # i32.const 42, end
    body = make_body([0x41, 42, 0x0B])
    result = Decoder.decode_function_body(body)
    assert_equal 2, result.length
    assert_equal 0x41, result[0].opcode
    assert_equal 42, result[0].operand
    assert_equal 0x0B, result[1].opcode
  end

  def test_decode_local_get_0
    # local.get 0, end
    body = make_body([0x20, 0x00, 0x0B])
    result = Decoder.decode_function_body(body)
    assert_equal 2, result.length
    assert_equal 0x20, result[0].opcode
    assert_equal 0, result[0].operand
  end

  def test_decode_two_local_gets_and_mul
    # local.get 0, local.get 0, i32.mul, end
    body = make_body([0x20, 0x00, 0x20, 0x00, 0x6C, 0x0B])
    result = Decoder.decode_function_body(body)
    assert_equal 4, result.length
    assert_equal 0x20, result[0].opcode
    assert_equal 0x20, result[1].opcode
    assert_equal 0x6C, result[2].opcode
    assert_equal 0x0B, result[3].opcode
  end

  def test_decode_instruction_offsets
    # i32.const 10, i32.const 20, i32.add, end
    body = make_body([0x41, 10, 0x41, 20, 0x6A, 0x0B])
    result = Decoder.decode_function_body(body)
    assert_equal 0, result[0].offset
    assert_equal 2, result[0].size # opcode + LEB128
    assert_equal 2, result[1].offset
    assert_equal 2, result[1].size
    assert_equal 4, result[2].offset
    assert_equal 1, result[2].size
    assert_equal 5, result[3].offset
  end

  def test_decode_nop
    body = make_body([0x01, 0x0B])
    result = Decoder.decode_function_body(body)
    assert_equal 2, result.length
    assert_equal 0x01, result[0].opcode
    assert_nil result[0].operand
  end

  def test_decode_block_with_type
    # block (result i32), i32.const 1, end, end
    body = make_body([0x02, 0x7F, 0x41, 0x01, 0x0B, 0x0B])
    result = Decoder.decode_function_body(body)
    assert_equal 4, result.length
    assert_equal 0x02, result[0].opcode
    assert_equal 0x7F, result[0].operand # block type: i32
  end

  def test_decode_block_void
    # block (void), nop, end, end
    body = make_body([0x02, 0x40, 0x01, 0x0B, 0x0B])
    result = Decoder.decode_function_body(body)
    assert_equal 4, result.length
    assert_equal 0x40, result[0].operand # block type: void
  end

  # ── build_control_flow_map ─────────────────────────────────────────

  def test_control_flow_map_simple_block
    # block(void), nop, end, end
    # Decoded indices: 0=block, 1=nop, 2=end(block), 3=end(func)
    body = make_body([0x02, 0x40, 0x01, 0x0B, 0x0B])
    decoded = Decoder.decode_function_body(body)
    map = Decoder.build_control_flow_map(decoded)

    assert map.key?(0)
    assert_equal 2, map[0].end_pc
    assert_nil map[0].else_pc
  end

  def test_control_flow_map_nested_blocks
    # block(void), block(void), nop, end, end, end
    # Decoded indices: 0=block, 1=block, 2=nop, 3=end(inner), 4=end(outer), 5=end(func)
    body = make_body([0x02, 0x40, 0x02, 0x40, 0x01, 0x0B, 0x0B, 0x0B])
    decoded = Decoder.decode_function_body(body)
    map = Decoder.build_control_flow_map(decoded)

    # Inner block at index 1 ends at index 3
    assert map.key?(1)
    assert_equal 3, map[1].end_pc

    # Outer block at index 0 ends at index 4
    assert map.key?(0)
    assert_equal 4, map[0].end_pc
  end

  def test_control_flow_map_if_else
    # if(void), nop, else, nop, end, end
    # Decoded indices: 0=if, 1=nop, 2=else, 3=nop, 4=end(if), 5=end(func)
    body = make_body([0x04, 0x40, 0x01, 0x05, 0x01, 0x0B, 0x0B])
    decoded = Decoder.decode_function_body(body)
    map = Decoder.build_control_flow_map(decoded)

    assert map.key?(0)
    assert_equal 4, map[0].end_pc
    assert_equal 2, map[0].else_pc
  end

  def test_control_flow_map_if_without_else
    # if(void), nop, end, end
    # Decoded indices: 0=if, 1=nop, 2=end(if), 3=end(func)
    body = make_body([0x04, 0x40, 0x01, 0x0B, 0x0B])
    decoded = Decoder.decode_function_body(body)
    map = Decoder.build_control_flow_map(decoded)

    assert map.key?(0)
    assert_equal 2, map[0].end_pc
    assert_nil map[0].else_pc
  end

  def test_control_flow_map_loop
    # loop(void), nop, end, end
    # Decoded indices: 0=loop, 1=nop, 2=end(loop), 3=end(func)
    body = make_body([0x03, 0x40, 0x01, 0x0B, 0x0B])
    decoded = Decoder.decode_function_body(body)
    map = Decoder.build_control_flow_map(decoded)

    assert map.key?(0)
    assert_equal 2, map[0].end_pc
  end

  # ── to_vm_instructions ─────────────────────────────────────────────

  def test_to_vm_instructions_converts_format
    body = make_body([0x41, 42, 0x0B])
    decoded = Decoder.decode_function_body(body)
    vm_instrs = Decoder.to_vm_instructions(decoded)

    assert_equal 2, vm_instrs.length
    assert_kind_of CodingAdventures::VirtualMachine::Instruction, vm_instrs[0]
    assert_equal 0x41, vm_instrs[0].opcode
    assert_equal 42, vm_instrs[0].operand
  end

  # ── decode_signed_64 ───────────────────────────────────────────────

  def test_decode_signed_64_positive
    # 42 in signed LEB128 = [42]
    val, consumed = Decoder.decode_signed_64([42], 0)
    assert_equal 42, val
    assert_equal 1, consumed
  end

  def test_decode_signed_64_negative_one
    # -1 in signed LEB128 = [0x7F]
    val, consumed = Decoder.decode_signed_64([0x7F], 0)
    assert_equal(-1, val)
    assert_equal 1, consumed
  end

  def test_decode_signed_64_zero
    val, consumed = Decoder.decode_signed_64([0x00], 0)
    assert_equal 0, val
    assert_equal 1, consumed
  end

  def test_decode_signed_64_multi_byte
    # 128 in signed LEB128 = [0x80, 0x01]
    val, consumed = Decoder.decode_signed_64([0x80, 0x01], 0)
    assert_equal 128, val
    assert_equal 2, consumed
  end

  def test_decode_signed_64_negative_128
    # -128 in signed LEB128 = [0x80, 0x7F]
    val, consumed = Decoder.decode_signed_64([0x80, 0x7F], 0)
    assert_equal(-128, val)
    assert_equal 2, consumed
  end

  def test_decode_signed_64_unterminated_raises
    assert_raises(TrapError) { Decoder.decode_signed_64([], 0) }
  end

  # ── ConstExpr ──────────────────────────────────────────────────────

  def test_const_expr_i32
    # i32.const 42, end
    expr = [0x41, 42, 0x0B].pack("C*")
    result = CodingAdventures::WasmExecution::ConstExpr.evaluate(expr)
    assert_equal CodingAdventures::WasmTypes::VALUE_TYPE[:i32], result.type
    assert_equal 42, result.value
  end

  def test_const_expr_i64
    # i64.const 99, end
    # 99 in signed LEB128 needs 2 bytes because bit 6 is set in 99 (0b1100011):
    # [0xE3, 0x00] => low 7 bits = 0x63 = 99, continuation bit set;
    # next byte 0x00 => no more bits, sign bit clear => positive 99
    expr = [0x42, 0xE3, 0x00, 0x0B].pack("C*")
    result = CodingAdventures::WasmExecution::ConstExpr.evaluate(expr)
    assert_equal CodingAdventures::WasmTypes::VALUE_TYPE[:i64], result.type
    assert_equal 99, result.value
  end

  def test_const_expr_global_get
    # global.get 0, end
    expr = [0x23, 0x00, 0x0B].pack("C*")
    globals = [CodingAdventures::WasmExecution.i32(77)]
    result = CodingAdventures::WasmExecution::ConstExpr.evaluate(expr, globals)
    assert_equal 77, result.value
  end

  def test_const_expr_illegal_opcode_raises
    expr = [0xFF, 0x0B].pack("C*")
    assert_raises(TrapError) do
      CodingAdventures::WasmExecution::ConstExpr.evaluate(expr)
    end
  end

  def test_const_expr_missing_end_raises
    expr = [0x41, 42].pack("C*")
    assert_raises(TrapError) do
      CodingAdventures::WasmExecution::ConstExpr.evaluate(expr)
    end
  end

  def test_const_expr_empty_raises
    expr = [0x0B].pack("C*")
    assert_raises(TrapError) do
      CodingAdventures::WasmExecution::ConstExpr.evaluate(expr)
    end
  end

  # ── Table ──────────────────────────────────────────────────────────

  def test_table_get_and_set
    table = CodingAdventures::WasmExecution::Table.new(5)
    table.set(2, 42)
    assert_equal 42, table.get(2)
  end

  def test_table_uninitialized_returns_nil
    table = CodingAdventures::WasmExecution::Table.new(5)
    assert_nil table.get(0)
  end

  def test_table_oob_get_traps
    table = CodingAdventures::WasmExecution::Table.new(3)
    assert_raises(TrapError) { table.get(3) }
  end

  def test_table_oob_set_traps
    table = CodingAdventures::WasmExecution::Table.new(3)
    assert_raises(TrapError) { table.set(3, 0) }
  end

  def test_table_negative_index_traps
    table = CodingAdventures::WasmExecution::Table.new(3)
    assert_raises(TrapError) { table.get(-1) }
  end

  def test_table_size
    table = CodingAdventures::WasmExecution::Table.new(7)
    assert_equal 7, table.size
  end

  def test_table_grow
    table = CodingAdventures::WasmExecution::Table.new(2, 5)
    old = table.grow(2)
    assert_equal 2, old
    assert_equal 4, table.size
  end

  def test_table_grow_beyond_max
    table = CodingAdventures::WasmExecution::Table.new(2, 3)
    result = table.grow(2) # would be 4, exceeds max 3
    assert_equal(-1, result)
    assert_equal 2, table.size
  end

  # ── HostFunction ───────────────────────────────────────────────────

  def test_host_function_call
    func_type = CodingAdventures::WasmTypes::FuncType.new(
      [CodingAdventures::WasmTypes::VALUE_TYPE[:i32]],
      [CodingAdventures::WasmTypes::VALUE_TYPE[:i32]]
    )
    host_fn = CodingAdventures::WasmExecution::HostFunction.new(
      func_type: func_type,
      implementation: ->(args) { [CodingAdventures::WasmExecution.i32(args[0].value * 2)] }
    )

    result = host_fn.call([CodingAdventures::WasmExecution.i32(21)])
    assert_equal 1, result.length
    assert_equal 42, result[0].value
  end

  # ── HostInterface defaults ─────────────────────────────────────────

  def test_host_interface_defaults_return_nil
    host_class = Class.new do
      include CodingAdventures::WasmExecution::HostInterface
    end
    host = host_class.new
    assert_nil host.resolve_function("mod", "fn")
    assert_nil host.resolve_global("mod", "g")
    assert_nil host.resolve_memory("mod", "m")
    assert_nil host.resolve_table("mod", "t")
  end

  # ── TrapError ──────────────────────────────────────────────────────

  def test_trap_error_is_runtime_error
    assert TrapError < RuntimeError
  end

  def test_trap_error_message
    err = TrapError.new("divide by zero")
    assert_equal "divide by zero", err.message
  end
end
