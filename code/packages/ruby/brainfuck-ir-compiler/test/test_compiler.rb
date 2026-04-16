# frozen_string_literal: true

require_relative "test_helper"

# ==========================================================================
# Tests for BrainfuckIrCompiler — compilation of Brainfuck ASTs to IR
# ==========================================================================
#
# These tests mirror the Go compiler_test.go suite exactly, translated to
# Ruby minitest idioms. They verify:
#
#   - BuildConfig presets
#   - Empty program → prologue + HALT
#   - Each BF command emits correct IR opcodes
#   - Loops emit labels / BRANCH_Z / JUMP
#   - Source map entries are populated correctly
#   - Bounds checks emitted (or not) based on config
#   - Byte masking emitted (or not) based on config
#   - IR print/parse roundtrip works
#   - Error cases (bad AST, zero tape size)
# ==========================================================================

class TestBrainfuckIrCompiler < Minitest::Test
  IR  = CodingAdventures::CompilerIr
  BIC = CodingAdventures::BrainfuckIrCompiler

  # ── Helpers ──────────────────────────────────────────────────────────────

  # compile_source(source, config) → CompileResult
  # Parses and compiles a Brainfuck source string.
  def compile_source(source, config = BIC::BuildConfig.release_config)
    ast = CodingAdventures::Brainfuck::Parser.parse(source)
    BIC.compile(ast, "test.bf", config)
  end

  # count_opcode(program, opcode) → Integer
  # Counts instructions with the given opcode.
  def count_opcode(program, opcode)
    program.instructions.count { |i| i.opcode == opcode }
  end

  # has_label?(program, name) → Boolean
  # Returns true if the program contains a LABEL instruction with the given name.
  def has_label?(program, name)
    program.instructions.any? do |i|
      i.opcode == IR::IrOp::LABEL &&
        i.operands.first.is_a?(IR::IrLabel) &&
        i.operands.first.name == name
    end
  end

  # ── BuildConfig ──────────────────────────────────────────────────────────

  def test_debug_config_flags
    cfg = BIC::BuildConfig.debug_config
    assert cfg.insert_bounds_checks
    assert cfg.insert_debug_locs
    assert cfg.mask_byte_arithmetic
    assert_equal 30_000, cfg.tape_size
  end

  def test_release_config_flags
    cfg = BIC::BuildConfig.release_config
    refute cfg.insert_bounds_checks
    refute cfg.insert_debug_locs
    assert cfg.mask_byte_arithmetic
    assert_equal 30_000, cfg.tape_size
  end

  # ── Empty program ─────────────────────────────────────────────────────────

  def test_empty_program_has_start_label
    result = compile_source("")
    assert has_label?(result.program, "_start"), "expected _start label"
  end

  def test_empty_program_has_halt
    result = compile_source("")
    assert_equal 1, count_opcode(result.program, IR::IrOp::HALT)
  end

  def test_empty_program_version
    result = compile_source("")
    assert_equal 1, result.program.version
  end

  def test_empty_program_entry_label
    result = compile_source("")
    assert_equal "_start", result.program.entry_label
  end

  def test_empty_program_has_tape_data
    result = compile_source("")
    assert_equal 1, result.program.data.length
    assert_equal "tape", result.program.data[0].label
    assert_equal 30_000, result.program.data[0].size
    assert_equal 0, result.program.data[0].init
  end

  # ── INC (+) ──────────────────────────────────────────────────────────────

  def test_inc_emits_load_byte
    result = compile_source("+")
    assert count_opcode(result.program, IR::IrOp::LOAD_BYTE) >= 1
  end

  def test_inc_emits_store_byte
    result = compile_source("+")
    assert count_opcode(result.program, IR::IrOp::STORE_BYTE) >= 1
  end

  def test_inc_emits_and_imm_when_masking
    result = compile_source("+")
    assert count_opcode(result.program, IR::IrOp::AND_IMM) >= 1
  end

  def test_inc_no_and_imm_without_masking
    cfg = BIC::BuildConfig.release_config
    cfg = BIC::BuildConfig.new(
      insert_bounds_checks: false,
      insert_debug_locs: false,
      mask_byte_arithmetic: false,
      tape_size: 30_000
    )
    result = compile_source("+", cfg)
    assert_equal 0, count_opcode(result.program, IR::IrOp::AND_IMM)
  end

  # ── DEC (-) ──────────────────────────────────────────────────────────────

  def test_dec_emits_add_imm_minus_one
    result = compile_source("-")
    found = result.program.instructions.any? do |i|
      i.opcode == IR::IrOp::ADD_IMM &&
        i.operands.length >= 3 &&
        i.operands[2].is_a?(IR::IrImmediate) &&
        i.operands[2].value == -1
    end
    assert found, "expected ADD_IMM with -1 for DEC"
  end

  # ── RIGHT (>) ────────────────────────────────────────────────────────────

  def test_right_emits_add_imm_plus_one
    result = compile_source(">")
    found = result.program.instructions.any? do |i|
      i.opcode == IR::IrOp::ADD_IMM &&
        i.operands.length >= 3 &&
        i.operands[0].is_a?(IR::IrRegister) &&
        i.operands[0].index == 1 &&      # v1 = tape ptr
        i.operands[2].is_a?(IR::IrImmediate) &&
        i.operands[2].value == 1
    end
    assert found, "expected ADD_IMM v1, v1, 1 for RIGHT"
  end

  # ── LEFT (<) ─────────────────────────────────────────────────────────────

  def test_left_emits_add_imm_minus_one
    result = compile_source("<")
    found = result.program.instructions.any? do |i|
      i.opcode == IR::IrOp::ADD_IMM &&
        i.operands.length >= 3 &&
        i.operands[0].is_a?(IR::IrRegister) &&
        i.operands[0].index == 1 &&
        i.operands[2].is_a?(IR::IrImmediate) &&
        i.operands[2].value == -1
    end
    assert found, "expected ADD_IMM v1, v1, -1 for LEFT"
  end

  # ── OUTPUT (.) ───────────────────────────────────────────────────────────

  def test_output_emits_syscall_write
    result = compile_source(".")
    found_copy = result.program.instructions.any? do |i|
      i.opcode == IR::IrOp::ADD_IMM &&
        i.operands[0].is_a?(IR::IrRegister) &&
        i.operands[0].index == 4 &&
        i.operands[1].is_a?(IR::IrRegister) &&
        i.operands[1].index == 2 &&
        i.operands[2].is_a?(IR::IrImmediate) &&
        i.operands[2].value == 0
    end
    found = result.program.instructions.any? do |i|
      i.opcode == IR::IrOp::SYSCALL &&
        i.operands.first.is_a?(IR::IrImmediate) &&
        i.operands.first.value == 1  # SYSCALL_WRITE = 1
    end
    assert found_copy, "expected ADD_IMM copy into syscall arg register"
    assert found, "expected SYSCALL 1 (write) for OUTPUT"
  end

  # ── INPUT (,) ────────────────────────────────────────────────────────────

  def test_input_emits_syscall_read
    result = compile_source(",")
    found = result.program.instructions.any? do |i|
      i.opcode == IR::IrOp::SYSCALL &&
        i.operands.first.is_a?(IR::IrImmediate) &&
        i.operands.first.value == 2  # SYSCALL_READ = 2
    end
    assert found, "expected SYSCALL 2 (read) for INPUT"
  end

  # ── Loops ─────────────────────────────────────────────────────────────────

  def test_simple_loop_has_start_label
    result = compile_source("[-]")
    assert has_label?(result.program, "loop_0_start"), "expected loop_0_start"
  end

  def test_simple_loop_has_end_label
    result = compile_source("[-]")
    assert has_label?(result.program, "loop_0_end"), "expected loop_0_end"
  end

  def test_simple_loop_has_branch_z
    result = compile_source("[-]")
    assert count_opcode(result.program, IR::IrOp::BRANCH_Z) >= 1
  end

  def test_simple_loop_has_jump
    result = compile_source("[-]")
    assert count_opcode(result.program, IR::IrOp::JUMP) >= 1
  end

  def test_empty_loop
    result = compile_source("[]")
    assert has_label?(result.program, "loop_0_start")
    assert has_label?(result.program, "loop_0_end")
  end

  def test_nested_loops
    result = compile_source("[>[+<-]]")
    assert has_label?(result.program, "loop_0_start"), "expected loop_0_start"
    assert has_label?(result.program, "loop_1_start"), "expected loop_1_start"
  end

  # ── Bounds checks ─────────────────────────────────────────────────────────

  def test_debug_right_has_cmp_gt
    result = compile_source(">", BIC::BuildConfig.debug_config)
    assert count_opcode(result.program, IR::IrOp::CMP_GT) >= 1, "expected CMP_GT for right bounds"
  end

  def test_debug_right_has_branch_nz
    result = compile_source(">", BIC::BuildConfig.debug_config)
    assert count_opcode(result.program, IR::IrOp::BRANCH_NZ) >= 1
  end

  def test_debug_has_trap_oob_label
    result = compile_source(">", BIC::BuildConfig.debug_config)
    assert has_label?(result.program, "__trap_oob")
  end

  def test_debug_left_has_cmp_lt
    result = compile_source("<", BIC::BuildConfig.debug_config)
    assert count_opcode(result.program, IR::IrOp::CMP_LT) >= 1, "expected CMP_LT for left bounds"
  end

  def test_release_no_bounds_checks
    result = compile_source("><")
    assert_equal 0, count_opcode(result.program, IR::IrOp::CMP_GT)
    assert_equal 0, count_opcode(result.program, IR::IrOp::CMP_LT)
    refute has_label?(result.program, "__trap_oob")
  end

  # ── Source map ────────────────────────────────────────────────────────────

  def test_source_map_two_commands
    result = compile_source("+.")
    assert_equal 2, result.source_map.source_to_ast.entries.length,
                 "expected 2 SourceToAst entries for '+.'"
  end

  def test_source_map_column_one
    result = compile_source("+.")
    entry0 = result.source_map.source_to_ast.entries[0]
    assert_equal 1, entry0.pos.column, "expected '+' at column 1"
  end

  def test_source_map_column_two
    result = compile_source("+.")
    entry1 = result.source_map.source_to_ast.entries[1]
    assert_equal 2, entry1.pos.column, "expected '.' at column 2"
  end

  def test_source_map_file_name
    result = compile_source("+")
    result.source_map.source_to_ast.entries.each do |e|
      assert_equal "test.bf", e.pos.file
    end
  end

  def test_source_map_plus_four_ir_ids
    result = compile_source("+")
    assert_equal 1, result.source_map.ast_to_ir.entries.length
    entry = result.source_map.ast_to_ir.entries[0]
    # "+" produces LOAD_BYTE, ADD_IMM, AND_IMM, STORE_BYTE = 4 IR instructions
    assert_equal 4, entry.ir_ids.length,
                 "expected 4 IR IDs for '+', got #{entry.ir_ids.length}"
  end

  def test_source_map_loop_has_entry
    result = compile_source("[-]")
    assert result.source_map.source_to_ast.entries.length >= 2,
           "expected at least 2 SourceToAst entries for '[-]'"
  end

  # ── IR text output ────────────────────────────────────────────────────────

  def test_ir_is_printable
    result = compile_source("+.")
    text = IR::IrPrinter.print(result.program)
    assert_includes text, ".version 1"
    assert_includes text, ".data tape 30000 0"
    assert_includes text, ".entry _start"
    assert_includes text, "LOAD_BYTE"
    assert_includes text, "HALT"
  end

  def test_ir_roundtrip
    result = compile_source("++[-].")
    text = IR::IrPrinter.print(result.program)
    parsed = IR::IrParser.parse(text)
    assert_equal result.program.instructions.length, parsed.instructions.length,
                 "roundtrip: instruction count mismatch"
  end

  # ── Complex programs ──────────────────────────────────────────────────────

  def test_hello_world_subset
    # Set cell 0 to 72 ('H') and output it: ++++++++ [>+++++++++<-] >.
    source = "++++++++[>+++++++++<-]>."
    result = compile_source(source)
    assert has_label?(result.program, "loop_0_start")
    found_write = result.program.instructions.any? do |i|
      i.opcode == IR::IrOp::SYSCALL &&
        i.operands.first.is_a?(IR::IrImmediate) &&
        i.operands.first.value == 1
    end
    assert found_write, "expected SYSCALL 1 (write) in hello world fragment"
  end

  def test_cat_program
    result = compile_source(",[.,]")
    found_read  = result.program.instructions.any? { |i| i.opcode == IR::IrOp::SYSCALL && i.operands.first&.value == 2 }
    found_write = result.program.instructions.any? { |i| i.opcode == IR::IrOp::SYSCALL && i.operands.first&.value == 1 }
    assert found_read, "expected SYSCALL 2 (read) in cat program"
    assert found_write, "expected SYSCALL 1 (write) in cat program"
  end

  # ── Custom tape size ──────────────────────────────────────────────────────

  def test_custom_tape_size
    cfg = BIC::BuildConfig.new(
      insert_bounds_checks: false,
      insert_debug_locs: false,
      mask_byte_arithmetic: true,
      tape_size: 1000
    )
    result = compile_source("", cfg)
    assert_equal 1000, result.program.data[0].size
  end

  # ── Instruction ID uniqueness ─────────────────────────────────────────────

  def test_instruction_ids_are_unique
    result = compile_source("++[>+<-].")
    seen = {}
    result.program.instructions.each do |instr|
      next if instr.id == -1  # labels have -1

      if seen[instr.id]
        flunk "duplicate instruction ID: #{instr.id}"
      end
      seen[instr.id] = true
    end
  end

  # ── Error cases ───────────────────────────────────────────────────────────

  def test_error_wrong_ast_root
    fake_ast = CodingAdventures::Parser::ASTNode.new(
      rule_name: "not_a_program",
      children: []
    )
    err = assert_raises(RuntimeError) do
      BIC.compile(fake_ast, "test.bf", BIC::BuildConfig.release_config)
    end
    assert_match(/program/, err.message)
  end

  def test_error_zero_tape_size
    ast = CodingAdventures::Brainfuck::Parser.parse("")
    cfg = BIC::BuildConfig.new(
      insert_bounds_checks: false,
      insert_debug_locs: false,
      mask_byte_arithmetic: true,
      tape_size: 0
    )
    assert_raises(RuntimeError) { BIC.compile(ast, "test.bf", cfg) }
  end

  def test_error_negative_tape_size
    ast = CodingAdventures::Brainfuck::Parser.parse("")
    cfg = BIC::BuildConfig.new(
      insert_bounds_checks: false,
      insert_debug_locs: false,
      mask_byte_arithmetic: true,
      tape_size: -1
    )
    assert_raises(RuntimeError) { BIC.compile(ast, "test.bf", cfg) }
  end
end
