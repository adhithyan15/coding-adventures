defmodule CodingAdventures.TestBootstrap.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_test_bootstrap,
      version: "0.1.0",
      elixir: "~> 1.15",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      # the tests define hostile Inspect/String.Chars impls to prove they are never called
      consolidate_protocols: Mix.env() != :test,
      test_coverage: [summary: [threshold: 95]]
    ]
  end

  def application, do: [extra_applications: []]

  # Zero dependencies: the engine is pure Elixir/OTP (VER05m section 2).
  defp deps, do: []
end
