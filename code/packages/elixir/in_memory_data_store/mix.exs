defmodule CodingAdventures.InMemoryDataStore.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_in_memory_data_store,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_coverage: [summary: [threshold: 80]]
    ]
  end

  def application, do: [extra_applications: [:logger]]

  defp deps do
    [
      {:coding_adventures_resp_protocol, path: "../resp_protocol"},
      {:coding_adventures_in_memory_data_store_protocol, path: "../in_memory_data_store_protocol"},
      {:coding_adventures_in_memory_data_store_engine, path: "../in_memory_data_store_engine"}
    ]
  end
end
