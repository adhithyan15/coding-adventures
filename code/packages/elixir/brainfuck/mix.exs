defmodule CodingAdventures.Brainfuck.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_brainfuck,
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
      {:coding_adventures_virtual_machine, path: "../virtual_machine"}
    ]
  end
end
