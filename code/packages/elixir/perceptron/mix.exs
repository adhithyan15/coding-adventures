defmodule Perceptron.MixProject do
  use Mix.Project

  def project do
    [
      app: :perceptron,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      test_coverage: [summary: [threshold: 80], ignore_modules: [~r/.*Tokens$/, ~r/.*Grammar$/]],
      deps: deps()
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [
      {:matrix, path: "../matrix"},
      {:coding_adventures_loss_functions, path: "../loss_functions"},
      {:activation_functions, path: "../activation_functions"}
    ]
  end
end
