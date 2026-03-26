defmodule Mix.Tasks.GrammarTools.CompileTokens do
  @moduledoc """
  Compiles a .tokens file into an Elixir module.

  ## Usage

      mix grammar_tools.compile_tokens <file.tokens> <ExportModule>
  """
  use Mix.Task

  alias CodingAdventures.GrammarTools.{TokenGrammar, Compiler}

  @shortdoc "Compile a .tokens file to Elixir code"

  @impl Mix.Task
  def run([tokens_path, export_name]) do
    unless File.exists?(tokens_path) do
      Mix.raise("Error: File not found: #{tokens_path}")
    end

    case TokenGrammar.parse(File.read!(tokens_path)) do
      {:error, msg} ->
        Mix.raise("PARSE ERROR\n  #{msg}")

      {:ok, tg} ->
        issues = TokenGrammar.validate_token_grammar(tg)
        errors = Enum.count(issues, fn issue -> not String.starts_with?(issue, "Warning:") end)

        if errors > 0 do
          for issue <- issues, do: IO.puts("  #{issue}")
          Mix.raise("Error: Cannot compile invalid grammar file.")
        else
          IO.write(Compiler.compile_tokens_to_elixir(tg, export_name))
        end
    end
  end

  def run(_) do
    Mix.raise("Usage: mix grammar_tools.compile_tokens <file.tokens> <ExportModule>")
  end
end
