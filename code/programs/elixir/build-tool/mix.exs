defmodule BuildTool.MixProject do
  use Mix.Project

  def project do
    [
      app: :build_tool,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      escript: [main_module: BuildTool.CLI],
      test_coverage: [summary: [threshold: 80], ignore_modules: [~r/.*Tokens$/, ~r/.*Grammar$/]],
      deps: deps()
    ]
  end

  def application do
    [extra_applications: [:logger, :crypto]]
  end

  defp deps do
    [
      {:jason, "~> 1.4"},
      {:coding_adventures_progress_bar, path: "../../../packages/elixir/progress_bar"},
      {:coding_adventures_starlark_interpreter, path: "../../../packages/elixir/starlark_interpreter"}
    ]
  end
end
