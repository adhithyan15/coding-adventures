defmodule CodingAdventures.ArmSimulatorTest do
  use ExUnit.Case

  alias CodingAdventures.ArmDecoder
  alias CodingAdventures.ArmExecutor
  alias CodingAdventures.ArmSimulator

  test "decoder understands mov immediate add sub and hlt" do
    decoded = ArmDecoder.decode(ArmSimulator.encode_mov_imm(0, 42), 0)
    assert decoded.mnemonic == "mov"
    assert decoded.fields.rd == 0
    assert decoded.fields.imm == 42

    decoded = ArmDecoder.decode(ArmSimulator.encode_add(2, 0, 1), 4)
    assert decoded.mnemonic == "add"
    assert decoded.fields.rn == 0
    assert decoded.fields.rm == 1

    decoded = ArmDecoder.decode(ArmSimulator.encode_sub(3, 1, 2), 8)
    assert decoded.mnemonic == "sub"
    assert decoded.fields.rd == 3

    decoded = ArmDecoder.decode(ArmSimulator.encode_hlt(), 12)
    assert decoded.mnemonic == "hlt"
  end

  test "executor updates registers for mov add and sub" do
    registers = Enum.into(0..15, %{}, fn reg -> {reg, 0} end)

    {registers, mov_result} =
      ArmExecutor.execute(ArmDecoder.decode(ArmSimulator.encode_mov_imm(0, 5), 0), registers, %{}, 0)

    assert registers[0] == 5
    assert mov_result.next_pc == 4

    registers = Map.put(registers, 1, 2)

    {registers, add_result} =
      ArmExecutor.execute(ArmDecoder.decode(ArmSimulator.encode_add(2, 0, 1), 4), registers, %{}, 4)

    assert registers[2] == 7
    assert add_result.description =~ "R2 = R0(5) + R1(2)"

    {registers, sub_result} =
      ArmExecutor.execute(ArmDecoder.decode(ArmSimulator.encode_sub(3, 2, 1), 8), registers, %{}, 8)

    assert registers[3] == 5
    assert sub_result.description =~ "R3 = R2(7) - R1(2)"
  end

  test "simulator runs x equals 1 plus 2 and halts" do
    program =
      ArmSimulator.assemble([
        ArmSimulator.encode_mov_imm(0, 1),
        ArmSimulator.encode_mov_imm(1, 2),
        ArmSimulator.encode_add(2, 0, 1),
        ArmSimulator.encode_hlt()
      ])

    {sim, traces} = ArmSimulator.new() |> ArmSimulator.run(program)
    assert length(traces) == 4
    assert sim.registers[0] == 1
    assert sim.registers[1] == 2
    assert sim.registers[2] == 3
    assert sim.halted
  end

  test "simulator can step and then refuses after halt" do
    sim =
      ArmSimulator.new()
      |> ArmSimulator.load_program([
        ArmSimulator.encode_hlt()
      ])

    {sim, trace} = ArmSimulator.step(sim)
    assert trace.decoded.mnemonic == "hlt"
    assert sim.halted
    assert_raise RuntimeError, ~r/has halted/, fn -> ArmSimulator.step(sim) end
  end

  test "simulator raises when pc moves past end of program" do
    sim = ArmSimulator.new() |> ArmSimulator.load_program([])
    assert_raise RuntimeError, ~r/past end of program/, fn -> ArmSimulator.step(sim) end
  end

  test "encoding helpers produce the expected ARM words" do
    assert ArmSimulator.encode_mov_imm(0, 42) == 0xE3A0002A
    assert ArmSimulator.encode_add(2, 0, 1) == 0xE0802001
    assert ArmSimulator.encode_sub(3, 1, 2) == 0xE0413002
    assert ArmSimulator.encode_hlt() == 0xFFFFFFFF
  end
end
