defmodule CodingAdventures.Brainfuck.HandlersTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.VirtualMachine.GenericVM
  alias CodingAdventures.VirtualMachine.Types.{Instruction, CodeObject}
  alias CodingAdventures.Brainfuck.{Handlers, Opcodes}
  alias CodingAdventures.Brainfuck.Handlers.BrainfuckError

  # =========================================================================
  # Test helpers
  # =========================================================================
  # Each handler test creates a minimal VM with Brainfuck state in `extra`,
  # then calls the handler directly and checks the result.

  defp make_vm(opts \\ []) do
    tape = Keyword.get(opts, :tape, List.duplicate(0, Handlers.tape_size()))
    dp = Keyword.get(opts, :dp, 0)
    input_buffer = Keyword.get(opts, :input_buffer, "")
    input_pos = Keyword.get(opts, :input_pos, 0)

    GenericVM.new()
    |> GenericVM.put_extra(:tape, tape)
    |> GenericVM.put_extra(:dp, dp)
    |> GenericVM.put_extra(:input_buffer, input_buffer)
    |> GenericVM.put_extra(:input_pos, input_pos)
  end

  defp dummy_instr(opcode \\ 0x00, operand \\ nil) do
    %Instruction{opcode: opcode, operand: operand}
  end

  defp dummy_code do
    %CodeObject{instructions: [], constants: [], names: []}
  end

  # =========================================================================
  # Pointer movement: > and <
  # =========================================================================

  describe "handle_right/3" do
    test "moves data pointer right" do
      vm = make_vm(dp: 0)
      {output, vm} = Handlers.handle_right(vm, dummy_instr(), dummy_code())
      assert output == nil
      assert GenericVM.get_extra(vm, :dp) == 1
    end

    test "advances program counter" do
      vm = make_vm(dp: 0)
      {_output, vm} = Handlers.handle_right(vm, dummy_instr(), dummy_code())
      assert vm.pc == 1
    end

    test "raises when pointer goes past end of tape" do
      vm = make_vm(dp: Handlers.tape_size() - 1)
      assert_raise BrainfuckError, ~r/past end of tape/, fn ->
        Handlers.handle_right(vm, dummy_instr(), dummy_code())
      end
    end
  end

  describe "handle_left/3" do
    test "moves data pointer left" do
      vm = make_vm(dp: 5)
      {output, vm} = Handlers.handle_left(vm, dummy_instr(), dummy_code())
      assert output == nil
      assert GenericVM.get_extra(vm, :dp) == 4
    end

    test "advances program counter" do
      vm = make_vm(dp: 5)
      {_output, vm} = Handlers.handle_left(vm, dummy_instr(), dummy_code())
      assert vm.pc == 1
    end

    test "raises when pointer goes before start of tape" do
      vm = make_vm(dp: 0)
      assert_raise BrainfuckError, ~r/before start of tape/, fn ->
        Handlers.handle_left(vm, dummy_instr(), dummy_code())
      end
    end
  end

  # =========================================================================
  # Cell modification: + and -
  # =========================================================================

  describe "handle_inc/3" do
    test "increments cell value" do
      vm = make_vm()
      {output, vm} = Handlers.handle_inc(vm, dummy_instr(), dummy_code())
      assert output == nil
      assert Enum.at(GenericVM.get_extra(vm, :tape), 0) == 1
    end

    test "wraps from 255 to 0" do
      tape = List.replace_at(List.duplicate(0, Handlers.tape_size()), 0, 255)
      vm = make_vm(tape: tape)
      {_output, vm} = Handlers.handle_inc(vm, dummy_instr(), dummy_code())
      assert Enum.at(GenericVM.get_extra(vm, :tape), 0) == 0
    end

    test "advances program counter" do
      vm = make_vm()
      {_output, vm} = Handlers.handle_inc(vm, dummy_instr(), dummy_code())
      assert vm.pc == 1
    end
  end

  describe "handle_dec/3" do
    test "decrements cell value" do
      tape = List.replace_at(List.duplicate(0, Handlers.tape_size()), 0, 5)
      vm = make_vm(tape: tape)
      {output, vm} = Handlers.handle_dec(vm, dummy_instr(), dummy_code())
      assert output == nil
      assert Enum.at(GenericVM.get_extra(vm, :tape), 0) == 4
    end

    test "wraps from 0 to 255" do
      vm = make_vm()
      {_output, vm} = Handlers.handle_dec(vm, dummy_instr(), dummy_code())
      assert Enum.at(GenericVM.get_extra(vm, :tape), 0) == 255
    end

    test "advances program counter" do
      vm = make_vm()
      {_output, vm} = Handlers.handle_dec(vm, dummy_instr(), dummy_code())
      assert vm.pc == 1
    end
  end

  # =========================================================================
  # I/O: . and ,
  # =========================================================================

  describe "handle_output/3" do
    test "outputs cell value as ASCII character" do
      tape = List.replace_at(List.duplicate(0, Handlers.tape_size()), 0, 65)
      vm = make_vm(tape: tape)
      {output, vm} = Handlers.handle_output(vm, dummy_instr(), dummy_code())
      assert output == "A"
      assert vm.output == ["A"]
    end

    test "advances program counter" do
      tape = List.replace_at(List.duplicate(0, Handlers.tape_size()), 0, 65)
      vm = make_vm(tape: tape)
      {_output, vm} = Handlers.handle_output(vm, dummy_instr(), dummy_code())
      assert vm.pc == 1
    end

    test "appends to existing output" do
      tape = List.replace_at(List.duplicate(0, Handlers.tape_size()), 0, 66)
      vm = make_vm(tape: tape)
      vm = %{vm | output: ["A"]}
      {output, vm} = Handlers.handle_output(vm, dummy_instr(), dummy_code())
      assert output == "B"
      assert vm.output == ["A", "B"]
    end
  end

  describe "handle_input/3" do
    test "reads byte from input buffer" do
      vm = make_vm(input_buffer: "AB", input_pos: 0)
      {output, vm} = Handlers.handle_input(vm, dummy_instr(), dummy_code())
      assert output == nil
      assert Enum.at(GenericVM.get_extra(vm, :tape), 0) == ?A
      assert GenericVM.get_extra(vm, :input_pos) == 1
    end

    test "reads second byte on second call" do
      vm = make_vm(input_buffer: "AB", input_pos: 1)
      {_output, vm} = Handlers.handle_input(vm, dummy_instr(), dummy_code())
      assert Enum.at(GenericVM.get_extra(vm, :tape), 0) == ?B
      assert GenericVM.get_extra(vm, :input_pos) == 2
    end

    test "sets cell to 0 on EOF" do
      tape = List.replace_at(List.duplicate(0, Handlers.tape_size()), 0, 42)
      vm = make_vm(tape: tape, input_buffer: "", input_pos: 0)
      {_output, vm} = Handlers.handle_input(vm, dummy_instr(), dummy_code())
      assert Enum.at(GenericVM.get_extra(vm, :tape), 0) == 0
    end

    test "advances program counter" do
      vm = make_vm(input_buffer: "A", input_pos: 0)
      {_output, vm} = Handlers.handle_input(vm, dummy_instr(), dummy_code())
      assert vm.pc == 1
    end
  end

  # =========================================================================
  # Control flow: [ and ]
  # =========================================================================

  describe "handle_loop_start/3" do
    test "enters loop when cell is nonzero" do
      tape = List.replace_at(List.duplicate(0, Handlers.tape_size()), 0, 1)
      vm = make_vm(tape: tape)
      instr = %Instruction{opcode: Opcodes.loop_start(), operand: 10}
      {output, vm} = Handlers.handle_loop_start(vm, instr, dummy_code())
      assert output == nil
      assert vm.pc == 1  # advance to next instruction (enter loop)
    end

    test "skips loop when cell is zero" do
      vm = make_vm()  # cell 0 is 0
      instr = %Instruction{opcode: Opcodes.loop_start(), operand: 10}
      {output, vm} = Handlers.handle_loop_start(vm, instr, dummy_code())
      assert output == nil
      assert vm.pc == 10  # jump to operand (past loop end)
    end
  end

  describe "handle_loop_end/3" do
    test "loops back when cell is nonzero" do
      tape = List.replace_at(List.duplicate(0, Handlers.tape_size()), 0, 1)
      vm = make_vm(tape: tape)
      instr = %Instruction{opcode: Opcodes.loop_end(), operand: 3}
      {output, vm} = Handlers.handle_loop_end(vm, instr, dummy_code())
      assert output == nil
      assert vm.pc == 3  # jump back to matching loop start
    end

    test "exits loop when cell is zero" do
      vm = make_vm()  # cell 0 is 0
      instr = %Instruction{opcode: Opcodes.loop_end(), operand: 3}
      {output, vm} = Handlers.handle_loop_end(vm, instr, dummy_code())
      assert output == nil
      assert vm.pc == 1  # advance past loop end
    end
  end

  # =========================================================================
  # HALT
  # =========================================================================

  describe "handle_halt/3" do
    test "sets halted to true" do
      vm = make_vm()
      {output, vm} = Handlers.handle_halt(vm, dummy_instr(), dummy_code())
      assert output == nil
      assert vm.halted == true
    end
  end

  # =========================================================================
  # Handler registry
  # =========================================================================

  describe "handlers/0" do
    test "returns a map with 9 handlers" do
      handlers = Handlers.handlers()
      assert map_size(handlers) == 9
    end

    test "all values are functions of arity 3" do
      handlers = Handlers.handlers()
      assert Enum.all?(handlers, fn {_opcode, handler} ->
        is_function(handler, 3)
      end)
    end

    test "includes all expected opcodes" do
      handlers = Handlers.handlers()
      expected = [
        Opcodes.right(), Opcodes.left(),
        Opcodes.inc(), Opcodes.dec(),
        Opcodes.output_op(), Opcodes.input_op(),
        Opcodes.loop_start(), Opcodes.loop_end(),
        Opcodes.halt()
      ]
      assert MapSet.new(Map.keys(handlers)) == MapSet.new(expected)
    end
  end
end
