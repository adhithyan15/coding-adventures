defmodule HashBreaker.MixProject do
  use Mix.Project

  def project do
    [
      app: :hash_breaker,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
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
      {:coding_adventures_md5, path: "../../../packages/elixir/md5"}
    ]
  end
end
