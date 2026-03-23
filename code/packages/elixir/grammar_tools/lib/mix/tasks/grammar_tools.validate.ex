defmodule Mix.Tasks.GrammarTools.Validate do
  @moduledoc """
  Validate `.tokens` and `.grammar` files for correctness.

  This Mix task is the Elixir equivalent of `python -m grammar_tools validate`.
  It parses and validates grammar files, reporting any issues it finds.

  ## Usage

      mix grammar_tools.validate <file.tokens> <file.grammar>
      mix grammar_tools.validate_tokens <file.tokens>
      mix grammar_tools.validate_grammar <file.grammar>

  ## Subcommands

  - `validate` — validate both files and cross-validate them together.
  - `validate_tokens` — validate only a `.tokens` file.
  - `validate_grammar` — validate only a `.grammar` file.

  ## Example Output

      Validating css.tokens ... OK (39 tokens, 2 skip, 2 error)
      Validating css.grammar ... OK (36 rules)
      Cross-validating ... OK
      All checks passed.

  On errors:

      Validating broken.tokens ... 2 issues
        Line 5: Duplicate token name 'IDENT' (first defined on line 3)
        Line 8: Invalid regex for token 'BAD': ...
      Found 2 issue(s). Fix them and try again.

  ## Exit Code

  Exits with code 0 on success, 1 on errors.
  """

  use Mix.Task

  alias CodingAdventures.GrammarTools.{TokenGrammar, ParserGrammar, CrossValidator}

  @shortdoc "Validate .tokens and .grammar files"

  @impl Mix.Task
  def run(args) do
    case args do
      ["validate", tokens_path, grammar_path] ->
        result = validate_command(tokens_path, grammar_path)
        if result != 0, do: System.stop(1)

      ["validate_tokens", tokens_path] ->
        result = validate_tokens_only(tokens_path)
        if result != 0, do: System.stop(1)

      ["validate_grammar", grammar_path] ->
        result = validate_grammar_only(grammar_path)
        if result != 0, do: System.stop(1)

      # Also support the two-argument form as the default (validate both)
      [tokens_path, grammar_path] ->
        result = validate_command(tokens_path, grammar_path)
        if result != 0, do: System.stop(1)

      _ ->
        print_usage()
        System.stop(2)
    end
  end

  # -- Validate both files and cross-validate --------------------------------

  defp validate_command(tokens_path, grammar_path) do
    total_issues = 0

    # --- Parse and validate the .tokens file ---
    tokens_file = tokens_path

    unless File.exists?(tokens_file) do
      IO.puts("Error: File not found: #{tokens_path}")
      System.stop(1)
    end

    IO.write("Validating #{Path.basename(tokens_file)} ... ")

    {token_grammar, total_issues} =
      case TokenGrammar.parse(File.read!(tokens_file)) do
        {:error, msg} ->
          IO.puts("PARSE ERROR")
          IO.puts("  #{msg}")
          System.stop(1)

        {:ok, tg} ->
          token_issues = TokenGrammar.validate_token_grammar(tg)
          n_tokens = length(tg.definitions)
          n_skip = length(tg.skip_definitions)
          n_error = length(tg.error_definitions)
          token_errors = count_errors(token_issues)

          if token_errors > 0 do
            IO.puts("#{token_errors} issue(s)")
            print_issues(token_issues)
            {tg, total_issues + token_errors}
          else
            parts = ["#{n_tokens} tokens"]
            parts = if n_skip > 0, do: parts ++ ["#{n_skip} skip"], else: parts
            parts = if n_error > 0, do: parts ++ ["#{n_error} error"], else: parts
            IO.puts("OK (#{Enum.join(parts, ", ")})")
            {tg, total_issues}
          end
      end

    # --- Parse and validate the .grammar file ---
    grammar_file = grammar_path

    unless File.exists?(grammar_file) do
      IO.puts("Error: File not found: #{grammar_path}")
      System.stop(1)
    end

    IO.write("Validating #{Path.basename(grammar_file)} ... ")

    {parser_grammar, total_issues} =
      case ParserGrammar.parse(File.read!(grammar_file)) do
        {:error, msg} ->
          IO.puts("PARSE ERROR")
          IO.puts("  #{msg}")
          System.stop(1)

        {:ok, pg} ->
          # Pass token names so undefined token references are caught
          tg_token_names = TokenGrammar.token_names(token_grammar)
          parser_issues = ParserGrammar.validate_parser_grammar(pg, tg_token_names)
          n_rules = length(pg.rules)
          parser_errors = count_errors(parser_issues)

          if parser_errors > 0 do
            IO.puts("#{parser_errors} issue(s)")
            print_issues(parser_issues)
            {pg, total_issues + parser_errors}
          else
            IO.puts("OK (#{n_rules} rules)")
            {pg, total_issues}
          end
      end

    # --- Cross-validate ---
    IO.write("Cross-validating ... ")
    cross_issues = CrossValidator.validate(token_grammar, parser_grammar)
    cross_errors = count_errors(cross_issues)
    cross_warnings = length(cross_issues) - cross_errors

    total_issues =
      cond do
        cross_errors > 0 ->
          IO.puts("#{cross_errors} error(s)")
          print_issues(cross_issues)
          total_issues + cross_errors

        cross_warnings > 0 ->
          IO.puts("OK (#{cross_warnings} warning(s))")
          print_issues(cross_issues)
          total_issues

        true ->
          IO.puts("OK")
          total_issues
      end

    # --- Summary ---
    if total_issues > 0 do
      IO.puts("\nFound #{total_issues} error(s). Fix them and try again.")
      1
    else
      IO.puts("\nAll checks passed.")
      0
    end
  end

  # -- Validate only a .tokens file -----------------------------------------

  defp validate_tokens_only(tokens_path) do
    unless File.exists?(tokens_path) do
      IO.puts("Error: File not found: #{tokens_path}")
      System.stop(1)
    end

    IO.write("Validating #{Path.basename(tokens_path)} ... ")

    case TokenGrammar.parse(File.read!(tokens_path)) do
      {:error, msg} ->
        IO.puts("PARSE ERROR")
        IO.puts("  #{msg}")
        1

      {:ok, tg} ->
        issues = TokenGrammar.validate_token_grammar(tg)
        n_tokens = length(tg.definitions)
        n_skip = length(tg.skip_definitions)
        n_error = length(tg.error_definitions)
        errors = count_errors(issues)

        if errors > 0 do
          IO.puts("#{errors} issue(s)")
          print_issues(issues)
          IO.puts("\nFound #{errors} error(s). Fix them and try again.")
          1
        else
          parts = ["#{n_tokens} tokens"]
          parts = if n_skip > 0, do: parts ++ ["#{n_skip} skip"], else: parts
          parts = if n_error > 0, do: parts ++ ["#{n_error} error"], else: parts
          IO.puts("OK (#{Enum.join(parts, ", ")})")
          IO.puts("\nAll checks passed.")
          0
        end
    end
  end

  # -- Validate only a .grammar file ----------------------------------------

  defp validate_grammar_only(grammar_path) do
    unless File.exists?(grammar_path) do
      IO.puts("Error: File not found: #{grammar_path}")
      System.stop(1)
    end

    IO.write("Validating #{Path.basename(grammar_path)} ... ")

    case ParserGrammar.parse(File.read!(grammar_path)) do
      {:error, msg} ->
        IO.puts("PARSE ERROR")
        IO.puts("  #{msg}")
        1

      {:ok, pg} ->
        # Without a tokens file we can only check rule-level issues
        issues = ParserGrammar.validate_parser_grammar(pg)
        n_rules = length(pg.rules)
        errors = count_errors(issues)

        if errors > 0 do
          IO.puts("#{errors} issue(s)")
          print_issues(issues)
          IO.puts("\nFound #{errors} error(s). Fix them and try again.")
          1
        else
          IO.puts("OK (#{n_rules} rules)")
          IO.puts("\nAll checks passed.")
          0
        end
    end
  end

  # -- Helpers ---------------------------------------------------------------

  # Count how many issues are actual errors (not warnings).
  # Issues starting with "Warning:" are informational and do not cause failure.
  defp count_errors(issues) do
    Enum.count(issues, fn issue -> not String.starts_with?(issue, "Warning:") end)
  end

  defp print_issues(issues, indent \\ "  ") do
    Enum.each(issues, fn issue -> IO.puts("#{indent}#{issue}") end)
  end

  defp print_usage do
    IO.puts("Usage: mix grammar_tools.validate <subcommand> [args...]")
    IO.puts("")
    IO.puts("Subcommands:")
    IO.puts("  validate <file.tokens> <file.grammar>  Validate a token/grammar pair")
    IO.puts("  validate_tokens <file.tokens>           Validate just a .tokens file")
    IO.puts("  validate_grammar <file.grammar>         Validate just a .grammar file")
    IO.puts("")
    IO.puts("Examples:")
    IO.puts("  mix grammar_tools.validate validate css.tokens css.grammar")
    IO.puts("  mix grammar_tools.validate validate_tokens css.tokens")
    IO.puts("  mix grammar_tools.validate validate_grammar css.grammar")
    IO.puts("  mix grammar_tools.validate css.tokens css.grammar  (shorthand)")
  end
end
