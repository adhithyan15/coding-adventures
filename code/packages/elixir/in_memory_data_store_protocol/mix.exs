defmodule CodingAdventures.InMemoryDataStoreProtocol.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_in_memory_data_store_protocol,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: [{:coding_adventures_resp_protocol, path: "../resp_protocol"}],
      test_coverage: [summary: [threshold: 80]]
    ]
  end

  def application, do: [extra_applications: [:logger]]
end
