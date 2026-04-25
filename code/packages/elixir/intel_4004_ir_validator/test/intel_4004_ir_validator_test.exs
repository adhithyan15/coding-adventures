defmodule CodingAdventures.Intel4004IrValidatorTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.CompilerIr.{
    IrDataDecl,
    IrImmediate,
    IrInstruction,
    IrLabel,
    IrProgram,
    IrRegister
  }

  alias CodingAdventures.Intel4004IrValidator
  alias CodingAdventures.Intel4004IrValidator.{IrValidationError, IrValidator}

  defp program(instructions, data \\ []) do
    %IrProgram{IrProgram.new("_start") | instructions: instructions, data: data}
  end

  defp label(name), do: %IrInstruction{opcode: :label, operands: [%IrLabel{name: name}], id: -1}
  defp call(name, id), do: %IrInstruction{opcode: :call, operands: [%IrLabel{name: name}], id: id}

  defp load_imm(register, value, id \\ 1) do
    %IrInstruction{
      opcode: :load_imm,
      operands: [%IrRegister{index: register}, %IrImmediate{value: value}],
      id: id
    }
  end

  test "accepts a small feasible program" do
    valid = program([label("_start"), load_imm(0, 0), %IrInstruction{opcode: :halt, id: 2}])

    assert Intel4004IrValidator.validate(valid) == []
    assert IrValidator.validate(valid) == []
  end

  test "rejects unsupported word operations once per operation family" do
    invalid =
      program([
        %IrInstruction{opcode: :load_word, id: 1},
        %IrInstruction{opcode: :load_word, id: 2},
        %IrInstruction{opcode: :store_word, id: 3},
        %IrInstruction{opcode: :store_word, id: 4}
      ])

    errors = IrValidator.validate(invalid)

    assert Enum.map(errors, & &1.rule) == ["no_word_ops", "no_word_ops"]
    assert Enum.at(errors, 0).message =~ "LOAD_WORD"
    assert Enum.at(errors, 1).message =~ "STORE_WORD"
  end

  test "rejects excessive static RAM" do
    invalid = program([], [%IrDataDecl{label: "huge", size: 200, init: 0}])

    assert [%IrValidationError{rule: "static_ram"}] = IrValidator.validate(invalid)
  end

  test "rejects recursive call graphs" do
    invalid =
      program([
        label("_fn_a"),
        call("_fn_b", 1),
        label("_fn_b"),
        call("_fn_a", 2)
      ])

    assert [%IrValidationError{rule: "call_depth"} = error] = IrValidator.validate(invalid)
    assert error.message =~ "_fn_a -> _fn_b -> _fn_a"
    assert Exception.message(error) == error.message
    assert Kernel.to_string(error) =~ "[call_depth]"
  end

  test "rejects call graphs deeper than the hardware stack" do
    invalid =
      program([
        label("_start"),
        call("_a", 1),
        label("_a"),
        call("_b", 2),
        label("_b"),
        call("_c", 3),
        label("_c")
      ])

    assert [%IrValidationError{rule: "call_depth", message: message}] =
             IrValidator.validate(invalid)

    assert message =~ "depth 3"
  end

  test "rejects too many distinct virtual registers" do
    instructions = Enum.map(0..12, fn index -> load_imm(index, index, index) end)
    invalid = program(instructions)

    assert [%IrValidationError{rule: "register_count"}] = IrValidator.validate(invalid)
  end

  test "rejects load immediates outside Intel 4004 byte range" do
    invalid = program([load_imm(0, -1, 1), load_imm(1, 256, 2), load_imm(2, 255, 3)])

    errors = IrValidator.validate(invalid)

    assert Enum.map(errors, & &1.rule) == ["operand_range", "operand_range"]
    assert Enum.all?(errors, &String.contains?(&1.message, "LOAD_IMM"))
  end
end
