defmodule CodingAdventures.ComputeRuntime.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_compute_runtime,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      test_coverage: [summary: [threshold: 80], ignore_modules: [~r/.*Tokens$/, ~r/.*Grammar$/]]
    ]
  end

  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp deps do
    [
      {:coding_adventures_device_simulator, path: "../device_simulator"},
      {:coding_adventures_compute_unit, path: "../compute_unit"},
      {:coding_adventures_parallel_execution_engine, path: "../parallel_execution_engine"},
      {:coding_adventures_gpu_core, path: "../gpu_core"},
      {:coding_adventures_fp_arithmetic, path: "../fp_arithmetic"},
      {:coding_adventures_cache, path: "../cache"},
      {:coding_adventures_clock, path: "../clock"}
    ]
  end
end
