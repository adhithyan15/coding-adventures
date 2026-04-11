defmodule CodingAdventures.Scrypt.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_scrypt,
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
      extra_applications: [:logger, :crypto]
    ]
  end

  defp deps do
    [
      {:coding_adventures_pbkdf2, path: "../pbkdf2"}
    ]
  end
end
