defmodule CodingAdventures.Repl.MixProject do
  use Mix.Project

  # ---------------------------------------------------------------------------
  # Mix project configuration.
  #
  # This package is intentionally dependency-free. The REPL framework is a
  # pure-Elixir library that relies only on the standard library and OTP
  # primitives (Task, Process). That keeps it composable: any language
  # implementor can pull this in without dragging in extra dependencies.
  # ---------------------------------------------------------------------------

  def project do
    [
      app: :coding_adventures_repl,
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

  # No external dependencies — standard library only.
  defp deps do
    []
  end
end
