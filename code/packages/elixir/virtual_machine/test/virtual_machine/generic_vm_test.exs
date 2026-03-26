defmodule CodingAdventures.VirtualMachine.GenericVMTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.VirtualMachine.GenericVM
  alias CodingAdventures.VirtualMachine.Types.{Instruction, CodeObject, VMTrace, CallFrame, BuiltinFunction}
  alias CodingAdventures.VirtualMachine.Errors

  # ===========================================================================
  # Test Opcodes
  #
  # We define a small toy instruction set for testing. These are the minimum
  # opcodes needed to verify all VM functionality:
  #
  #   OP_PUSH  (0x01) — push a constant from the pool onto the stack
  #   OP_ADD   (0x02) — pop two values, push their sum
  #   OP_PRINT (0x03) — pop the top value and output it as a string
  #   OP_HALT  (0xFF) — stop execution
  # ===========================================================================

  @op_push  0x01
  @op_add   0x02
  @op_print 0x03
  @op_halt  0xFF

  # --- Handler functions for the toy instruction set ---

  defp handle_push(vm, instr, code) do
    value = Enum.at(code.constants, instr.operand)
    vm = GenericVM.push(vm, value)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  defp handle_add(vm, _instr, _code) do
    {b, vm} = GenericVM.pop(vm)
    {a, vm} = GenericVM.pop(vm)
    vm = GenericVM.push(vm, a + b)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  defp handle_print(vm, _instr, _code) do
    {value, vm} = GenericVM.pop(vm)
    output = inspect(value)
    vm = GenericVM.advance_pc(vm)
    {output, vm}
  end

  defp handle_halt(vm, _instr, _code) do
    vm = %{vm | halted: true}
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  # --- Helper to build a VM with all test opcodes registered ---

  defp build_test_vm do
    GenericVM.new()
    |> GenericVM.register_opcode(@op_push, &handle_push/3)
    |> GenericVM.register_opcode(@op_add, &handle_add/3)
    |> GenericVM.register_opcode(@op_print, &handle_print/3)
    |> GenericVM.register_opcode(@op_halt, &handle_halt/3)
  end

  # ===========================================================================
  # Construction Tests
  # ===========================================================================

  describe "new/0" do
    test "creates a VM with empty state" do
      vm = GenericVM.new()
      assert vm.stack == []
      assert vm.variables == %{}
      assert vm.locals == []
      assert vm.pc == 0
      assert vm.halted == false
      assert vm.output == []
      assert vm.call_stack == []
      assert vm.handlers == %{}
      assert vm.builtins == %{}
      assert vm.max_recursion_depth == nil
      assert vm.frozen == false
      assert vm.extra == %{}
    end
  end

  # ===========================================================================
  # Plugin Registration Tests
  # ===========================================================================

  describe "register_opcode/3" do
    test "registers a handler for an opcode" do
      vm = GenericVM.new()
      handler = fn vm, _instr, _code -> {nil, vm} end
      vm = GenericVM.register_opcode(vm, 0x01, handler)
      assert Map.has_key?(vm.handlers, 0x01)
    end

    test "overwrites an existing handler for the same opcode" do
      handler1 = fn vm, _instr, _code -> {"first", vm} end
      handler2 = fn vm, _instr, _code -> {"second", vm} end

      vm = GenericVM.new()
            |> GenericVM.register_opcode(0x01, handler1)
            |> GenericVM.register_opcode(0x01, handler2)

      # The second handler should have replaced the first
      assert map_size(vm.handlers) == 1

      # Verify by executing
      code = %CodeObject{
        instructions: [%Instruction{opcode: 0x01, operand: nil}],
        constants: []
      }

      # handler2 does not advance PC, so step will work but we just check output
      {trace, _vm} = GenericVM.step(vm, code)
      assert trace.output == "second"
    end

    test "registers multiple different opcodes" do
      vm = build_test_vm()
      assert map_size(vm.handlers) == 4
      assert Map.has_key?(vm.handlers, @op_push)
      assert Map.has_key?(vm.handlers, @op_add)
      assert Map.has_key?(vm.handlers, @op_print)
      assert Map.has_key?(vm.handlers, @op_halt)
    end
  end

  describe "register_builtin/3" do
    test "registers a builtin function" do
      vm = GenericVM.new()
      impl = fn _args, vm -> {nil, vm} end
      vm = GenericVM.register_builtin(vm, "print", impl)
      assert Map.has_key?(vm.builtins, "print")
    end

    test "the registered builtin has correct name and implementation" do
      impl = fn _args, vm -> {nil, vm} end
      vm = GenericVM.new() |> GenericVM.register_builtin("len", impl)
      builtin = GenericVM.get_builtin(vm, "len")
      assert %BuiltinFunction{} = builtin
      assert builtin.name == "len"
      assert builtin.implementation == impl
    end
  end

  describe "get_builtin/2" do
    test "returns nil for unregistered builtin" do
      vm = GenericVM.new()
      assert GenericVM.get_builtin(vm, "nonexistent") == nil
    end

    test "returns the builtin when registered" do
      impl = fn _args, vm -> {nil, vm} end
      vm = GenericVM.new() |> GenericVM.register_builtin("sqrt", impl)
      builtin = GenericVM.get_builtin(vm, "sqrt")
      assert builtin != nil
      assert builtin.name == "sqrt"
    end
  end

  # ===========================================================================
  # Stack Operation Tests
  # ===========================================================================

  describe "push/2" do
    test "pushes a value onto the stack" do
      vm = GenericVM.new() |> GenericVM.push(42)
      assert vm.stack == [42]
    end

    test "pushes multiple values (last pushed is on top)" do
      vm = GenericVM.new()
            |> GenericVM.push(1)
            |> GenericVM.push(2)
            |> GenericVM.push(3)
      assert vm.stack == [3, 2, 1]
    end

    test "pushes different types" do
      vm = GenericVM.new()
            |> GenericVM.push(42)
            |> GenericVM.push("hello")
            |> GenericVM.push(true)
            |> GenericVM.push([1, 2, 3])
      assert vm.stack == [[1, 2, 3], true, "hello", 42]
    end
  end

  describe "pop/1" do
    test "pops the top value" do
      vm = GenericVM.new() |> GenericVM.push(42)
      {value, vm} = GenericVM.pop(vm)
      assert value == 42
      assert vm.stack == []
    end

    test "pops values in LIFO order" do
      vm = GenericVM.new() |> GenericVM.push(1) |> GenericVM.push(2)
      {first, vm} = GenericVM.pop(vm)
      {second, _vm} = GenericVM.pop(vm)
      assert first == 2
      assert second == 1
    end

    test "raises StackUnderflowError on empty stack" do
      vm = GenericVM.new()
      assert_raise Errors.StackUnderflowError, fn ->
        GenericVM.pop(vm)
      end
    end
  end

  describe "peek/1" do
    test "returns the top value without removing it" do
      vm = GenericVM.new() |> GenericVM.push(42)
      assert GenericVM.peek(vm) == 42
      # Stack should be unchanged
      assert vm.stack == [42]
    end

    test "raises StackUnderflowError on empty stack" do
      vm = GenericVM.new()
      assert_raise Errors.StackUnderflowError, fn ->
        GenericVM.peek(vm)
      end
    end
  end

  # ===========================================================================
  # Call Stack Tests
  # ===========================================================================

  describe "push_frame/2" do
    test "pushes a frame onto the call stack" do
      vm = GenericVM.new()
      frame = %CallFrame{return_address: 5, saved_variables: %{"x" => 1}, saved_locals: []}
      vm = GenericVM.push_frame(vm, frame)
      assert length(vm.call_stack) == 1
      assert hd(vm.call_stack) == frame
    end

    test "pushes multiple frames" do
      vm = GenericVM.new()
      frame1 = %CallFrame{return_address: 5, saved_variables: %{}, saved_locals: []}
      frame2 = %CallFrame{return_address: 10, saved_variables: %{}, saved_locals: []}
      vm = vm |> GenericVM.push_frame(frame1) |> GenericVM.push_frame(frame2)
      assert length(vm.call_stack) == 2
    end

    test "raises MaxRecursionError when depth exceeded" do
      vm = GenericVM.new() |> GenericVM.set_max_recursion_depth(2)
      frame = %CallFrame{return_address: 0, saved_variables: %{}, saved_locals: []}
      vm = vm |> GenericVM.push_frame(frame) |> GenericVM.push_frame(frame)

      assert_raise Errors.MaxRecursionError, fn ->
        GenericVM.push_frame(vm, frame)
      end
    end

    test "allows unlimited frames when max_recursion_depth is nil" do
      vm = GenericVM.new()
      frame = %CallFrame{return_address: 0, saved_variables: %{}, saved_locals: []}

      # Push 100 frames — should not raise
      vm = Enum.reduce(1..100, vm, fn _i, acc -> GenericVM.push_frame(acc, frame) end)
      assert length(vm.call_stack) == 100
    end
  end

  describe "pop_frame/1" do
    test "pops a frame from the call stack" do
      frame = %CallFrame{return_address: 7, saved_variables: %{"x" => 42}, saved_locals: ["y"]}
      vm = GenericVM.new() |> GenericVM.push_frame(frame)
      {popped, vm} = GenericVM.pop_frame(vm)
      assert popped == frame
      assert vm.call_stack == []
    end

    test "raises VMError on empty call stack" do
      vm = GenericVM.new()
      assert_raise Errors.VMError, fn ->
        GenericVM.pop_frame(vm)
      end
    end
  end

  # ===========================================================================
  # Program Counter Tests
  # ===========================================================================

  describe "advance_pc/1" do
    test "increments the program counter by one" do
      vm = GenericVM.new()
      assert vm.pc == 0
      vm = GenericVM.advance_pc(vm)
      assert vm.pc == 1
      vm = GenericVM.advance_pc(vm)
      assert vm.pc == 2
    end
  end

  describe "jump_to/2" do
    test "sets the program counter to a specific value" do
      vm = GenericVM.new()
      vm = GenericVM.jump_to(vm, 42)
      assert vm.pc == 42
    end

    test "can jump to zero" do
      vm = GenericVM.new() |> GenericVM.advance_pc() |> GenericVM.advance_pc()
      assert vm.pc == 2
      vm = GenericVM.jump_to(vm, 0)
      assert vm.pc == 0
    end
  end

  # ===========================================================================
  # Configuration Tests
  # ===========================================================================

  describe "set_max_recursion_depth/2" do
    test "sets the max recursion depth" do
      vm = GenericVM.new() |> GenericVM.set_max_recursion_depth(50)
      assert vm.max_recursion_depth == 50
    end

    test "can set to nil for no limit" do
      vm = GenericVM.new()
            |> GenericVM.set_max_recursion_depth(50)
            |> GenericVM.set_max_recursion_depth(nil)
      assert vm.max_recursion_depth == nil
    end
  end

  describe "set_frozen/2" do
    test "sets the frozen flag" do
      vm = GenericVM.new() |> GenericVM.set_frozen(true)
      assert vm.frozen == true
    end

    test "can unfreeze" do
      vm = GenericVM.new()
            |> GenericVM.set_frozen(true)
            |> GenericVM.set_frozen(false)
      assert vm.frozen == false
    end
  end

  # ===========================================================================
  # Execution Tests
  # ===========================================================================

  describe "execute/2" do
    test "executes a simple push-push-add-halt program" do
      vm = build_test_vm()
      code = %CodeObject{
        instructions: [
          %Instruction{opcode: @op_push, operand: 0},    # push 3
          %Instruction{opcode: @op_push, operand: 1},    # push 4
          %Instruction{opcode: @op_add, operand: nil},    # add -> 7
          %Instruction{opcode: @op_halt, operand: nil}    # halt
        ],
        constants: [3, 4]
      }

      {traces, final_vm} = GenericVM.execute(vm, code)
      assert length(traces) == 4
      assert final_vm.halted == true
      assert final_vm.stack == [7]
    end

    test "executes a push-print program with output" do
      vm = build_test_vm()
      code = %CodeObject{
        instructions: [
          %Instruction{opcode: @op_push, operand: 0},
          %Instruction{opcode: @op_print, operand: nil},
          %Instruction{opcode: @op_halt, operand: nil}
        ],
        constants: [42]
      }

      {traces, final_vm} = GenericVM.execute(vm, code)
      assert length(traces) == 3
      # The PRINT trace should have output
      print_trace = Enum.at(traces, 1)
      assert print_trace.output == "42"
      assert final_vm.stack == []
    end

    test "executes an empty program" do
      vm = build_test_vm()
      code = %CodeObject{instructions: [], constants: []}
      {traces, final_vm} = GenericVM.execute(vm, code)
      assert traces == []
      assert final_vm.pc == 0
    end

    test "stops at end of instructions when no HALT" do
      vm = build_test_vm()
      code = %CodeObject{
        instructions: [
          %Instruction{opcode: @op_push, operand: 0}
        ],
        constants: [99]
      }

      {traces, final_vm} = GenericVM.execute(vm, code)
      assert length(traces) == 1
      assert final_vm.stack == [99]
      assert final_vm.halted == false
    end
  end

  describe "step/2" do
    test "executes a single instruction and returns a trace" do
      vm = build_test_vm()
      code = %CodeObject{
        instructions: [
          %Instruction{opcode: @op_push, operand: 0},
          %Instruction{opcode: @op_halt, operand: nil}
        ],
        constants: [42]
      }

      {trace, vm} = GenericVM.step(vm, code)
      assert %VMTrace{} = trace
      assert trace.pc == 0
      assert trace.stack_before == []
      assert trace.stack_after == [42]
      assert trace.output == nil
      assert vm.pc == 1
      assert vm.stack == [42]
    end

    test "raises InvalidOpcodeError for unregistered opcode" do
      vm = GenericVM.new()  # No handlers registered
      code = %CodeObject{
        instructions: [%Instruction{opcode: 0x99, operand: nil}],
        constants: []
      }

      assert_raise Errors.InvalidOpcodeError, fn ->
        GenericVM.step(vm, code)
      end
    end
  end

  # ===========================================================================
  # Trace Tests
  # ===========================================================================

  describe "traces" do
    test "contain correct pc values" do
      vm = build_test_vm()
      code = %CodeObject{
        instructions: [
          %Instruction{opcode: @op_push, operand: 0},
          %Instruction{opcode: @op_push, operand: 1},
          %Instruction{opcode: @op_add, operand: nil},
          %Instruction{opcode: @op_halt, operand: nil}
        ],
        constants: [10, 20]
      }

      {traces, _vm} = GenericVM.execute(vm, code)
      pcs = Enum.map(traces, & &1.pc)
      assert pcs == [0, 1, 2, 3]
    end

    test "contain correct stack_before and stack_after" do
      vm = build_test_vm()
      code = %CodeObject{
        instructions: [
          %Instruction{opcode: @op_push, operand: 0},  # push 5
          %Instruction{opcode: @op_push, operand: 1},  # push 3
          %Instruction{opcode: @op_add, operand: nil}   # add -> 8
        ],
        constants: [5, 3]
      }

      {traces, _vm} = GenericVM.execute(vm, code)

      # After push 5: stack = [5]
      assert Enum.at(traces, 0).stack_before == []
      assert Enum.at(traces, 0).stack_after == [5]

      # After push 3: stack = [5, 3]
      assert Enum.at(traces, 1).stack_before == [5]
      assert Enum.at(traces, 1).stack_after == [5, 3]

      # After add: stack = [8]
      assert Enum.at(traces, 2).stack_before == [5, 3]
      assert Enum.at(traces, 2).stack_after == [8]
    end

    test "contain description strings" do
      vm = build_test_vm()
      code = %CodeObject{
        instructions: [
          %Instruction{opcode: @op_push, operand: 0},
          %Instruction{opcode: @op_add, operand: nil}
        ],
        constants: [1]
      }

      {traces, _vm} = GenericVM.execute(vm |> GenericVM.push(1), code)

      # OP_PUSH (0x01) with operand 0
      assert Enum.at(traces, 0).description =~ "0x01"
      assert Enum.at(traces, 0).description =~ "operand"

      # OP_ADD (0x02) without operand
      assert Enum.at(traces, 1).description =~ "0x02"
    end

    test "contain the instruction struct" do
      vm = build_test_vm()
      instr = %Instruction{opcode: @op_push, operand: 0}
      code = %CodeObject{instructions: [instr], constants: [7]}

      {[trace], _vm} = GenericVM.execute(vm, code)
      assert trace.instruction == instr
    end

    test "capture output from print-like instructions" do
      vm = build_test_vm()
      code = %CodeObject{
        instructions: [
          %Instruction{opcode: @op_push, operand: 0},
          %Instruction{opcode: @op_print, operand: nil}
        ],
        constants: ["hello"]
      }

      {traces, _vm} = GenericVM.execute(vm, code)
      push_trace = Enum.at(traces, 0)
      print_trace = Enum.at(traces, 1)
      assert push_trace.output == nil
      assert print_trace.output == "\"hello\""
    end
  end

  # ===========================================================================
  # Reset Tests
  # ===========================================================================

  describe "reset/1" do
    test "clears runtime state" do
      vm = build_test_vm()
            |> GenericVM.push(1)
            |> GenericVM.push(2)
            |> GenericVM.advance_pc()
            |> GenericVM.set_frozen(true)
            |> GenericVM.put_extra(:key, "value")

      vm = %{vm | variables: %{"x" => 42}, halted: true, output: ["test"]}
      frame = %CallFrame{return_address: 0, saved_variables: %{}, saved_locals: []}
      vm = GenericVM.push_frame(vm, frame)

      reset_vm = GenericVM.reset(vm)

      assert reset_vm.stack == []
      assert reset_vm.variables == %{}
      assert reset_vm.locals == []
      assert reset_vm.pc == 0
      assert reset_vm.halted == false
      assert reset_vm.output == []
      assert reset_vm.call_stack == []
      assert reset_vm.frozen == false
      assert reset_vm.extra == %{}
    end

    test "preserves registered handlers" do
      vm = build_test_vm() |> GenericVM.push(999)
      reset_vm = GenericVM.reset(vm)

      assert map_size(reset_vm.handlers) == 4
      assert Map.has_key?(reset_vm.handlers, @op_push)
    end

    test "preserves registered builtins" do
      impl = fn _args, vm -> {nil, vm} end
      vm = GenericVM.new()
            |> GenericVM.register_builtin("test_fn", impl)
            |> GenericVM.push(1)

      reset_vm = GenericVM.reset(vm)
      assert GenericVM.get_builtin(reset_vm, "test_fn") != nil
    end

    test "preserves max_recursion_depth" do
      vm = GenericVM.new()
            |> GenericVM.set_max_recursion_depth(50)
            |> GenericVM.push(1)

      reset_vm = GenericVM.reset(vm)
      assert reset_vm.max_recursion_depth == 50
    end
  end

  # ===========================================================================
  # Extra State Tests
  # ===========================================================================

  describe "put_extra/3 and get_extra/2,3" do
    test "stores and retrieves a value" do
      vm = GenericVM.new() |> GenericVM.put_extra(:tape, [0, 0, 0])
      assert GenericVM.get_extra(vm, :tape) == [0, 0, 0]
    end

    test "returns default for missing key" do
      vm = GenericVM.new()
      assert GenericVM.get_extra(vm, :missing) == nil
      assert GenericVM.get_extra(vm, :missing, :default) == :default
    end

    test "overwrites existing extra key" do
      vm = GenericVM.new()
            |> GenericVM.put_extra(:counter, 0)
            |> GenericVM.put_extra(:counter, 5)
      assert GenericVM.get_extra(vm, :counter) == 5
    end

    test "supports multiple keys" do
      vm = GenericVM.new()
            |> GenericVM.put_extra(:tape, [0, 0, 0])
            |> GenericVM.put_extra(:pointer, 0)
            |> GenericVM.put_extra(:input_buffer, "abc")
      assert GenericVM.get_extra(vm, :tape) == [0, 0, 0]
      assert GenericVM.get_extra(vm, :pointer) == 0
      assert GenericVM.get_extra(vm, :input_buffer) == "abc"
    end
  end

  # ===========================================================================
  # Integration Test — Full Program
  # ===========================================================================

  describe "integration" do
    test "computes 10 + 20 + 30 and prints the result" do
      vm = build_test_vm()
      code = %CodeObject{
        instructions: [
          %Instruction{opcode: @op_push, operand: 0},    # push 10
          %Instruction{opcode: @op_push, operand: 1},    # push 20
          %Instruction{opcode: @op_add, operand: nil},    # add -> 30
          %Instruction{opcode: @op_push, operand: 2},    # push 30
          %Instruction{opcode: @op_add, operand: nil},    # add -> 60
          %Instruction{opcode: @op_print, operand: nil},  # print 60
          %Instruction{opcode: @op_halt, operand: nil}    # halt
        ],
        constants: [10, 20, 30]
      }

      {traces, final_vm} = GenericVM.execute(vm, code)
      assert length(traces) == 7
      assert final_vm.halted == true
      assert final_vm.stack == []

      # The print instruction should have output "60"
      print_trace = Enum.at(traces, 5)
      assert print_trace.output == "60"
    end

    test "reset and re-execute with same handlers" do
      vm = build_test_vm()

      # First program: 1 + 2 = 3
      code1 = %CodeObject{
        instructions: [
          %Instruction{opcode: @op_push, operand: 0},
          %Instruction{opcode: @op_push, operand: 1},
          %Instruction{opcode: @op_add, operand: nil},
          %Instruction{opcode: @op_halt, operand: nil}
        ],
        constants: [1, 2]
      }

      {_traces, vm} = GenericVM.execute(vm, code1)
      assert vm.stack == [3]

      # Reset and run second program: 10 + 20 = 30
      vm = GenericVM.reset(vm)
      code2 = %CodeObject{
        instructions: [
          %Instruction{opcode: @op_push, operand: 0},
          %Instruction{opcode: @op_push, operand: 1},
          %Instruction{opcode: @op_add, operand: nil},
          %Instruction{opcode: @op_halt, operand: nil}
        ],
        constants: [10, 20]
      }

      {_traces, vm} = GenericVM.execute(vm, code2)
      assert vm.stack == [30]
    end

    test "step-by-step execution matches full execution" do
      vm = build_test_vm()
      code = %CodeObject{
        instructions: [
          %Instruction{opcode: @op_push, operand: 0},
          %Instruction{opcode: @op_push, operand: 1},
          %Instruction{opcode: @op_add, operand: nil}
        ],
        constants: [5, 7]
      }

      # Full execution
      {full_traces, full_vm} = GenericVM.execute(vm, code)

      # Step-by-step
      {t1, vm1} = GenericVM.step(vm, code)
      {t2, vm2} = GenericVM.step(vm1, code)
      {t3, vm3} = GenericVM.step(vm2, code)

      # Results should match
      assert full_vm.stack == vm3.stack
      assert full_vm.pc == vm3.pc
      assert length(full_traces) == 3
      assert Enum.at(full_traces, 0).stack_after == t1.stack_after
      assert Enum.at(full_traces, 1).stack_after == t2.stack_after
      assert Enum.at(full_traces, 2).stack_after == t3.stack_after
    end
  end

  # ===========================================================================
  # Error Handling Tests
  # ===========================================================================

  describe "error handling" do
    test "stack underflow during execution raises" do
      vm = build_test_vm()
      code = %CodeObject{
        instructions: [
          %Instruction{opcode: @op_add, operand: nil}  # add with empty stack
        ],
        constants: []
      }

      assert_raise Errors.StackUnderflowError, fn ->
        GenericVM.execute(vm, code)
      end
    end

    test "invalid opcode during execution raises" do
      vm = build_test_vm()
      code = %CodeObject{
        instructions: [
          %Instruction{opcode: 0x99, operand: nil}  # unregistered opcode
        ],
        constants: []
      }

      assert_raise Errors.InvalidOpcodeError, fn ->
        GenericVM.execute(vm, code)
      end
    end
  end

  # ===========================================================================
  # Variables Tests
  # ===========================================================================

  describe "variables" do
    test "handlers can set variables via struct update" do
      store_handler = fn vm, instr, code ->
        {value, vm} = GenericVM.pop(vm)
        name = Enum.at(code.names, instr.operand)
        vm = %{vm | variables: Map.put(vm.variables, name, value)}
        vm = GenericVM.advance_pc(vm)
        {nil, vm}
      end

      vm = GenericVM.new()
            |> GenericVM.register_opcode(@op_push, &handle_push/3)
            |> GenericVM.register_opcode(0x10, store_handler)

      code = %CodeObject{
        instructions: [
          %Instruction{opcode: @op_push, operand: 0},
          %Instruction{opcode: 0x10, operand: 0}
        ],
        constants: [42],
        names: ["x"]
      }

      {_traces, final_vm} = GenericVM.execute(vm, code)
      assert final_vm.variables == %{"x" => 42}
    end

    test "traces include current variables" do
      store_handler = fn vm, instr, code ->
        {value, vm} = GenericVM.pop(vm)
        name = Enum.at(code.names, instr.operand)
        vm = %{vm | variables: Map.put(vm.variables, name, value)}
        vm = GenericVM.advance_pc(vm)
        {nil, vm}
      end

      vm = GenericVM.new()
            |> GenericVM.register_opcode(@op_push, &handle_push/3)
            |> GenericVM.register_opcode(0x10, store_handler)

      code = %CodeObject{
        instructions: [
          %Instruction{opcode: @op_push, operand: 0},
          %Instruction{opcode: 0x10, operand: 0}
        ],
        constants: [99],
        names: ["y"]
      }

      {traces, _vm} = GenericVM.execute(vm, code)
      # After storing, the trace should show y = 99
      store_trace = Enum.at(traces, 1)
      assert store_trace.variables == %{"y" => 99}
    end

    test "inject_globals merges and overwrites variables" do
      vm =
        GenericVM.new()
        |> Map.put(:variables, %{"existing" => 1, "ctx_os" => "linux"})
        |> GenericVM.inject_globals(%{"ctx_os" => "darwin", "answer" => 42})

      assert vm.variables == %{
               "existing" => 1,
               "ctx_os" => "darwin",
               "answer" => 42
             }
    end
  end
end
