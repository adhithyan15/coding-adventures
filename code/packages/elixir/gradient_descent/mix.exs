defmodule GradientDescent.MixProject do
  use Mix.Project

  def project do
    [
      app: :gradient_descent,
      version: "0.1.0",
      elixir: "~> 1.10",
      start_permanent: Mix.env() == :prod,
      deps: []
    ]
  end

  def application do
    [extra_applications: [:logger]]
  end
end
