defmodule Affine2D.MixProject do
  use Mix.Project
  def project do
    [
      app: :affine2d,
      version: "0.1.0",
      elixir: "~> 1.14",
      deps: [
        {:trig, path: "../trig"},
        {:point2d, path: "../point2d"}
      ]
    ]
  end
end
