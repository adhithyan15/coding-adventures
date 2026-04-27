defmodule CodingAdventures.AesModes.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_aes_modes,
      version: "0.1.0",
      elixir: "~> 1.14",
      deps: deps(),
      test_coverage: [threshold: 80]
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [{:coding_adventures_aes, path: "../aes"}]
  end
end
