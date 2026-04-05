defmodule CodingAdventuresImageCodecPpm.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_image_codec_ppm,
      version: "0.1.0",
      elixir: "~> 1.17",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_coverage: [threshold: 90]
    ]
  end

  def application, do: [extra_applications: [:logger]]

  defp deps do
    [
      {:coding_adventures_pixel_container, path: "../pixel_container"}
    ]
  end
end
