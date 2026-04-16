defmodule CodingAdventures.IrToWasmCompiler.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_ir_to_wasm_compiler,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_coverage: [
        summary: [threshold: 80]
      ]
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [
      {:coding_adventures_compiler_ir, path: "../compiler_ir"},
      {:coding_adventures_wasm_leb128, path: "../wasm_leb128"},
      {:coding_adventures_wasm_opcodes, path: "../wasm_opcodes"},
      {:coding_adventures_wasm_types, path: "../wasm_types"},
      {:coding_adventures_wasm_validator, path: "../wasm_validator"},
      {:coding_adventures_brainfuck, path: "../brainfuck"},
      {:coding_adventures_brainfuck_ir_compiler, path: "../brainfuck_ir_compiler"}
    ]
  end
end
