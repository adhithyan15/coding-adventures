defmodule CodingAdventures.Argon2id.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_argon2id,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_coverage: [summary: [threshold: 80]]
    ]
  end

  def application, do: [extra_applications: [:logger]]

  defp deps do
    [{:coding_adventures_blake2b, path: "../blake2b"}]
  end
end
