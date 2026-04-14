# frozen_string_literal: true

require_relative "test_helper"

# ==========================================================================
# Tests for IrOp — opcode constants, name lookup, and parse lookup
# ==========================================================================
#
# Every opcode must have a stable integer value, a canonical text name,
# and a working reverse parse. These tests verify the full round-trip:
#   IrOp::ADD_IMM → "ADD_IMM" → IrOp::ADD_IMM
# ==========================================================================

class TestIrOp < Minitest::Test
  include CodingAdventures::CompilerIr

  # ── Constant values ───────────────────────────────────────────────────────

  def test_load_imm_is_zero
    # The very first opcode must be 0, matching the Go iota order.
    assert_equal 0, IrOp::LOAD_IMM
  end

  def test_opcodes_are_sequential
    # All 25 opcodes must be 0..24 with no gaps.
    expected = (0..24).to_a
    actual = [
      IrOp::LOAD_IMM, IrOp::LOAD_ADDR,
      IrOp::LOAD_BYTE, IrOp::STORE_BYTE, IrOp::LOAD_WORD, IrOp::STORE_WORD,
      IrOp::ADD, IrOp::ADD_IMM, IrOp::SUB, IrOp::AND, IrOp::AND_IMM,
      IrOp::CMP_EQ, IrOp::CMP_NE, IrOp::CMP_LT, IrOp::CMP_GT,
      IrOp::LABEL, IrOp::JUMP, IrOp::BRANCH_Z, IrOp::BRANCH_NZ,
      IrOp::CALL, IrOp::RET,
      IrOp::SYSCALL, IrOp::HALT,
      IrOp::NOP, IrOp::COMMENT
    ].sort
    assert_equal expected, actual
  end

  def test_halt_is_22
    assert_equal 22, IrOp::HALT
  end

  def test_comment_is_24
    assert_equal 24, IrOp::COMMENT
  end

  def test_label_is_15
    assert_equal 15, IrOp::LABEL
  end

  # ── op_name ───────────────────────────────────────────────────────────────

  def test_op_name_load_imm
    assert_equal "LOAD_IMM", IrOp.op_name(IrOp::LOAD_IMM)
  end

  def test_op_name_add_imm
    assert_equal "ADD_IMM", IrOp.op_name(IrOp::ADD_IMM)
  end

  def test_op_name_halt
    assert_equal "HALT", IrOp.op_name(IrOp::HALT)
  end

  def test_op_name_branch_z
    assert_equal "BRANCH_Z", IrOp.op_name(IrOp::BRANCH_Z)
  end

  def test_op_name_branch_nz
    assert_equal "BRANCH_NZ", IrOp.op_name(IrOp::BRANCH_NZ)
  end

  def test_op_name_unknown
    assert_equal "UNKNOWN", IrOp.op_name(9999)
  end

  def test_op_name_all_opcodes_defined
    # Every opcode in 0..24 must have a non-UNKNOWN name.
    (0..24).each do |op|
      name = IrOp.op_name(op)
      refute_equal "UNKNOWN", name, "Opcode #{op} has no name"
    end
  end

  # ── parse_op ──────────────────────────────────────────────────────────────

  def test_parse_op_add_imm
    assert_equal IrOp::ADD_IMM, IrOp.parse_op("ADD_IMM")
  end

  def test_parse_op_load_byte
    assert_equal IrOp::LOAD_BYTE, IrOp.parse_op("LOAD_BYTE")
  end

  def test_parse_op_unknown_returns_nil
    assert_nil IrOp.parse_op("BOGUS")
  end

  def test_parse_op_empty_string_returns_nil
    assert_nil IrOp.parse_op("")
  end

  def test_roundtrip_all_opcodes
    # parse_op(op_name(op)) == op for every valid opcode
    (0..24).each do |op|
      name = IrOp.op_name(op)
      parsed = IrOp.parse_op(name)
      assert_equal op, parsed, "Roundtrip failed for opcode #{op} (#{name})"
    end
  end
end
