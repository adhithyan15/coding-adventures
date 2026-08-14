defmodule CodingAdventures.ChaCha20Poly1305.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_chacha20_poly1305,
      version: "0.2.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_coverage: [summary: [threshold: 80]]
    ]
  end

  def application do
    [extra_applications: [:logger]]
  end

  defp deps do
    [{:jason, "~> 1.4", only: :test}]
  end
end
