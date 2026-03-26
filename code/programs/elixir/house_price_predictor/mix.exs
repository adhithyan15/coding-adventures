defmodule HousePricePredictor.MixProject do
  use Mix.Project

  def project do
    [
      app: :house_price_predictor,
      version: "0.1.0",
      elixir: "~> 1.14",
      test_coverage: [summary: [threshold: 80], ignore_modules: [~r/.*Tokens$/, ~r/.*Grammar$/]],
      deps: deps()
    ]
  end

  defp deps do
    [
      {:matrix, path: "../../../packages/elixir/matrix"},
      {:coding_adventures_loss_functions, path: "../../../packages/elixir/loss_functions"}
    ]
  end
end
