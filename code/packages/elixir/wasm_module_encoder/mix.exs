defmodule CodingAdventures.WasmModuleEncoder.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_wasm_module_encoder,
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
      {:coding_adventures_wasm_leb128, path: "../wasm_leb128"},
      {:coding_adventures_wasm_types, path: "../wasm_types"},
      {:coding_adventures_wasm_module_parser, path: "../wasm_module_parser"}
    ]
  end
end
