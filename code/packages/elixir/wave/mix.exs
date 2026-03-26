defmodule Wave.MixProject do
  use Mix.Project

  def project do
    [app: :wave, version: "0.1.0", elixir: "~> 1.14", test_coverage: [summary: [threshold: 80], ignore_modules: [~r/.*Tokens$/, ~r/.*Grammar$/]],
      deps: deps()]
  end

  defp deps do
    [{:trig, path: "../trig"}]
  end
end
