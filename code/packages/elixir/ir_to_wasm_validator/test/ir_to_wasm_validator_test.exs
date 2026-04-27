defmodule CodingAdventures.IrToWasmValidatorTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Brainfuck
  alias CodingAdventures.BrainfuckIrCompiler
  alias CodingAdventures.BrainfuckIrCompiler.BuildConfig
  alias CodingAdventures.CompilerIr.{IrImmediate, IrInstruction, IrLabel, IrProgram}
  alias CodingAdventures.IrToWasmCompiler.FunctionSignature
  alias CodingAdventures.IrToWasmValidator

  test "returns no errors for supported Brainfuck IR" do
    assert {:ok, ast} = Brainfuck.parse("+.")

    assert {:ok, ir_result} =
             BrainfuckIrCompiler.compile(ast, "program.bf", BuildConfig.release_config())

    errors =
      IrToWasmValidator.validate(ir_result.program, [
        FunctionSignature.exported("_start", 0)
      ])

    assert errors == []
  end

  test "reports unsupported syscalls" do
    program =
      IrProgram.new("_start")
      |> IrProgram.add_instruction(%IrInstruction{
        opcode: :label,
        operands: [%IrLabel{name: "_start"}],
        id: -1
      })
      |> IrProgram.add_instruction(%IrInstruction{
        opcode: :syscall,
        operands: [%IrImmediate{value: 99}],
        id: 0
      })

    errors =
      IrToWasmValidator.validate(program, [
        FunctionSignature.exported("_start", 0)
      ])

    assert length(errors) == 1
    assert hd(errors).message =~ "unsupported SYSCALL"
  end
end
