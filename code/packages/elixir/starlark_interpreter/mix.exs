defmodule CodingAdventures.StarlarkInterpreter.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_starlark_interpreter,
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
      {:coding_adventures_virtual_machine, path: "../virtual_machine"},
      {:coding_adventures_bytecode_compiler, path: "../bytecode_compiler"},
      {:coding_adventures_starlark_ast_to_bytecode_compiler, path: "../starlark_ast_to_bytecode_compiler"},
      {:coding_adventures_starlark_vm, path: "../starlark_vm"}
    ]
  end
end
