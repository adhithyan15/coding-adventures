defmodule CodingAdventures.ImageCodecPng.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_image_codec_png,
      version: "0.1.0",
      elixir: "~> 1.18",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_coverage: [summary: [threshold: 95]]
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [
      {:coding_adventures_pixel_container, path: "../pixel_container"},
      {:coding_adventures_zip, path: "../zip"}
    ]
  end
end
