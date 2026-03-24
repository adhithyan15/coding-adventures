# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_wasm_opcodes"

class TestWasmOpcodes < Minitest::Test
  M = CodingAdventures::WasmOpcodes

  # ── Version ────────────────────────────────────────────────────────────────

  def test_version_exists
    refute_nil M::VERSION
    assert_equal "0.1.0", M::VERSION
  end

  # ── Table completeness ─────────────────────────────────────────────────────

  def test_opcode_count_at_least_172
    # WASM 1.0 defines exactly 172 single-byte opcodes in 0x00–0xBF.
    # The bytes 0x06–0x0A, 0x12–0x19, 0x1C–0x1F, and 0x25–0x27 are reserved.
    assert M::OPCODES.size >= 172,
      "Expected at least 172 opcodes, got #{M::OPCODES.size}"
  end

  def test_opcodes_and_opcodes_by_name_same_size
    assert_equal M::OPCODES.size, M::OPCODES_BY_NAME.size
  end

  # ── Byte lookup ────────────────────────────────────────────────────────────

  def test_get_opcode_i32_add
    info = M.get_opcode(0x6A)
    refute_nil info
    assert_equal "i32.add", info.name
    assert_equal 0x6A, info.opcode
    assert_equal "numeric_i32", info.category
  end

  def test_get_opcode_unknown_byte_returns_nil
    assert_nil M.get_opcode(0xFF)
  end

  def test_get_opcode_reserved_gap_returns_nil
    assert_nil M.get_opcode(0x06)
  end

  # ── Name lookup ────────────────────────────────────────────────────────────

  def test_get_opcode_by_name_i32_add
    info = M.get_opcode_by_name("i32.add")
    refute_nil info
    assert_equal 0x6A, info.opcode
    assert_equal 2, info.stack_pop
    assert_equal 1, info.stack_push
  end

  def test_get_opcode_by_name_unknown_returns_nil
    assert_nil M.get_opcode_by_name("i32.foo")
    assert_nil M.get_opcode_by_name("")
  end

  # ── Stack effects ──────────────────────────────────────────────────────────

  def test_i32_add_stack_pop_push
    info = M.get_opcode(0x6A)
    assert_equal 2, info.stack_pop
    assert_equal 1, info.stack_push
  end

  def test_i32_const_stack
    info = M.get_opcode_by_name("i32.const")
    assert_equal 0, info.stack_pop
    assert_equal 1, info.stack_push
  end

  def test_drop_stack
    info = M.get_opcode_by_name("drop")
    assert_equal 1, info.stack_pop
    assert_equal 0, info.stack_push
  end

  def test_select_stack
    info = M.get_opcode_by_name("select")
    assert_equal 3, info.stack_pop
    assert_equal 1, info.stack_push
  end

  def test_nop_stack
    info = M.get_opcode_by_name("nop")
    assert_equal 0, info.stack_pop
    assert_equal 0, info.stack_push
  end

  def test_local_tee_peek_and_store
    info = M.get_opcode_by_name("local.tee")
    assert_equal 1, info.stack_pop
    assert_equal 1, info.stack_push
  end

  def test_memory_grow_stack
    info = M.get_opcode_by_name("memory.grow")
    assert_equal 1, info.stack_pop
    assert_equal 1, info.stack_push
  end

  def test_i32_store_stack
    info = M.get_opcode_by_name("i32.store")
    assert_equal 2, info.stack_pop
    assert_equal 0, info.stack_push
  end

  # ── Immediates ─────────────────────────────────────────────────────────────

  def test_i32_const_immediates
    info = M.get_opcode_by_name("i32.const")
    assert_equal ["i32"], info.immediates
  end

  def test_i64_const_immediates
    assert_equal ["i64"], M.get_opcode_by_name("i64.const").immediates
  end

  def test_f32_const_immediates
    assert_equal ["f32"], M.get_opcode_by_name("f32.const").immediates
  end

  def test_f64_const_immediates
    assert_equal ["f64"], M.get_opcode_by_name("f64.const").immediates
  end

  def test_i32_load_memarg
    assert_equal ["memarg"], M.get_opcode_by_name("i32.load").immediates
  end

  def test_i32_store_memarg
    assert_equal ["memarg"], M.get_opcode_by_name("i32.store").immediates
  end

  def test_block_blocktype
    assert_equal ["blocktype"], M.get_opcode_by_name("block").immediates
  end

  def test_loop_blocktype
    assert_equal ["blocktype"], M.get_opcode_by_name("loop").immediates
  end

  def test_if_blocktype
    assert_equal ["blocktype"], M.get_opcode_by_name("if").immediates
  end

  def test_call_indirect_immediates
    assert_equal ["typeidx", "tableidx"], M.get_opcode_by_name("call_indirect").immediates
  end

  def test_call_funcidx
    assert_equal ["funcidx"], M.get_opcode_by_name("call").immediates
  end

  def test_memory_size_memidx
    assert_equal ["memidx"], M.get_opcode_by_name("memory.size").immediates
  end

  def test_nop_no_immediates
    assert_equal [], M.get_opcode_by_name("nop").immediates
  end

  def test_i32_add_no_immediates
    assert_equal [], M.get_opcode_by_name("i32.add").immediates
  end

  def test_br_table_vec_labelidx
    assert_equal ["vec_labelidx"], M.get_opcode_by_name("br_table").immediates
  end

  # ── Data integrity ─────────────────────────────────────────────────────────

  def test_all_opcodes_have_nonempty_name
    M::OPCODES.each_value do |info|
      assert info.name.length > 0, "Opcode 0x#{info.opcode.to_s(16)} has empty name"
    end
  end

  def test_all_opcode_bytes_are_unique
    bytes = M::OPCODES.keys
    assert_equal bytes.uniq.size, bytes.size
  end

  def test_all_opcode_names_are_unique
    names = M::OPCODES_BY_NAME.keys
    assert_equal names.uniq.size, names.size
  end

  # ── Specific instruction checks ────────────────────────────────────────────

  def test_unreachable_is_0x00_control
    info = M.get_opcode(0x00)
    assert_equal "unreachable", info.name
    assert_equal "control", info.category
  end

  def test_f64_reinterpret_i64_conversion
    info = M.get_opcode(0xBF)
    assert_equal "f64.reinterpret_i64", info.name
    assert_equal "conversion", info.category
    assert_equal [], info.immediates
    assert_equal 1, info.stack_pop
    assert_equal 1, info.stack_push
  end

  def test_i64_add_category
    assert_equal "numeric_i64", M.get_opcode_by_name("i64.add").category
  end

  def test_f32_sqrt_category
    assert_equal "numeric_f32", M.get_opcode_by_name("f32.sqrt").category
  end

  def test_f64_sqrt_category
    assert_equal "numeric_f64", M.get_opcode_by_name("f64.sqrt").category
  end

  # ── Map consistency ────────────────────────────────────────────────────────

  def test_opcodes_and_opcodes_by_name_consistent
    M::OPCODES.each_value do |info|
      by_name = M::OPCODES_BY_NAME[info.name]
      refute_nil by_name
      assert_equal info.opcode, by_name.opcode
    end
  end

  def test_get_opcode_and_hash_return_same_object
    a = M.get_opcode(0x6A)
    b = M::OPCODES[0x6A]
    assert_same a, b
  end
end
