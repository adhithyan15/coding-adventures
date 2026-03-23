defmodule Wave.MixProject do
  use Mix.Project

  def project do
    [app: :wave, version: "0.1.0", elixir: "~> 1.14", deps: deps()]
  end

  defp deps do
    [{:trig, path: "../trig"}]
  end
end
