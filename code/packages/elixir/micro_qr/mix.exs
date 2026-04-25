defmodule CodingAdventures.MicroQR.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_micro_qr,
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
      {:coding_adventures_gf256, path: "../gf256"},
      {:coding_adventures_barcode_2d, path: "../barcode_2d"}
    ]
  end
end
