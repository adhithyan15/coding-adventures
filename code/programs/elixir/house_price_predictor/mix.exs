defmodule HousePricePredictor.MixProject do
  use Mix.Project

  def project do
    [
      app: :house_price_predictor,
      version: "0.1.0",
      elixir: "~> 1.14",
      deps: deps()
    ]
  end

  defp deps do
    [
      {:matrix, path: "../../../packages/elixir/matrix"},
      {:coding_adventures_loss_functions, path: "../../../packages/elixir/loss_functions"},
      {:coding_adventures_feature_normalization,
       path: "../../../packages/elixir/feature_normalization"}
    ]
  end
end
