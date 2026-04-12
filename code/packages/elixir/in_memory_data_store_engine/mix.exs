defmodule CodingAdventures.InMemoryDataStoreEngine.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_in_memory_data_store_engine,
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
      {:coding_adventures_hash_map, path: "../hash_map"},
      {:coding_adventures_hash_set, path: "../hash_set"},
      {:coding_adventures_heap, path: "../heap"},
      {:coding_adventures_skip_list, path: "../skip_list"},
      {:coding_adventures_hyperloglog, path: "../hyperloglog"},
      {:coding_adventures_array_list, path: "../array_list"},
      {:coding_adventures_radix_tree, path: "../radix_tree"},
      {:coding_adventures_in_memory_data_store_protocol, path: "../in_memory_data_store_protocol"}
    ]
  end
end
