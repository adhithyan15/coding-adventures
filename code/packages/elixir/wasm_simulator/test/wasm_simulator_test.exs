defmodule CodingAdventures.WasmSimulatorTest do
  use ExUnit.Case

  alias CodingAdventures.WasmDecoder
  alias CodingAdventures.WasmExecutor
  alias CodingAdventures.WasmSimulator

  test "encoding helpers assemble a simple program" do
    bytecode =
      WasmSimulator.assemble_wasm([
        WasmSimulator.encode_i32_const(1),
        WasmSimulator.encode_i32_const(2),
        WasmSimulator.encode_i32_add(),
        WasmSimulator.encode_local_set(0),
        WasmSimulator.encode_end()
      ])

    assert is_binary(bytecode)
    assert byte_size(bytecode) == 14
  end

  test "decoder returns structured instructions" do
    bytecode =
      WasmSimulator.assemble_wasm([
        WasmSimulator.encode_i32_const(42),
        WasmSimulator.encode_local_get(3),
        WasmSimulator.encode_end()
      ])

    assert WasmDecoder.decode(bytecode, 0).mnemonic == "i32.const"
    assert WasmDecoder.decode(bytecode, 0).operand == 42
    assert WasmDecoder.decode(bytecode, 5).mnemonic == "local.get"
    assert WasmDecoder.decode(bytecode, 7).mnemonic == "end"
  end

  test "simulator runs x equals 1 plus 2 and stores in local 0" do
    program =
      WasmSimulator.assemble_wasm([
        WasmSimulator.encode_i32_const(1),
        WasmSimulator.encode_i32_const(2),
        WasmSimulator.encode_i32_add(),
        WasmSimulator.encode_local_set(0),
        WasmSimulator.encode_end()
      ])

    {sim, traces} = WasmSimulator.new() |> WasmSimulator.run(program)
    assert length(traces) == 5
    assert Enum.at(sim.locals, 0) == 3
    assert sim.halted
    assert sim.cycle == 5
  end

  test "local.get reloads values onto the stack" do
    program =
      WasmSimulator.assemble_wasm([
        WasmSimulator.encode_i32_const(7),
        WasmSimulator.encode_local_set(1),
        WasmSimulator.encode_local_get(1),
        WasmSimulator.encode_local_set(0),
        WasmSimulator.encode_end()
      ])

    {sim, _traces} = WasmSimulator.new(num_locals: 3) |> WasmSimulator.run(program)
    assert Enum.at(sim.locals, 0) == 7
    assert Enum.at(sim.locals, 1) == 7
  end

  test "i32.sub and overflow semantics are preserved" do
    program =
      WasmSimulator.assemble_wasm([
        WasmSimulator.encode_i32_const(1),
        WasmSimulator.encode_i32_const(2),
        WasmSimulator.encode_i32_sub(),
        WasmSimulator.encode_local_set(0),
        WasmSimulator.encode_end()
      ])

    {sim, _traces} = WasmSimulator.new() |> WasmSimulator.run(program)
    assert Enum.at(sim.locals, 0) == -1

    program =
      WasmSimulator.assemble_wasm([
        WasmSimulator.encode_i32_const(2_147_483_647),
        WasmSimulator.encode_i32_const(1),
        WasmSimulator.encode_i32_add(),
        WasmSimulator.encode_local_set(0),
        WasmSimulator.encode_end()
      ])

    {sim, _traces} = WasmSimulator.new() |> WasmSimulator.run(program)
    assert Enum.at(sim.locals, 0) == -2_147_483_648
  end

  test "executor produces descriptive traces and halt flag" do
    instruction = WasmDecoder.decode(WasmSimulator.encode_i32_const(5), 0)
    trace = WasmExecutor.execute(instruction, [], [0, 0], 0)
    assert trace.description == "push 5"
    assert trace.stack_after == [5]

    halt = WasmDecoder.decode(WasmSimulator.encode_end(), 0)
    trace = WasmExecutor.execute(halt, [1], [0, 0], 0)
    assert trace.halted
    assert trace.description == "halt"
  end

  test "step after halt and invalid bytecode raise helpful errors" do
    program = WasmSimulator.assemble_wasm([WasmSimulator.encode_end()])
    {sim, _traces} = WasmSimulator.new() |> WasmSimulator.run(program)
    assert_raise RuntimeError, ~r/has halted/, fn -> WasmSimulator.step(sim) end

    assert_raise ArgumentError, ~r/Unknown WASM opcode/, fn ->
      WasmDecoder.decode(<<0xFF>>, 0)
    end
  end

  test "stack underflow surfaces for invalid programs" do
    add_instruction = WasmDecoder.decode(WasmSimulator.encode_i32_add(), 0)
    assert_raise RuntimeError, ~r/requires 2 operands/, fn ->
      WasmExecutor.execute(add_instruction, [1], [0, 0], 0)
    end

    set_instruction = WasmDecoder.decode(WasmSimulator.encode_local_set(0), 0)
    assert_raise RuntimeError, ~r/requires 1 operand/, fn ->
      WasmExecutor.execute(set_instruction, [], [0, 0], 0)
    end
  end
end
