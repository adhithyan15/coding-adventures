defmodule CodingAdventures.Uuid.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_uuid,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_coverage: [summary: [threshold: 80], ignore_modules: [~r/.*Tokens$/, ~r/.*Grammar$/]]
    ]
  end

  def application do
    [extra_applications: [:logger, :crypto]]
  end

  defp deps do
    [
      {:coding_adventures_sha1, path: "../sha1"},
      {:coding_adventures_md5, path: "../md5"}
    ]
  end
end
