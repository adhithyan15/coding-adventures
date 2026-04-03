defmodule Point2D.MixProject do
  use Mix.Project
  def project do
    [
      app: :point2d,
      version: "0.1.0",
      elixir: "~> 1.14",
      deps: [{:trig, path: "../trig"}]
    ]
  end
end
