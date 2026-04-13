# frozen_string_literal: true

require_relative "test_helper"

# ==========================================================================
# Tests for IrParser — canonical text → IrProgram
# ==========================================================================
#
# Tests cover:
#   - Version parsing
#   - Data declaration parsing
#   - Entry point parsing
#   - Label definition parsing
#   - Instruction parsing (register, immediate, and label operands)
#   - COMMENT instruction parsing
#   - Instruction ID parsing from "; #N" suffix
#   - Error cases (unknown opcodes, malformed directives)
#   - Print/parse roundtrip
# ==========================================================================

class TestIrParser < Minitest::Test
  include CodingAdventures::CompilerIr

  # A minimal complete IR text for smoke tests
  MINIMAL_IR = <<~IR
    .version 1

    .data tape 30000 0

    .entry _start

    _start:
      LOAD_ADDR   v0, tape  ; #0
      LOAD_IMM    v1, 0  ; #1
      HALT               ; #2
  IR

  def test_parses_version
    prog = IrParser.parse(MINIMAL_IR)
    assert_equal 1, prog.version
  end

  def test_parses_entry
    prog = IrParser.parse(MINIMAL_IR)
    assert_equal "_start", prog.entry_label
  end

  def test_parses_data_decl
    prog = IrParser.parse(MINIMAL_IR)
    assert_equal 1, prog.data.length
    assert_equal "tape", prog.data[0].label
    assert_equal 30_000, prog.data[0].size
    assert_equal 0, prog.data[0].init
  end

  def test_parses_label_instruction
    prog = IrParser.parse(MINIMAL_IR)
    label_instr = prog.instructions.find { |i| i.opcode == IrOp::LABEL }
    refute_nil label_instr
    assert_equal IrLabel.new("_start"), label_instr.operands[0]
  end

  def test_parses_halt
    prog = IrParser.parse(MINIMAL_IR)
    halt_instr = prog.instructions.find { |i| i.opcode == IrOp::HALT }
    refute_nil halt_instr
    assert_equal 2, halt_instr.id
  end

  def test_parses_load_addr_with_register_and_label
    prog = IrParser.parse(MINIMAL_IR)
    la = prog.instructions.find { |i| i.opcode == IrOp::LOAD_ADDR }
    refute_nil la
    assert_equal IrRegister.new(0), la.operands[0]
    assert_equal IrLabel.new("tape"), la.operands[1]
    assert_equal 0, la.id
  end

  def test_parses_load_imm_with_immediate
    prog = IrParser.parse(MINIMAL_IR)
    li = prog.instructions.find { |i| i.opcode == IrOp::LOAD_IMM }
    refute_nil li
    assert_equal IrRegister.new(1), li.operands[0]
    assert_equal IrImmediate.new(0), li.operands[1]
    assert_equal 1, li.id
  end

  def test_parses_comment_instruction
    ir = ".version 1\n.entry e\n  ; a comment here\n"
    prog = IrParser.parse(ir)
    comment = prog.instructions.find { |i| i.opcode == IrOp::COMMENT }
    refute_nil comment
    assert_equal "a comment here", comment.operands[0].to_s
  end

  def test_ignores_blank_lines
    ir = ".version 1\n\n\n.entry e\n"
    prog = IrParser.parse(ir)
    assert_equal "e", prog.entry_label
  end

  def test_negative_immediate
    ir = ".version 1\n.entry e\n  ADD_IMM v1, v1, -1  ; #5\n"
    prog = IrParser.parse(ir)
    add_instr = prog.instructions.first
    assert_equal IrOp::ADD_IMM, add_instr.opcode
    assert_equal IrImmediate.new(-1), add_instr.operands[2]
    assert_equal 5, add_instr.id
  end

  def test_error_unknown_opcode
    ir = ".version 1\n.entry e\n  BOGUS_OP v0  ; #0\n"
    assert_raises(RuntimeError) { IrParser.parse(ir) }
  end

  def test_error_bad_version_directive
    ir = ".version\n.entry e\n"
    assert_raises(RuntimeError) { IrParser.parse(ir) }
  end

  def test_error_bad_entry_directive
    ir = ".version 1\n.entry\n"
    assert_raises(RuntimeError) { IrParser.parse(ir) }
  end

  def test_error_bad_data_directive
    ir = ".version 1\n.data tape\n.entry e\n"
    assert_raises(RuntimeError) { IrParser.parse(ir) }
  end

  # ── Print/Parse roundtrip ─────────────────────────────────────────────────

  def test_roundtrip_minimal
    prog = IrParser.parse(MINIMAL_IR)
    text = IrPrinter.print(prog)
    prog2 = IrParser.parse(text)
    assert_equal prog.version, prog2.version
    assert_equal prog.entry_label, prog2.entry_label
    assert_equal prog.instructions.length, prog2.instructions.length
  end

  def test_roundtrip_instruction_count
    prog = IrProgram.new("_start")
    gen = IDGenerator.new
    prog.add_data(IrDataDecl.new("tape", 30_000, 0))
    prog.add_instruction(IrInstruction.new(IrOp::LABEL, [IrLabel.new("_start")], -1))
    prog.add_instruction(IrInstruction.new(IrOp::LOAD_ADDR, [IrRegister.new(0), IrLabel.new("tape")], gen.next))
    prog.add_instruction(IrInstruction.new(IrOp::LOAD_IMM, [IrRegister.new(1), IrImmediate.new(0)], gen.next))
    prog.add_instruction(IrInstruction.new(IrOp::HALT, [], gen.next))

    text = IrPrinter.print(prog)
    parsed = IrParser.parse(text)

    assert_equal prog.instructions.length, parsed.instructions.length
  end

  def test_roundtrip_all_opcodes_used_in_brainfuck
    # A typical Brainfuck-compiled program uses these opcodes
    opcodes_used = [
      IrOp::LOAD_ADDR, IrOp::LOAD_IMM, IrOp::LOAD_BYTE, IrOp::STORE_BYTE,
      IrOp::ADD_IMM, IrOp::AND_IMM, IrOp::BRANCH_Z, IrOp::JUMP,
      IrOp::SYSCALL, IrOp::HALT
    ]
    prog = IrProgram.new("_start")
    gen = IDGenerator.new
    prog.add_instruction(IrInstruction.new(IrOp::LABEL, [IrLabel.new("_start")], -1))
    opcodes_used.each do |op|
      prog.add_instruction(IrInstruction.new(op, [], gen.next))
    end

    text = IrPrinter.print(prog)
    parsed = IrParser.parse(text)
    assert_equal prog.instructions.length, parsed.instructions.length
  end
end
