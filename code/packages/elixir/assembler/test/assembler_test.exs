defmodule CodingAdventures.AssemblerTest do
  use ExUnit.Case

  alias CodingAdventures.ArmInstruction
  alias CodingAdventures.ArmOpcode
  alias CodingAdventures.Assembler
  alias CodingAdventures.Operand2

  test "parse_register accepts named and numbered ARM registers" do
    assert Assembler.parse_register("R0") == 0
    assert Assembler.parse_register("R15") == 15
    assert Assembler.parse_register("SP") == 13
    assert Assembler.parse_register("LR") == 14
    assert Assembler.parse_register("pc") == 15
    assert Assembler.parse_register("R16") == nil
    assert Assembler.parse_register("X0") == nil
  end

  test "parse_immediate supports decimal hexadecimal and bare numbers" do
    assert Assembler.parse_immediate("#42") == 42
    assert Assembler.parse_immediate("#0xFF") == 255
    assert Assembler.parse_immediate("42") == 42
    assert Assembler.parse_immediate("#-1") == nil
  end

  test "parse can read mov add cmp ldr str nop and labels" do
    {:ok, assembler, instructions} =
      Assembler.new()
      |> Assembler.parse("""
      loop:
      MOV R0, #42
      ADD R2, R0, R1
      CMP R0, R1
      LDR R3, [R4]
      STR R3, [R4]
      NOP
      """)

    assert assembler.labels["loop"] == 0
    assert Enum.at(instructions, 0) == %ArmInstruction{kind: :label, label: "loop"}

    assert Enum.at(instructions, 1) == %ArmInstruction{
             kind: :data_processing,
             opcode: ArmOpcode.mov(),
             rd: 0,
             rn: nil,
             operand2: %Operand2{type: :immediate, value: 42},
             set_flags: false,
             label: nil
           }

    assert Enum.at(instructions, 2) == %ArmInstruction{
             kind: :data_processing,
             opcode: ArmOpcode.add(),
             rd: 2,
             rn: 0,
             operand2: %Operand2{type: :register, value: 1},
             set_flags: false,
             label: nil
           }

    assert Enum.at(instructions, 3).opcode == ArmOpcode.cmp()
    assert Enum.at(instructions, 3).rd == nil
    assert Enum.at(instructions, 4) == %ArmInstruction{kind: :load, rd: 3, rn: 4}
    assert Enum.at(instructions, 5) == %ArmInstruction{kind: :store, rd: 3, rn: 4}
    assert Enum.at(instructions, 6) == %ArmInstruction{kind: :nop}
  end

  test "parse strips comments and skips empty lines" do
    {:ok, _assembler, instructions} =
      Assembler.new()
      |> Assembler.parse("\nMOV R0, #1 ; comment\n\nADD R1, R0, R0 // twice\n")

    assert length(instructions) == 2
  end

  test "parse returns clear errors for invalid input" do
    assert {:error, "Unknown mnemonic: BLAH"} =
             Assembler.new() |> Assembler.parse("BLAH R0, R1")

    assert {:error, "Invalid register: X0"} =
             Assembler.new() |> Assembler.parse("MOV X0, #1")

    assert {:error, "ADD: expected 3 operands, got 2"} =
             Assembler.new() |> Assembler.parse("ADD R0, R1")
  end

  test "encode generates expected ARM words" do
    {:ok, assembler, instructions} =
      Assembler.new()
      |> Assembler.parse("MOV R0, #42\nADD R2, R0, R1\nLDR R0, [R1]\nSTR R0, [R1]\nNOP")

    {:ok, binary} = Assembler.encode(assembler, instructions)

    assert length(binary) == 5
    assert Enum.at(binary, 0) == 0xE3A0002A
    assert Enum.at(binary, 1) == 0xE0802001
    assert Enum.at(binary, 2) == 0xE5910000
    assert Enum.at(binary, 3) == 0xE5810000
    assert Enum.at(binary, 4) == 0xE1A00000
  end

  test "labels do not produce binary output" do
    {:ok, assembler, instructions} =
      Assembler.new()
      |> Assembler.parse("start:\nMOV R0, #1")

    {:ok, binary} = Assembler.encode(assembler, instructions)
    assert length(binary) == 1
  end
end
