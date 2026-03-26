# frozen_string_literal: true

# ==========================================================================
# Tests for Brainfuck Opcode Handlers
# ==========================================================================
#
# Each handler is tested in isolation. We create a Brainfuck VM, set up
# specific state, execute one instruction via vm.step(), and verify the
# result.
# ==========================================================================

require_relative "test_helper"

class TestHandlers < Minitest::Test
  Op = CodingAdventures::Brainfuck::Op
  Inst = CodingAdventures::VirtualMachine::Instruction
  CO = CodingAdventures::VirtualMachine::CodeObject

  def create_vm(input_data: "")
    CodingAdventures::Brainfuck.create_brainfuck_vm(input_data: input_data)
  end

  def make_code(instructions)
    CO.new(instructions: instructions, constants: [], names: [])
  end

  # =====================================================================
  # Pointer movement
  # =====================================================================

  def test_right_moves_pointer
    vm = create_vm
    code = make_code([Inst.new(opcode: Op::RIGHT), Inst.new(opcode: Op::HALT)])
    vm.step(code)
    assert_equal 1, vm.dp
  end

  def test_left_moves_pointer
    vm = create_vm
    vm.dp = 5
    code = make_code([Inst.new(opcode: Op::LEFT), Inst.new(opcode: Op::HALT)])
    vm.step(code)
    assert_equal 4, vm.dp
  end

  def test_right_past_end_raises
    vm = create_vm
    vm.dp = CodingAdventures::Brainfuck::TAPE_SIZE - 1
    code = make_code([Inst.new(opcode: Op::RIGHT), Inst.new(opcode: Op::HALT)])
    assert_raises(CodingAdventures::Brainfuck::BrainfuckError) do
      vm.step(code)
    end
  end

  def test_left_before_start_raises
    vm = create_vm
    vm.dp = 0
    code = make_code([Inst.new(opcode: Op::LEFT), Inst.new(opcode: Op::HALT)])
    assert_raises(CodingAdventures::Brainfuck::BrainfuckError) do
      vm.step(code)
    end
  end

  # =====================================================================
  # Cell modification
  # =====================================================================

  def test_inc_increments_cell
    vm = create_vm
    code = make_code([Inst.new(opcode: Op::INC), Inst.new(opcode: Op::HALT)])
    vm.step(code)
    assert_equal 1, vm.tape[0]
  end

  def test_dec_decrements_cell
    vm = create_vm
    vm.tape[0] = 5
    code = make_code([Inst.new(opcode: Op::DEC), Inst.new(opcode: Op::HALT)])
    vm.step(code)
    assert_equal 4, vm.tape[0]
  end

  def test_inc_wraps_at_255
    vm = create_vm
    vm.tape[0] = 255
    code = make_code([Inst.new(opcode: Op::INC), Inst.new(opcode: Op::HALT)])
    vm.step(code)
    assert_equal 0, vm.tape[0]
  end

  def test_dec_wraps_at_0
    vm = create_vm
    vm.tape[0] = 0
    code = make_code([Inst.new(opcode: Op::DEC), Inst.new(opcode: Op::HALT)])
    vm.step(code)
    assert_equal 255, vm.tape[0]
  end

  def test_inc_multiple_times
    vm = create_vm
    code = make_code([
      Inst.new(opcode: Op::INC),
      Inst.new(opcode: Op::INC),
      Inst.new(opcode: Op::INC),
      Inst.new(opcode: Op::HALT)
    ])
    vm.execute(code)
    assert_equal 3, vm.tape[0]
  end

  # =====================================================================
  # Output
  # =====================================================================

  def test_output_appends_char
    vm = create_vm
    vm.tape[0] = 72  # 'H'
    code = make_code([Inst.new(opcode: Op::OUTPUT), Inst.new(opcode: Op::HALT)])
    vm.step(code)
    assert_equal ["H"], vm.output
  end

  def test_output_returns_char
    vm = create_vm
    vm.tape[0] = 65  # 'A'
    code = make_code([Inst.new(opcode: Op::OUTPUT), Inst.new(opcode: Op::HALT)])
    trace = vm.step(code)
    assert_equal "A", trace.output
  end

  # =====================================================================
  # Input
  # =====================================================================

  def test_input_reads_byte
    vm = create_vm(input_data: "A")
    code = make_code([Inst.new(opcode: Op::INPUT), Inst.new(opcode: Op::HALT)])
    vm.step(code)
    assert_equal 65, vm.tape[0]
  end

  def test_input_advances_position
    vm = create_vm(input_data: "AB")
    code = make_code([
      Inst.new(opcode: Op::INPUT),
      Inst.new(opcode: Op::RIGHT),
      Inst.new(opcode: Op::INPUT),
      Inst.new(opcode: Op::HALT)
    ])
    vm.execute(code)
    assert_equal 65, vm.tape[0]
    assert_equal 66, vm.tape[1]
  end

  def test_input_eof_returns_zero
    vm = create_vm(input_data: "")
    code = make_code([Inst.new(opcode: Op::INPUT), Inst.new(opcode: Op::HALT)])
    vm.step(code)
    assert_equal 0, vm.tape[0]
  end

  def test_input_eof_after_data
    vm = create_vm(input_data: "X")
    code = make_code([
      Inst.new(opcode: Op::INPUT),
      Inst.new(opcode: Op::RIGHT),
      Inst.new(opcode: Op::INPUT),
      Inst.new(opcode: Op::HALT)
    ])
    vm.execute(code)
    assert_equal 88, vm.tape[0]  # 'X'
    assert_equal 0, vm.tape[1]   # EOF
  end

  # =====================================================================
  # Loop control flow
  # =====================================================================

  def test_loop_start_skips_when_zero
    vm = create_vm
    vm.tape[0] = 0
    code = make_code([
      Inst.new(opcode: Op::LOOP_START, operand: 3),
      Inst.new(opcode: Op::INC),
      Inst.new(opcode: Op::LOOP_END, operand: 0),
      Inst.new(opcode: Op::HALT)
    ])
    vm.execute(code)
    assert_equal 0, vm.tape[0]  # INC was skipped
  end

  def test_loop_start_enters_when_nonzero
    vm = create_vm
    vm.tape[0] = 1
    code = make_code([
      Inst.new(opcode: Op::LOOP_START, operand: 4),
      Inst.new(opcode: Op::DEC),
      Inst.new(opcode: Op::LOOP_END, operand: 0),
      Inst.new(opcode: Op::HALT)  # index 3, but LOOP_START operand is 4 (past the range)
    ])
    # Wait — LOOP_START operand should be index past LOOP_END.
    # LOOP_END is at 2, so operand = 3. Let me fix:
    code = make_code([
      Inst.new(opcode: Op::LOOP_START, operand: 3),
      Inst.new(opcode: Op::DEC),
      Inst.new(opcode: Op::LOOP_END, operand: 0),
      Inst.new(opcode: Op::HALT)
    ])
    vm.execute(code)
    assert_equal 0, vm.tape[0]  # DEC ran once (1 → 0), then loop exited
  end

  def test_loop_end_jumps_back_when_nonzero
    vm = create_vm
    vm.tape[0] = 3
    code = make_code([
      Inst.new(opcode: Op::LOOP_START, operand: 3),
      Inst.new(opcode: Op::DEC),
      Inst.new(opcode: Op::LOOP_END, operand: 0),
      Inst.new(opcode: Op::HALT)
    ])
    vm.execute(code)
    assert_equal 0, vm.tape[0]  # Looped 3 times
  end

  def test_loop_end_falls_through_when_zero
    vm = create_vm
    vm.tape[0] = 1
    code = make_code([
      Inst.new(opcode: Op::LOOP_START, operand: 3),
      Inst.new(opcode: Op::DEC),
      Inst.new(opcode: Op::LOOP_END, operand: 0),
      Inst.new(opcode: Op::HALT)
    ])
    vm.execute(code)
    assert vm.halted
  end

  # =====================================================================
  # HALT
  # =====================================================================

  def test_halt_stops_vm
    vm = create_vm
    code = make_code([Inst.new(opcode: Op::HALT)])
    vm.execute(code)
    assert vm.halted
  end

  # =====================================================================
  # PC advancement
  # =====================================================================

  def test_pc_advances_after_inc
    vm = create_vm
    code = make_code([Inst.new(opcode: Op::INC), Inst.new(opcode: Op::HALT)])
    vm.step(code)
    assert_equal 1, vm.pc
  end

  def test_pc_advances_after_right
    vm = create_vm
    code = make_code([Inst.new(opcode: Op::RIGHT), Inst.new(opcode: Op::HALT)])
    vm.step(code)
    assert_equal 1, vm.pc
  end
end
