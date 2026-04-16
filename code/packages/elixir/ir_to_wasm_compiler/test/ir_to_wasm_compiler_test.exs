defmodule CodingAdventures.IrToWasmCompilerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.Brainfuck
  alias CodingAdventures.BrainfuckIrCompiler
  alias CodingAdventures.BrainfuckIrCompiler.BuildConfig
  alias CodingAdventures.IrToWasmCompiler
  alias CodingAdventures.IrToWasmCompiler.FunctionSignature
  alias CodingAdventures.WasmValidator

  test "lowers Brainfuck output to a WASM module" do
    assert {:ok, ast} = Brainfuck.parse("+.")

    assert {:ok, ir_result} =
             BrainfuckIrCompiler.compile(ast, "program.bf", BuildConfig.release_config())

    module =
      IrToWasmCompiler.compile(ir_result.program, [
        %FunctionSignature{label: "_start", param_count: 0, export_name: "_start"}
      ])

    assert length(module.imports) == 1
    assert Enum.at(module.imports, 0).name == "fd_write"
    assert length(module.functions) == 1
    assert length(module.code) == 1
    assert {:ok, _validated} = WasmValidator.validate(module)
  end

  test "adds fd_read when the IR uses stdin" do
    assert {:ok, ast} = Brainfuck.parse(",.")

    assert {:ok, ir_result} =
             BrainfuckIrCompiler.compile(ast, "program.bf", BuildConfig.release_config())

    module =
      IrToWasmCompiler.compile(ir_result.program, [
        FunctionSignature.exported("_start", 0)
      ])

    assert Enum.map(module.imports, & &1.name) == ["fd_write", "fd_read"]
  end
end
