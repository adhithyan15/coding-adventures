defmodule CodingAdventures.WasmLeb128.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_wasm_leb128,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_coverage: [summary: [threshold: 80], ignore_modules: [~r/.*Tokens$/, ~r/.*Grammar$/]]
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [
    ]
  end
end
