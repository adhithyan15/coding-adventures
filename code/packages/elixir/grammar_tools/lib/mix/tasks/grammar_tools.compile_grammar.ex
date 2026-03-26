defmodule Mix.Tasks.GrammarTools.CompileGrammar do
  @moduledoc """
  Compiles a .grammar file into an Elixir module.

  ## Usage

      mix grammar_tools.compile_grammar <file.grammar> <ExportModule>
  """
  use Mix.Task

  alias CodingAdventures.GrammarTools.{ParserGrammar, Compiler}

  @shortdoc "Compile a .grammar file to Elixir code"

  @impl Mix.Task
  def run([grammar_path, export_name]) do
    unless File.exists?(grammar_path) do
      Mix.raise("Error: File not found: #{grammar_path}")
    end

    case ParserGrammar.parse(File.read!(grammar_path)) do
      {:error, msg} ->
        Mix.raise("PARSE ERROR\n  #{msg}")

      {:ok, pg} ->
        issues = ParserGrammar.validate_parser_grammar(pg)
        errors = Enum.count(issues, fn issue -> not String.starts_with?(issue, "Warning:") end)

        if errors > 0 do
          for issue <- issues, do: IO.puts("  #{issue}")
          Mix.raise("Error: Cannot compile invalid grammar file.")
        else
          IO.write(Compiler.compile_parser_to_elixir(pg, export_name))
        end
    end
  end

  def run(_) do
    Mix.raise("Usage: mix grammar_tools.compile_grammar <file.grammar> <ExportModule>")
  end
end
