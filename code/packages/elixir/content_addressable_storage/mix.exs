defmodule CodingAdventures.ContentAddressableStorage.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_content_addressable_storage,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_coverage: [
        summary: [threshold: 80]
      ]
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [
      {:coding_adventures_sha1, path: "../sha1"}
    ]
  end
end
