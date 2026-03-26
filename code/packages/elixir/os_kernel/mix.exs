defmodule CodingAdventures.OsKernel.MixProject do
  use Mix.Project
  def project do
    [app: :coding_adventures_os_kernel, version: "0.1.0", elixir: "~> 1.14",
     start_permanent: Mix.env() == :prod, deps: deps(),
     test_coverage: [summary: [threshold: 80], ignore_modules: [~r/.*Tokens$/, ~r/.*Grammar$/]]]
  end
  def application, do: [extra_applications: [:logger]]
  defp deps do
    [{:coding_adventures_display, path: "../display"},
     {:coding_adventures_interrupt_handler, path: "../interrupt_handler"},
     {:coding_adventures_riscv_simulator, path: "../riscv_simulator"}]
  end
end
