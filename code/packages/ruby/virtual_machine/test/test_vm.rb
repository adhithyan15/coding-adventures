# frozen_string_literal: true

require_relative "test_helper"

class TestVM < Minitest::Test
  VM = CodingAdventures::VirtualMachine::VM
  OC = CodingAdventures::VirtualMachine::OpCode
  Inst = CodingAdventures::VirtualMachine::Instruction
  CO = CodingAdventures::VirtualMachine::CodeObject

  def make_code(instructions, constants: [], names: [])
    CO.new(instructions: instructions, constants: constants, names: names)
  end

  def run_code(instructions, constants: [], names: [])
    code = make_code(instructions, constants: constants, names: names)
    vm = VM.new
    traces = vm.execute(code)
    [vm, traces]
  end

  # -----------------------------------------------------------------------
  # Stack operations
  # -----------------------------------------------------------------------

  def test_load_const
    vm, traces = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),
      Inst.new(opcode: OC::HALT)
    ], constants: [42])
    assert_equal [42], vm.stack
  end

  def test_pop
    vm, _ = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),
      Inst.new(opcode: OC::POP),
      Inst.new(opcode: OC::HALT)
    ], constants: [42])
    assert_empty vm.stack
  end

  def test_dup
    vm, _ = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),
      Inst.new(opcode: OC::DUP),
      Inst.new(opcode: OC::HALT)
    ], constants: [42])
    assert_equal [42, 42], vm.stack
  end

  # -----------------------------------------------------------------------
  # Variable operations
  # -----------------------------------------------------------------------

  def test_store_and_load_name
    vm, _ = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),
      Inst.new(opcode: OC::STORE_NAME, operand: 0),
      Inst.new(opcode: OC::LOAD_NAME, operand: 0),
      Inst.new(opcode: OC::HALT)
    ], constants: [42], names: ["x"])
    assert_equal [42], vm.stack
    assert_equal({ "x" => 42 }, vm.variables)
  end

  def test_store_and_load_local
    vm, _ = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),
      Inst.new(opcode: OC::STORE_LOCAL, operand: 0),
      Inst.new(opcode: OC::LOAD_LOCAL, operand: 0),
      Inst.new(opcode: OC::HALT)
    ], constants: [99])
    assert_equal [99], vm.stack
  end

  def test_store_local_auto_grows
    vm, _ = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),
      Inst.new(opcode: OC::STORE_LOCAL, operand: 5),
      Inst.new(opcode: OC::HALT)
    ], constants: [42])
    assert_equal 6, vm.locals.length
    assert_equal 42, vm.locals[5]
  end

  # -----------------------------------------------------------------------
  # Arithmetic
  # -----------------------------------------------------------------------

  def test_add
    vm, _ = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),
      Inst.new(opcode: OC::LOAD_CONST, operand: 1),
      Inst.new(opcode: OC::ADD),
      Inst.new(opcode: OC::HALT)
    ], constants: [3, 4])
    assert_equal [7], vm.stack
  end

  def test_sub
    vm, _ = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),
      Inst.new(opcode: OC::LOAD_CONST, operand: 1),
      Inst.new(opcode: OC::SUB),
      Inst.new(opcode: OC::HALT)
    ], constants: [10, 3])
    assert_equal [7], vm.stack
  end

  def test_mul
    vm, _ = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),
      Inst.new(opcode: OC::LOAD_CONST, operand: 1),
      Inst.new(opcode: OC::MUL),
      Inst.new(opcode: OC::HALT)
    ], constants: [6, 7])
    assert_equal [42], vm.stack
  end

  def test_div
    vm, _ = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),
      Inst.new(opcode: OC::LOAD_CONST, operand: 1),
      Inst.new(opcode: OC::DIV),
      Inst.new(opcode: OC::HALT)
    ], constants: [10, 3])
    assert_equal [3], vm.stack  # integer division
  end

  def test_add_strings
    vm, _ = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),
      Inst.new(opcode: OC::LOAD_CONST, operand: 1),
      Inst.new(opcode: OC::ADD),
      Inst.new(opcode: OC::HALT)
    ], constants: ["hello", " world"])
    assert_equal ["hello world"], vm.stack
  end

  # -----------------------------------------------------------------------
  # Comparison
  # -----------------------------------------------------------------------

  def test_cmp_eq_true
    vm, _ = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),
      Inst.new(opcode: OC::CMP_EQ),
      Inst.new(opcode: OC::HALT)
    ], constants: [42])
    assert_equal [1], vm.stack
  end

  def test_cmp_eq_false
    vm, _ = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),
      Inst.new(opcode: OC::LOAD_CONST, operand: 1),
      Inst.new(opcode: OC::CMP_EQ),
      Inst.new(opcode: OC::HALT)
    ], constants: [1, 2])
    assert_equal [0], vm.stack
  end

  def test_cmp_lt
    vm, _ = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),
      Inst.new(opcode: OC::LOAD_CONST, operand: 1),
      Inst.new(opcode: OC::CMP_LT),
      Inst.new(opcode: OC::HALT)
    ], constants: [1, 2])
    assert_equal [1], vm.stack
  end

  def test_cmp_gt
    vm, _ = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),
      Inst.new(opcode: OC::LOAD_CONST, operand: 1),
      Inst.new(opcode: OC::CMP_GT),
      Inst.new(opcode: OC::HALT)
    ], constants: [5, 3])
    assert_equal [1], vm.stack
  end

  # -----------------------------------------------------------------------
  # Control flow
  # -----------------------------------------------------------------------

  def test_jump
    vm, _ = run_code([
      Inst.new(opcode: OC::JUMP, operand: 2),
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),  # skipped
      Inst.new(opcode: OC::LOAD_CONST, operand: 1),
      Inst.new(opcode: OC::HALT)
    ], constants: [99, 42])
    assert_equal [42], vm.stack
  end

  def test_jump_if_false_takes_branch
    vm, _ = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),   # push 0 (falsy)
      Inst.new(opcode: OC::JUMP_IF_FALSE, operand: 3),
      Inst.new(opcode: OC::LOAD_CONST, operand: 1),   # skipped
      Inst.new(opcode: OC::LOAD_CONST, operand: 2),   # lands here
      Inst.new(opcode: OC::HALT)
    ], constants: [0, 99, 42])
    assert_equal [42], vm.stack
  end

  def test_jump_if_false_falls_through
    vm, _ = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),   # push 1 (truthy)
      Inst.new(opcode: OC::JUMP_IF_FALSE, operand: 3),
      Inst.new(opcode: OC::LOAD_CONST, operand: 1),   # NOT skipped
      Inst.new(opcode: OC::HALT)
    ], constants: [1, 42])
    assert_equal [42], vm.stack
  end

  def test_jump_if_true
    vm, _ = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),   # push 1 (truthy)
      Inst.new(opcode: OC::JUMP_IF_TRUE, operand: 3),
      Inst.new(opcode: OC::LOAD_CONST, operand: 1),   # skipped
      Inst.new(opcode: OC::LOAD_CONST, operand: 2),   # lands here
      Inst.new(opcode: OC::HALT)
    ], constants: [1, 99, 42])
    assert_equal [42], vm.stack
  end

  def test_loop
    # Count from 0 to 2: push 0, then increment until >= 3
    vm, _ = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),   # 0: push 0
      Inst.new(opcode: OC::STORE_NAME, operand: 0),   # 1: store x
      Inst.new(opcode: OC::LOAD_NAME, operand: 0),    # 2: push x
      Inst.new(opcode: OC::LOAD_CONST, operand: 1),   # 3: push 3
      Inst.new(opcode: OC::CMP_LT),                    # 4: x < 3?
      Inst.new(opcode: OC::JUMP_IF_FALSE, operand: 13),# 5: if false, exit to HALT
      Inst.new(opcode: OC::LOAD_NAME, operand: 0),    # 6: push x
      Inst.new(opcode: OC::PRINT),                      # 7: print x
      Inst.new(opcode: OC::LOAD_NAME, operand: 0),    # 8: push x
      Inst.new(opcode: OC::LOAD_CONST, operand: 2),   # 9: push 1
      Inst.new(opcode: OC::ADD),                        # 10: x + 1
      Inst.new(opcode: OC::STORE_NAME, operand: 0),   # 11: x = x + 1
      Inst.new(opcode: OC::JUMP, operand: 2),          # 12: jump back to loop start
      Inst.new(opcode: OC::HALT)                        # 13: exit
    ], constants: [0, 3, 1], names: ["x"])
    assert_equal %w[0 1 2], vm.output
  end

  # -----------------------------------------------------------------------
  # I/O
  # -----------------------------------------------------------------------

  def test_print
    vm, _ = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),
      Inst.new(opcode: OC::PRINT),
      Inst.new(opcode: OC::HALT)
    ], constants: [42])
    assert_equal ["42"], vm.output
  end

  def test_print_string
    vm, _ = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),
      Inst.new(opcode: OC::PRINT),
      Inst.new(opcode: OC::HALT)
    ], constants: ["hello"])
    assert_equal ["hello"], vm.output
  end

  # -----------------------------------------------------------------------
  # HALT and program end
  # -----------------------------------------------------------------------

  def test_halt_stops_execution
    vm, traces = run_code([
      Inst.new(opcode: OC::HALT),
      Inst.new(opcode: OC::LOAD_CONST, operand: 0)
    ], constants: [42])
    assert vm.halted
    assert_empty vm.stack
  end

  def test_runs_past_end
    vm, _ = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0)
    ], constants: [42])
    # Should stop gracefully when PC goes past the end.
    assert_equal [42], vm.stack
  end

  # -----------------------------------------------------------------------
  # Functions (CALL / RETURN)
  # -----------------------------------------------------------------------

  def test_function_call
    # Define a function that pushes 99 and prints it.
    func_code = CO.new(
      instructions: [
        Inst.new(opcode: OC::LOAD_CONST, operand: 0),
        Inst.new(opcode: OC::PRINT),
        Inst.new(opcode: OC::RETURN)
      ],
      constants: [99]
    )

    # Main: store the function, then call it.
    vm, _ = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),
      Inst.new(opcode: OC::STORE_NAME, operand: 0),
      Inst.new(opcode: OC::CALL, operand: 0),
      Inst.new(opcode: OC::HALT)
    ], constants: [func_code], names: ["my_func"])

    # Store the func_code in variables manually before execute
    # Actually, the LOAD_CONST + STORE_NAME already does it.
    assert_equal ["99"], vm.output
  end

  def test_return_at_top_level
    vm, _ = run_code([
      Inst.new(opcode: OC::RETURN)
    ])
    assert vm.halted
  end

  # -----------------------------------------------------------------------
  # Trace recording
  # -----------------------------------------------------------------------

  def test_trace_records_all_steps
    _, traces = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),
      Inst.new(opcode: OC::LOAD_CONST, operand: 1),
      Inst.new(opcode: OC::ADD),
      Inst.new(opcode: OC::HALT)
    ], constants: [3, 4])
    assert_equal 4, traces.length
  end

  def test_trace_has_pc
    _, traces = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),
      Inst.new(opcode: OC::HALT)
    ], constants: [42])
    assert_equal 0, traces[0].pc
    assert_equal 1, traces[1].pc
  end

  def test_trace_has_stack_snapshots
    _, traces = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),
      Inst.new(opcode: OC::HALT)
    ], constants: [42])
    assert_equal [], traces[0].stack_before
    assert_equal [42], traces[0].stack_after
  end

  def test_trace_has_variables
    _, traces = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),
      Inst.new(opcode: OC::STORE_NAME, operand: 0),
      Inst.new(opcode: OC::HALT)
    ], constants: [42], names: ["x"])
    assert_equal({ "x" => 42 }, traces[1].variables)
  end

  def test_trace_print_output
    _, traces = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),
      Inst.new(opcode: OC::PRINT),
      Inst.new(opcode: OC::HALT)
    ], constants: [42])
    assert_equal "42", traces[1].output
  end

  def test_trace_description
    _, traces = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),
      Inst.new(opcode: OC::HALT)
    ], constants: [42])
    assert_includes traces[0].description, "42"
  end

  # -----------------------------------------------------------------------
  # Reset
  # -----------------------------------------------------------------------

  def test_reset
    vm = VM.new
    code = make_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),
      Inst.new(opcode: OC::HALT)
    ], constants: [42])
    vm.execute(code)
    vm.reset
    assert_empty vm.stack
    assert_empty vm.variables
    assert_equal 0, vm.pc
    refute vm.halted
    assert_empty vm.output
  end

  # -----------------------------------------------------------------------
  # Step-by-step execution
  # -----------------------------------------------------------------------

  def test_step
    vm = VM.new
    code = make_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),
      Inst.new(opcode: OC::LOAD_CONST, operand: 1),
      Inst.new(opcode: OC::ADD),
      Inst.new(opcode: OC::HALT)
    ], constants: [3, 4])

    trace1 = vm.step(code)
    assert_equal 0, trace1.pc
    assert_equal [3], vm.stack

    trace2 = vm.step(code)
    assert_equal 1, trace2.pc
    assert_equal [3, 4], vm.stack

    trace3 = vm.step(code)
    assert_equal 2, trace3.pc
    assert_equal [7], vm.stack
  end

  # -----------------------------------------------------------------------
  # Errors
  # -----------------------------------------------------------------------

  def test_stack_underflow_pop
    assert_raises(CodingAdventures::VirtualMachine::StackUnderflowError) do
      run_code([Inst.new(opcode: OC::POP), Inst.new(opcode: OC::HALT)])
    end
  end

  def test_stack_underflow_add
    assert_raises(CodingAdventures::VirtualMachine::StackUnderflowError) do
      run_code([
        Inst.new(opcode: OC::LOAD_CONST, operand: 0),
        Inst.new(opcode: OC::ADD),
        Inst.new(opcode: OC::HALT)
      ], constants: [1])
    end
  end

  def test_dup_empty_stack
    assert_raises(CodingAdventures::VirtualMachine::StackUnderflowError) do
      run_code([Inst.new(opcode: OC::DUP), Inst.new(opcode: OC::HALT)])
    end
  end

  def test_division_by_zero
    assert_raises(CodingAdventures::VirtualMachine::DivisionByZeroError) do
      run_code([
        Inst.new(opcode: OC::LOAD_CONST, operand: 0),
        Inst.new(opcode: OC::LOAD_CONST, operand: 1),
        Inst.new(opcode: OC::DIV),
        Inst.new(opcode: OC::HALT)
      ], constants: [10, 0])
    end
  end

  def test_undefined_name
    assert_raises(CodingAdventures::VirtualMachine::UndefinedNameError) do
      run_code([
        Inst.new(opcode: OC::LOAD_NAME, operand: 0),
        Inst.new(opcode: OC::HALT)
      ], names: ["x"])
    end
  end

  def test_invalid_opcode
    assert_raises(CodingAdventures::VirtualMachine::InvalidOpcodeError) do
      run_code([Inst.new(opcode: 0xEE), Inst.new(opcode: OC::HALT)])
    end
  end

  def test_load_const_out_of_range
    assert_raises(CodingAdventures::VirtualMachine::InvalidOperandError) do
      run_code([
        Inst.new(opcode: OC::LOAD_CONST, operand: 99),
        Inst.new(opcode: OC::HALT)
      ], constants: [1])
    end
  end

  def test_load_const_missing_operand
    assert_raises(CodingAdventures::VirtualMachine::InvalidOperandError) do
      run_code([Inst.new(opcode: OC::LOAD_CONST), Inst.new(opcode: OC::HALT)])
    end
  end

  def test_store_name_out_of_range
    assert_raises(CodingAdventures::VirtualMachine::InvalidOperandError) do
      run_code([
        Inst.new(opcode: OC::LOAD_CONST, operand: 0),
        Inst.new(opcode: OC::STORE_NAME, operand: 99),
        Inst.new(opcode: OC::HALT)
      ], constants: [1], names: ["x"])
    end
  end

  def test_load_name_out_of_range
    assert_raises(CodingAdventures::VirtualMachine::InvalidOperandError) do
      run_code([
        Inst.new(opcode: OC::LOAD_NAME, operand: 99),
        Inst.new(opcode: OC::HALT)
      ], names: ["x"])
    end
  end

  def test_store_local_invalid_operand
    assert_raises(CodingAdventures::VirtualMachine::InvalidOperandError) do
      run_code([
        Inst.new(opcode: OC::LOAD_CONST, operand: 0),
        Inst.new(opcode: OC::STORE_LOCAL, operand: -1),
        Inst.new(opcode: OC::HALT)
      ], constants: [1])
    end
  end

  def test_load_local_uninitialized
    assert_raises(CodingAdventures::VirtualMachine::InvalidOperandError) do
      run_code([
        Inst.new(opcode: OC::LOAD_LOCAL, operand: 0),
        Inst.new(opcode: OC::HALT)
      ])
    end
  end

  def test_load_local_invalid_operand
    assert_raises(CodingAdventures::VirtualMachine::InvalidOperandError) do
      run_code([
        Inst.new(opcode: OC::LOAD_LOCAL, operand: -1),
        Inst.new(opcode: OC::HALT)
      ])
    end
  end

  def test_jump_non_integer
    assert_raises(CodingAdventures::VirtualMachine::InvalidOperandError) do
      run_code([
        Inst.new(opcode: OC::JUMP, operand: "bad"),
        Inst.new(opcode: OC::HALT)
      ])
    end
  end

  def test_jump_if_false_non_integer
    assert_raises(CodingAdventures::VirtualMachine::InvalidOperandError) do
      run_code([
        Inst.new(opcode: OC::LOAD_CONST, operand: 0),
        Inst.new(opcode: OC::JUMP_IF_FALSE, operand: "bad"),
        Inst.new(opcode: OC::HALT)
      ], constants: [0])
    end
  end

  def test_jump_if_true_non_integer
    assert_raises(CodingAdventures::VirtualMachine::InvalidOperandError) do
      run_code([
        Inst.new(opcode: OC::LOAD_CONST, operand: 0),
        Inst.new(opcode: OC::JUMP_IF_TRUE, operand: "bad"),
        Inst.new(opcode: OC::HALT)
      ], constants: [1])
    end
  end

  def test_call_undefined_function
    assert_raises(CodingAdventures::VirtualMachine::UndefinedNameError) do
      run_code([
        Inst.new(opcode: OC::CALL, operand: 0),
        Inst.new(opcode: OC::HALT)
      ], names: ["missing"])
    end
  end

  def test_call_non_callable
    assert_raises(CodingAdventures::VirtualMachine::VMError) do
      run_code([
        Inst.new(opcode: OC::LOAD_CONST, operand: 0),
        Inst.new(opcode: OC::STORE_NAME, operand: 0),
        Inst.new(opcode: OC::CALL, operand: 0),
        Inst.new(opcode: OC::HALT)
      ], constants: [42], names: ["not_func"])
    end
  end

  def test_call_out_of_range
    assert_raises(CodingAdventures::VirtualMachine::InvalidOperandError) do
      run_code([
        Inst.new(opcode: OC::CALL, operand: 99),
        Inst.new(opcode: OC::HALT)
      ], names: ["f"])
    end
  end

  # -----------------------------------------------------------------------
  # Data type representations
  # -----------------------------------------------------------------------

  def test_instruction_to_s
    inst = Inst.new(opcode: OC::LOAD_CONST, operand: 0)
    assert_includes inst.to_s, "LOAD_CONST"
    assert_includes inst.to_s, "0"
  end

  def test_instruction_to_s_no_operand
    inst = Inst.new(opcode: OC::ADD)
    assert_includes inst.to_s, "ADD"
  end

  def test_instruction_unknown_opcode_to_s
    inst = Inst.new(opcode: 0xEE)
    assert_includes inst.to_s, "UNKNOWN"
  end

  # -----------------------------------------------------------------------
  # Complex program: x = 3 + 4 * 2; print x
  # -----------------------------------------------------------------------

  def test_complex_program
    vm, _ = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),  # 3
      Inst.new(opcode: OC::LOAD_CONST, operand: 1),  # 4
      Inst.new(opcode: OC::LOAD_CONST, operand: 2),  # 2
      Inst.new(opcode: OC::MUL),                       # 4 * 2 = 8
      Inst.new(opcode: OC::ADD),                       # 3 + 8 = 11
      Inst.new(opcode: OC::STORE_NAME, operand: 0),  # x = 11
      Inst.new(opcode: OC::LOAD_NAME, operand: 0),   # push x
      Inst.new(opcode: OC::PRINT),                     # print 11
      Inst.new(opcode: OC::HALT)
    ], constants: [3, 4, 2], names: ["x"])
    assert_equal ["11"], vm.output
    assert_equal({ "x" => 11 }, vm.variables)
  end

  # -----------------------------------------------------------------------
  # Falsy values
  # -----------------------------------------------------------------------

  def test_nil_is_falsy
    vm, _ = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),
      Inst.new(opcode: OC::JUMP_IF_FALSE, operand: 3),
      Inst.new(opcode: OC::LOAD_CONST, operand: 1),  # skipped
      Inst.new(opcode: OC::LOAD_CONST, operand: 2),
      Inst.new(opcode: OC::HALT)
    ], constants: [nil, 99, 42])
    assert_equal [42], vm.stack
  end

  def test_empty_string_is_falsy
    vm, _ = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),
      Inst.new(opcode: OC::JUMP_IF_FALSE, operand: 3),
      Inst.new(opcode: OC::LOAD_CONST, operand: 1),  # skipped
      Inst.new(opcode: OC::LOAD_CONST, operand: 2),
      Inst.new(opcode: OC::HALT)
    ], constants: ["", 99, 42])
    assert_equal [42], vm.stack
  end

  # -----------------------------------------------------------------------
  # Describe edge cases
  # -----------------------------------------------------------------------

  def test_describe_sub
    _, traces = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),
      Inst.new(opcode: OC::LOAD_CONST, operand: 1),
      Inst.new(opcode: OC::SUB),
      Inst.new(opcode: OC::HALT)
    ], constants: [10, 3])
    assert_includes traces[2].description, "difference"
  end

  def test_describe_mul
    _, traces = run_code([
      Inst.new(opcode: OC::LOAD_CONST, operand: 0),
      Inst.new(opcode: OC::LOAD_CONST, operand: 1),
      Inst.new(opcode: OC::MUL),
      Inst.new(opcode: OC::HALT)
    ], constants: [3, 4])
    assert_includes traces[2].description, "product"
  end
end
