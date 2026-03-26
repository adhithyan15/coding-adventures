defmodule CodingAdventures.CelsiusToFahrenheit.MixProject do
  use Mix.Project

  def project do
    [
      app: :celsius_to_fahrenheit_predictor,
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
      {:coding_adventures_loss_functions, path: "../../../packages/elixir/loss_functions"},
      {:coding_adventures_gradient_descent, path: "../../../packages/elixir/gradient_descent"}
    ]
  end
end
