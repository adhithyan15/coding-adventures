defmodule CodingAdventures.NibWasmCompiler.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_nib_wasm_compiler,
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
      {:coding_adventures_nib_parser, path: "../nib_parser"},
      {:coding_adventures_nib_type_checker, path: "../nib_type_checker"},
      {:coding_adventures_nib_ir_compiler, path: "../nib_ir_compiler"},
      {:coding_adventures_ir_to_wasm_compiler, path: "../ir_to_wasm_compiler"},
      {:coding_adventures_ir_to_wasm_validator, path: "../ir_to_wasm_validator"},
      {:coding_adventures_wasm_module_encoder, path: "../wasm_module_encoder"},
      {:coding_adventures_wasm_validator, path: "../wasm_validator"},
      {:coding_adventures_wasm_runtime, path: "../wasm_runtime"},
      {:coding_adventures_wasm_execution, path: "../wasm_execution"}
    ]
  end
end
