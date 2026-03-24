defmodule CodingAdventures.ClrSimulatorTest do
  use ExUnit.Case

  alias CodingAdventures.ClrSimulator

  test "encoding helpers choose the expected instruction widths" do
    assert ClrSimulator.encode_ldc_i4(0) == <<0x16>>
    assert ClrSimulator.encode_ldc_i4(5) == <<0x1B>>
    assert ClrSimulator.encode_ldc_i4(8) == <<0x1E>>
    assert byte_size(ClrSimulator.encode_ldc_i4(42)) == 2
    assert byte_size(ClrSimulator.encode_ldc_i4(-1)) == 2
    assert byte_size(ClrSimulator.encode_ldc_i4(1000)) == 5
    assert ClrSimulator.encode_stloc(0) == <<0x0A>>
    assert ClrSimulator.encode_stloc(10) == <<0x13, 10>>
    assert ClrSimulator.encode_ldloc(3) == <<0x09>>
    assert ClrSimulator.encode_ldloc(10) == <<0x11, 10>>
  end

  test "assemble_clr builds binaries from raw bytes and encoded helpers" do
    bytecode = ClrSimulator.assemble_clr([ClrSimulator.encode_ldc_i4(1), [ClrSimulator.ret()]])
    assert bytecode == <<0x17, 0x2A>>
  end

  test "simulator can add store and halt" do
    sim =
      ClrSimulator.new()
      |> ClrSimulator.load(
        ClrSimulator.assemble_clr([
          ClrSimulator.encode_ldc_i4(1),
          ClrSimulator.encode_ldc_i4(2),
          [ClrSimulator.add_op()],
          ClrSimulator.encode_stloc(0),
          [ClrSimulator.ret()]
        ])
      )

    {sim, traces} = ClrSimulator.run(sim)
    assert length(traces) == 5
    assert Enum.at(sim.locals, 0) == 3
    assert sim.halted
  end

  test "generic ldloc and stloc work for higher slots" do
    sim =
      ClrSimulator.new()
      |> ClrSimulator.load(
        ClrSimulator.assemble_clr([
          ClrSimulator.encode_ldc_i4(7),
          ClrSimulator.encode_stloc(5),
          ClrSimulator.encode_ldloc(5),
          ClrSimulator.encode_stloc(0),
          [ClrSimulator.ret()]
        ])
      )

    {sim, _traces} = ClrSimulator.run(sim)
    assert Enum.at(sim.locals, 0) == 7
    assert Enum.at(sim.locals, 5) == 7
  end

  test "arithmetic instructions and div truncation behave as expected" do
    sim =
      ClrSimulator.new()
      |> ClrSimulator.load(
        ClrSimulator.assemble_clr([
          ClrSimulator.encode_ldc_i4(5),
          ClrSimulator.encode_ldc_i4(3),
          [ClrSimulator.sub_op()],
          ClrSimulator.encode_ldc_i4(3),
          [ClrSimulator.mul_op()],
          ClrSimulator.encode_ldc_i4(2),
          [ClrSimulator.div_op()],
          ClrSimulator.encode_stloc(0),
          [ClrSimulator.ret()]
        ])
      )

    {sim, _traces} = ClrSimulator.run(sim)
    assert Enum.at(sim.locals, 0) == 3
  end

  test "division by zero raises" do
    sim =
      ClrSimulator.new()
      |> ClrSimulator.load(
        ClrSimulator.assemble_clr([
          ClrSimulator.encode_ldc_i4(5),
          ClrSimulator.encode_ldc_i4(0),
          [ClrSimulator.div_op()],
          [ClrSimulator.ret()]
        ])
      )

    assert_raise ArithmeticError, ~r/division by zero/, fn -> ClrSimulator.run(sim) end
  end

  test "branches handle taken and not-taken paths" do
    sim =
      ClrSimulator.new()
      |> ClrSimulator.load(
        ClrSimulator.assemble_clr([
          [ClrSimulator.br_s(), 2],
          ClrSimulator.encode_ldc_i4(5),
          ClrSimulator.encode_stloc(0),
          [ClrSimulator.ret()]
        ])
      )

    {sim, _traces} = ClrSimulator.run(sim)
    assert Enum.at(sim.locals, 0) == nil

    sim =
      ClrSimulator.new()
      |> ClrSimulator.load(
        ClrSimulator.assemble_clr([
          ClrSimulator.encode_ldc_i4(0),
          [ClrSimulator.brfalse_s(), 2],
          ClrSimulator.encode_ldc_i4(5),
          ClrSimulator.encode_stloc(0),
          [ClrSimulator.ret()]
        ])
      )

    {sim, _traces} = ClrSimulator.run(sim)
    assert Enum.at(sim.locals, 0) == nil

    sim =
      ClrSimulator.new()
      |> ClrSimulator.load(
        ClrSimulator.assemble_clr([
          ClrSimulator.encode_ldc_i4(1),
          [ClrSimulator.brtrue_s(), 2],
          ClrSimulator.encode_ldc_i4(5),
          ClrSimulator.encode_stloc(0),
          [ClrSimulator.ret()]
        ])
      )

    {sim, _traces} = ClrSimulator.run(sim)
    assert Enum.at(sim.locals, 0) == nil
  end

  test "ldnull and comparisons work including two-byte opcodes" do
    sim =
      ClrSimulator.new()
      |> ClrSimulator.load(
        ClrSimulator.assemble_clr([
          [ClrSimulator.ldnull()],
          [ClrSimulator.brfalse_s(), 2],
          ClrSimulator.encode_ldc_i4(5),
          ClrSimulator.encode_stloc(0),
          [ClrSimulator.ret()]
        ])
      )

    {sim, _traces} = ClrSimulator.run(sim)
    assert Enum.at(sim.locals, 0) == nil

    sim =
      ClrSimulator.new()
      |> ClrSimulator.load(
        ClrSimulator.assemble_clr([
          ClrSimulator.encode_ldc_i4(3),
          ClrSimulator.encode_ldc_i4(3),
          [ClrSimulator.prefix_fe(), ClrSimulator.ceq_byte()],
          ClrSimulator.encode_stloc(0),
          [ClrSimulator.ret()]
        ])
      )

    {sim, _traces} = ClrSimulator.run(sim)
    assert Enum.at(sim.locals, 0) == 1
  end

  test "step returns detailed trace and invalid bytecode raises" do
    sim =
      ClrSimulator.new()
      |> ClrSimulator.load(
        ClrSimulator.assemble_clr([
          ClrSimulator.encode_ldc_i4(1),
          [ClrSimulator.ret()]
        ])
      )

    {sim, trace} = ClrSimulator.step(sim)
    assert trace.pc == 0
    assert trace.opcode == "ldc.i4.1"
    assert trace.stack_before == []
    assert trace.stack_after == [1]

    {sim, _trace} = ClrSimulator.step(sim)
    assert sim.halted
    assert_raise RuntimeError, ~r/has halted/, fn -> ClrSimulator.step(sim) end

    assert_raise RuntimeError, ~r/beyond end of bytecode/, fn ->
      ClrSimulator.new() |> ClrSimulator.load(<<>>) |> ClrSimulator.step()
    end

    assert_raise ArgumentError, ~r/Unknown CLR opcode/, fn ->
      ClrSimulator.new() |> ClrSimulator.load(<<0xFF>>) |> ClrSimulator.step()
    end

    assert_raise ArgumentError, ~r/Incomplete two-byte opcode/, fn ->
      ClrSimulator.new() |> ClrSimulator.load(<<ClrSimulator.prefix_fe()>>) |> ClrSimulator.step()
    end
  end
end
