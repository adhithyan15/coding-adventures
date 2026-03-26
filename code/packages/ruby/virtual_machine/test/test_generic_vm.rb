# frozen_string_literal: true

# ==========================================================================
# Tests for GenericVM — the Pluggable Stack-Based Bytecode Interpreter
# ==========================================================================
#
# These tests verify that GenericVM works as a blank chassis: you register
# opcode handlers, and the VM dispatches to them. No hardcoded opcodes.
#
# We define tiny "toy" handlers inline to test each piece of GenericVM
# without depending on any language plugin (Starlark, Brainfuck, etc.).
# ==========================================================================

require_relative "test_helper"

class TestGenericVM < Minitest::Test
  GVM = CodingAdventures::VirtualMachine::GenericVM
  Inst = CodingAdventures::VirtualMachine::Instruction
  CO = CodingAdventures::VirtualMachine::CodeObject

  # -- Opcode constants for our toy language --
  OP_PUSH   = 0x01  # Push operand onto stack
  OP_ADD    = 0x02  # Pop two, push sum
  OP_PRINT  = 0x03  # Pop and output as string
  OP_HALT   = 0xFF  # Stop execution

  # -- Toy handlers --
  HANDLER_PUSH = ->(vm, instr, _code) {
    vm.push(instr.operand)
    vm.advance_pc
    nil
  }

  HANDLER_ADD = ->(vm, _instr, _code) {
    b = vm.pop
    a = vm.pop
    vm.push(a + b)
    vm.advance_pc
    nil
  }

  HANDLER_PRINT = ->(vm, _instr, _code) {
    value = vm.pop
    vm.output << value.to_s
    vm.advance_pc
    value.to_s
  }

  HANDLER_HALT = ->(vm, _instr, _code) {
    vm.halted = true
    vm.advance_pc
    nil
  }

  def setup_vm
    vm = GVM.new
    vm.register_opcode(OP_PUSH, HANDLER_PUSH)
    vm.register_opcode(OP_ADD, HANDLER_ADD)
    vm.register_opcode(OP_PRINT, HANDLER_PRINT)
    vm.register_opcode(OP_HALT, HANDLER_HALT)
    vm
  end

  def make_code(instructions, constants: [], names: [])
    CO.new(instructions: instructions, constants: constants, names: names)
  end

  # =====================================================================
  # Basic execution
  # =====================================================================

  def test_empty_program_halts
    vm = setup_vm
    code = make_code([Inst.new(opcode: OP_HALT)])
    traces = vm.execute(code)
    assert_equal 1, traces.length
    assert vm.halted
  end

  def test_push_and_halt
    vm = setup_vm
    code = make_code([
      Inst.new(opcode: OP_PUSH, operand: 42),
      Inst.new(opcode: OP_HALT)
    ])
    vm.execute(code)
    assert_equal [42], vm.stack
  end

  def test_push_add_halt
    vm = setup_vm
    code = make_code([
      Inst.new(opcode: OP_PUSH, operand: 3),
      Inst.new(opcode: OP_PUSH, operand: 4),
      Inst.new(opcode: OP_ADD),
      Inst.new(opcode: OP_HALT)
    ])
    vm.execute(code)
    assert_equal [7], vm.stack
  end

  def test_print_output
    vm = setup_vm
    code = make_code([
      Inst.new(opcode: OP_PUSH, operand: 42),
      Inst.new(opcode: OP_PRINT),
      Inst.new(opcode: OP_HALT)
    ])
    vm.execute(code)
    assert_equal ["42"], vm.output
  end

  # =====================================================================
  # Trace recording
  # =====================================================================

  def test_traces_count
    vm = setup_vm
    code = make_code([
      Inst.new(opcode: OP_PUSH, operand: 1),
      Inst.new(opcode: OP_PUSH, operand: 2),
      Inst.new(opcode: OP_ADD),
      Inst.new(opcode: OP_HALT)
    ])
    traces = vm.execute(code)
    assert_equal 4, traces.length
  end

  def test_trace_has_pc
    vm = setup_vm
    code = make_code([
      Inst.new(opcode: OP_PUSH, operand: 42),
      Inst.new(opcode: OP_HALT)
    ])
    traces = vm.execute(code)
    assert_equal 0, traces[0].pc
    assert_equal 1, traces[1].pc
  end

  def test_trace_has_stack_snapshots
    vm = setup_vm
    code = make_code([
      Inst.new(opcode: OP_PUSH, operand: 42),
      Inst.new(opcode: OP_HALT)
    ])
    traces = vm.execute(code)
    assert_equal [], traces[0].stack_before
    assert_equal [42], traces[0].stack_after
  end

  def test_trace_output_from_print
    vm = setup_vm
    code = make_code([
      Inst.new(opcode: OP_PUSH, operand: 99),
      Inst.new(opcode: OP_PRINT),
      Inst.new(opcode: OP_HALT)
    ])
    traces = vm.execute(code)
    assert_equal "99", traces[1].output
  end

  def test_trace_description
    vm = setup_vm
    code = make_code([
      Inst.new(opcode: OP_PUSH, operand: 42),
      Inst.new(opcode: OP_HALT)
    ])
    traces = vm.execute(code)
    assert_includes traces[0].description, "0x01"
    assert_includes traces[0].description, "42"
  end

  # =====================================================================
  # Stack operations
  # =====================================================================

  def test_push_pop
    vm = GVM.new
    vm.push(10)
    vm.push(20)
    assert_equal 20, vm.pop
    assert_equal 10, vm.pop
  end

  def test_peek
    vm = GVM.new
    vm.push(42)
    assert_equal 42, vm.peek
    assert_equal [42], vm.stack  # peek doesn't remove
  end

  def test_pop_empty_stack_raises
    vm = GVM.new
    assert_raises(CodingAdventures::VirtualMachine::StackUnderflowError) do
      vm.pop
    end
  end

  def test_peek_empty_stack_raises
    vm = GVM.new
    assert_raises(CodingAdventures::VirtualMachine::StackUnderflowError) do
      vm.peek
    end
  end

  # =====================================================================
  # Call stack
  # =====================================================================

  def test_push_pop_frame
    vm = GVM.new
    frame = { return_address: 5, saved_vars: {} }
    vm.push_frame(frame)
    assert_equal frame, vm.pop_frame
  end

  def test_pop_frame_empty_raises
    vm = GVM.new
    assert_raises(CodingAdventures::VirtualMachine::VMError) do
      vm.pop_frame
    end
  end

  def test_max_recursion_depth
    vm = GVM.new
    vm.set_max_recursion_depth(2)
    vm.push_frame({ a: 1 })
    vm.push_frame({ a: 2 })
    assert_raises(CodingAdventures::VirtualMachine::MaxRecursionError) do
      vm.push_frame({ a: 3 })
    end
  end

  def test_max_recursion_depth_zero
    vm = GVM.new
    vm.set_max_recursion_depth(0)
    assert_raises(CodingAdventures::VirtualMachine::MaxRecursionError) do
      vm.push_frame({ a: 1 })
    end
  end

  def test_unlimited_recursion_by_default
    vm = GVM.new
    100.times { |i| vm.push_frame({ i: i }) }
    assert_equal 100, vm.call_stack.length
  end

  # =====================================================================
  # Program counter
  # =====================================================================

  def test_advance_pc
    vm = GVM.new
    assert_equal 0, vm.pc
    vm.advance_pc
    assert_equal 1, vm.pc
  end

  def test_jump_to
    vm = GVM.new
    vm.jump_to(10)
    assert_equal 10, vm.pc
  end

  # =====================================================================
  # Built-in functions
  # =====================================================================

  def test_register_and_get_builtin
    vm = GVM.new
    impl = ->(args) { args.sum }
    vm.register_builtin("sum", impl)
    builtin = vm.get_builtin("sum")
    assert_equal "sum", builtin.name
    assert_equal impl, builtin.implementation
  end

  def test_get_builtin_not_found
    vm = GVM.new
    assert_nil vm.get_builtin("nonexistent")
  end

  # =====================================================================
  # Configuration
  # =====================================================================

  def test_frozen_state
    vm = GVM.new
    refute vm.frozen?
    vm.set_frozen(true)
    assert vm.frozen?
    vm.set_frozen(false)
    refute vm.frozen?
  end

  def test_max_recursion_depth_getter
    vm = GVM.new
    assert_nil vm.max_recursion_depth
    vm.set_max_recursion_depth(50)
    assert_equal 50, vm.max_recursion_depth
  end

  # =====================================================================
  # Reset
  # =====================================================================

  def test_reset_clears_state
    vm = setup_vm
    code = make_code([
      Inst.new(opcode: OP_PUSH, operand: 42),
      Inst.new(opcode: OP_PRINT),
      Inst.new(opcode: OP_HALT)
    ])
    vm.execute(code)

    vm.reset
    assert_empty vm.stack
    assert_empty vm.variables
    assert_empty vm.locals
    assert_equal 0, vm.pc
    refute vm.halted
    assert_empty vm.output
    assert_empty vm.call_stack
  end

  def test_reset_preserves_handlers
    vm = setup_vm
    code = make_code([
      Inst.new(opcode: OP_PUSH, operand: 42),
      Inst.new(opcode: OP_HALT)
    ])
    vm.execute(code)
    vm.reset

    # Handlers should still work after reset
    code2 = make_code([
      Inst.new(opcode: OP_PUSH, operand: 99),
      Inst.new(opcode: OP_HALT)
    ])
    vm.execute(code2)
    assert_equal [99], vm.stack
  end

  # =====================================================================
  # Error handling
  # =====================================================================

  def test_unknown_opcode_raises
    vm = GVM.new
    vm.register_opcode(OP_HALT, HANDLER_HALT)
    code = make_code([
      Inst.new(opcode: 0xEE),
      Inst.new(opcode: OP_HALT)
    ])
    assert_raises(CodingAdventures::VirtualMachine::InvalidOpcodeError) do
      vm.execute(code)
    end
  end

  def test_no_handlers_registered
    vm = GVM.new
    code = make_code([Inst.new(opcode: 0x01)])
    assert_raises(CodingAdventures::VirtualMachine::InvalidOpcodeError) do
      vm.execute(code)
    end
  end

  # =====================================================================
  # Step-by-step execution
  # =====================================================================

  def test_step_returns_trace
    vm = setup_vm
    code = make_code([
      Inst.new(opcode: OP_PUSH, operand: 42),
      Inst.new(opcode: OP_HALT)
    ])
    trace = vm.step(code)
    assert_equal 0, trace.pc
    assert_equal [42], vm.stack
  end

  def test_step_by_step_execution
    vm = setup_vm
    code = make_code([
      Inst.new(opcode: OP_PUSH, operand: 3),
      Inst.new(opcode: OP_PUSH, operand: 4),
      Inst.new(opcode: OP_ADD),
      Inst.new(opcode: OP_HALT)
    ])

    vm.step(code)
    assert_equal [3], vm.stack

    vm.step(code)
    assert_equal [3, 4], vm.stack

    vm.step(code)
    assert_equal [7], vm.stack

    vm.step(code)
    assert vm.halted
  end

  # =====================================================================
  # Variables and locals (direct access)
  # =====================================================================

  def test_variables_accessible
    vm = GVM.new
    vm.variables["x"] = 42
    assert_equal 42, vm.variables["x"]
  end

  def test_locals_accessible
    vm = GVM.new
    vm.locals << 10
    vm.locals << 20
    assert_equal [10, 20], vm.locals
  end

  def test_inject_globals_merges_and_overwrites_variables
    vm = GVM.new
    vm.variables["existing"] = 1
    vm.variables["ctx_os"] = "linux"

    vm.inject_globals("ctx_os" => "darwin", "answer" => 42)

    assert_equal(
      {
        "existing" => 1,
        "ctx_os" => "darwin",
        "answer" => 42
      },
      vm.variables
    )
  end

  # =====================================================================
  # Program ends without halt (PC past end)
  # =====================================================================

  def test_stops_when_pc_past_end
    vm = GVM.new
    vm.register_opcode(OP_PUSH, HANDLER_PUSH)
    code = make_code([
      Inst.new(opcode: OP_PUSH, operand: 42)
    ])
    traces = vm.execute(code)
    assert_equal 1, traces.length
    assert_equal [42], vm.stack
  end
end
