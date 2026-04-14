defmodule CodingAdventures.Barcode1D.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_barcode_1d,
      version: "0.1.0",
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
    [
      {:coding_adventures_codabar, path: "../codabar"},
      {:coding_adventures_code128, path: "../code128"},
      {:coding_adventures_code39, path: "../code39"},
      {:coding_adventures_ean_13, path: "../ean_13"},
      {:coding_adventures_itf, path: "../itf"},
      {:coding_adventures_paint_vm_metal_native, path: "../paint_vm_metal_native"},
      {:coding_adventures_paint_codec_png_native, path: "../paint_codec_png_native"},
      {:coding_adventures_pixel_container, path: "../pixel_container"},
      {:coding_adventures_upc_a, path: "../upc_a"}
    ]
  end
end
