defmodule CodingAdventures.DocumentAstToHtml.MixProject do
  use Mix.Project

  def project do
    [
      app: :coding_adventures_document_ast_to_html,
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
      {:coding_adventures_document_ast, path: "../document_ast"}
    ]
  end
end
