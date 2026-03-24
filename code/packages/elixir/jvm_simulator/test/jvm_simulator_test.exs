defmodule CodingAdventures.JvmSimulatorTest do
  use ExUnit.Case

  alias CodingAdventures.JvmSimulator

  test "encoding helpers cover constants loads stores and assembly" do
    assert JvmSimulator.encode_iconst(0) == <<0x03>>
    assert JvmSimulator.encode_iconst(1) == <<0x04>>
    assert JvmSimulator.encode_iconst(5) == <<0x08>>
    assert JvmSimulator.encode_iconst(42) == <<0x10, 42>>
    assert byte_size(JvmSimulator.encode_iconst(-1)) == 2
    assert_raise ArgumentError, fn -> JvmSimulator.encode_iconst(200) end

    assert JvmSimulator.encode_istore(0) == <<0x3B>>
    assert JvmSimulator.encode_istore(5) == <<0x36, 5>>
    assert JvmSimulator.encode_iload(0) == <<0x1A>>
    assert JvmSimulator.encode_iload(5) == <<0x15, 5>>

    bytecode =
      JvmSimulator.assemble_jvm([
        [JvmSimulator.iconst_1()],
        [JvmSimulator.iconst_2()],
        [JvmSimulator.iadd()],
        [JvmSimulator.istore_0()],
        [JvmSimulator.return_op()]
      ])

    assert bytecode == <<0x04, 0x05, 0x60, 0x3B, 0xB1>>
  end

  test "simulator can execute x equals 1 plus 2" do
    sim =
      JvmSimulator.new()
      |> JvmSimulator.load(
        JvmSimulator.assemble_jvm([
          [JvmSimulator.iconst_1()],
          [JvmSimulator.iconst_2()],
          [JvmSimulator.iadd()],
          [JvmSimulator.istore_0()],
          [JvmSimulator.return_op()]
        ])
      )

    {sim, traces} = JvmSimulator.run(sim)
    assert length(traces) == 5
    assert Enum.at(sim.locals, 0) == 3
    assert sim.halted
  end

  test "bipush ldc iload and istore work" do
    sim =
      JvmSimulator.new()
      |> JvmSimulator.load(
        JvmSimulator.assemble_jvm([
          [JvmSimulator.bipush(), 42],
          [JvmSimulator.istore_0()],
          [JvmSimulator.ldc(), 0],
          [JvmSimulator.istore(), 5],
          [JvmSimulator.iload(), 5],
          [JvmSimulator.istore_0()],
          [JvmSimulator.return_op()]
        ]),
        constants: [999]
      )

    {sim, _traces} = JvmSimulator.run(sim)
    assert Enum.at(sim.locals, 0) == 999
    assert Enum.at(sim.locals, 5) == 999
  end

  test "integer arithmetic and i32 overflow behave correctly" do
    sim =
      JvmSimulator.new()
      |> JvmSimulator.load(
        JvmSimulator.assemble_jvm([
          [JvmSimulator.iconst_5()],
          [JvmSimulator.iconst_3()],
          [JvmSimulator.isub()],
          [JvmSimulator.iconst_4()],
          [JvmSimulator.imul()],
          [JvmSimulator.iconst_2()],
          [JvmSimulator.idiv()],
          [JvmSimulator.ireturn()]
        ])
      )

    {sim, _traces} = JvmSimulator.run(sim)
    assert sim.return_value == 4

    sim =
      JvmSimulator.new()
      |> JvmSimulator.load(
        JvmSimulator.assemble_jvm([
          [JvmSimulator.ldc(), 0],
          [JvmSimulator.iconst_1()],
          [JvmSimulator.iadd()],
          [JvmSimulator.ireturn()]
        ]),
        constants: [2_147_483_647]
      )

    {sim, _traces} = JvmSimulator.run(sim)
    assert sim.return_value == -2_147_483_648
  end

  test "goto and comparison branches take and skip paths" do
    sim =
      JvmSimulator.new()
      |> JvmSimulator.load(
        JvmSimulator.assemble_jvm([
          [JvmSimulator.goto_op(), 5],
          [JvmSimulator.iconst_1()],
          [JvmSimulator.istore_0()],
          [JvmSimulator.return_op()]
        ])
      )

    {sim, _traces} = JvmSimulator.run(sim)
    assert Enum.at(sim.locals, 0) == nil

    sim =
      JvmSimulator.new()
      |> JvmSimulator.load(
        JvmSimulator.assemble_jvm([
          [JvmSimulator.iconst_1()],
          [JvmSimulator.iconst_1()],
          [JvmSimulator.if_icmpeq(), 5],
          [JvmSimulator.iconst_5()],
          [JvmSimulator.istore_0()],
          [JvmSimulator.return_op()]
        ])
      )

    {sim, _traces} = JvmSimulator.run(sim)
    assert Enum.at(sim.locals, 0) == nil

    sim =
      JvmSimulator.new()
      |> JvmSimulator.load(
        JvmSimulator.assemble_jvm([
          [JvmSimulator.iconst_1()],
          [JvmSimulator.iconst_5()],
          [JvmSimulator.if_icmpgt(), 5],
          [JvmSimulator.iconst_3()],
          [JvmSimulator.istore_0()],
          [JvmSimulator.return_op()]
        ])
      )

    {sim, _traces} = JvmSimulator.run(sim)
    assert Enum.at(sim.locals, 0) == 3
  end

  test "ireturn captures a return value and shortcuts work for locals" do
    sim =
      JvmSimulator.new()
      |> JvmSimulator.load(
        JvmSimulator.assemble_jvm([
          [JvmSimulator.iconst_1()],
          [JvmSimulator.istore_0()],
          [JvmSimulator.iconst_2()],
          [JvmSimulator.istore_0() + 1],
          [JvmSimulator.iconst_3()],
          [JvmSimulator.istore_0() + 2],
          [JvmSimulator.iconst_4()],
          [JvmSimulator.istore_0() + 3],
          [JvmSimulator.iload_0()],
          [JvmSimulator.iload_0() + 1],
          [JvmSimulator.iload_0() + 2],
          [JvmSimulator.iload_0() + 3],
          [JvmSimulator.ireturn()]
        ])
      )

    {sim, _traces} = JvmSimulator.run(sim)
    assert sim.return_value == 4
    assert sim.stack == [1, 2, 3]
  end

  test "step returns traces and invalid programs raise helpful errors" do
    sim =
      JvmSimulator.new()
      |> JvmSimulator.load(JvmSimulator.assemble_jvm([[JvmSimulator.iconst_1()], [JvmSimulator.return_op()]]))

    {sim, trace} = JvmSimulator.step(sim)
    assert trace.pc == 0
    assert trace.opcode == "iconst_1"
    assert trace.stack_before == []
    assert trace.stack_after == [1]

    {sim, _trace} = JvmSimulator.step(sim)
    assert sim.halted
    assert_raise RuntimeError, ~r/has halted/, fn -> JvmSimulator.step(sim) end
    assert_raise RuntimeError, ~r/past end of bytecode/, fn -> JvmSimulator.new() |> JvmSimulator.load(<<>>) |> JvmSimulator.step() end
    assert_raise RuntimeError, ~r/Unknown JVM opcode/, fn -> JvmSimulator.new() |> JvmSimulator.load(<<0xFF>>) |> JvmSimulator.step() end
  end

  test "error cases cover constant pool stack underflow and division by zero" do
    assert_raise RuntimeError, ~r/Constant pool index 5 out of range/, fn ->
      JvmSimulator.new()
      |> JvmSimulator.load(JvmSimulator.assemble_jvm([[JvmSimulator.ldc(), 5], [JvmSimulator.return_op()]]), constants: [1])
      |> JvmSimulator.run()
    end

    assert_raise RuntimeError, ~r/not an integer/, fn ->
      JvmSimulator.new()
      |> JvmSimulator.load(JvmSimulator.assemble_jvm([[JvmSimulator.ldc(), 0], [JvmSimulator.return_op()]]), constants: ["hello"])
      |> JvmSimulator.run()
    end

    assert_raise RuntimeError, ~r/has not been initialized/, fn ->
      JvmSimulator.new()
      |> JvmSimulator.load(JvmSimulator.assemble_jvm([[JvmSimulator.iload_0()], [JvmSimulator.return_op()]]))
      |> JvmSimulator.run()
    end

    assert_raise RuntimeError, ~r/division by zero/, fn ->
      JvmSimulator.new()
      |> JvmSimulator.load(JvmSimulator.assemble_jvm([[JvmSimulator.iconst_5()], [JvmSimulator.iconst_0()], [JvmSimulator.idiv()], [JvmSimulator.return_op()]]))
      |> JvmSimulator.run()
    end

    assert_raise RuntimeError, ~r/Stack underflow: iadd requires 2 operands/, fn ->
      JvmSimulator.new()
      |> JvmSimulator.load(JvmSimulator.assemble_jvm([[JvmSimulator.iconst_1()], [JvmSimulator.iadd()]]))
      |> JvmSimulator.run()
    end

    assert_raise RuntimeError, ~r/Stack underflow: istore_0 requires 1 operand/, fn ->
      JvmSimulator.new()
      |> JvmSimulator.load(JvmSimulator.assemble_jvm([[JvmSimulator.istore_0()]]))
      |> JvmSimulator.run()
    end
  end
end
