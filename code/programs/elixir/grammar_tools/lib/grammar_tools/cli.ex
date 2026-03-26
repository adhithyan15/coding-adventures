defmodule GrammarTools.CLI do
  @moduledoc """
  Command-line interface for grammar-tools validation.

  This escript wraps the `CodingAdventures.GrammarTools` library behind a
  user-friendly CLI built with `CodingAdventures.CliBuilder`. It is the Elixir
  counterpart of `python -m grammar_tools` and `ruby grammar-tools`, and
  produces identical output across all language implementations.

  ## Why an escript program instead of a Mix task?

  Mix tasks (`mix grammar_tools.validate`) require the project to be a Mix
  workspace and compile the grammar-tools library as a dep. An escript binary
  is self-contained: it bundles the BEAM bytecode and runs anywhere Elixir is
  installed, with no knowledge of Mix. This matches how the Python, Ruby, Go,
  Rust, and TypeScript counterparts work — they are all standalone executables.

  ## Usage

      grammar-tools validate <file.tokens> <file.grammar>
      grammar-tools validate-tokens <file.tokens>
      grammar-tools validate-grammar <file.grammar>
      grammar-tools --help

  ## Exit codes

  | Code | Meaning                                   |
  |------|-------------------------------------------|
  |  0   | All checks passed                         |
  |  1   | One or more validation errors found       |
  |  2   | Usage error (wrong number of arguments)   |

  ## Output format

  Success::

      Validating css.tokens ... OK (39 tokens, 2 skip, 2 error)
      Validating css.grammar ... OK (36 rules)
      Cross-validating ... OK

      All checks passed.

  Failure::

      Validating broken.tokens ... 2 error(s)
        Line 5: Duplicate token name 'IDENT'
      Found 2 error(s). Fix them and try again.
  """

  alias CodingAdventures.CliBuilder
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}
  alias CodingAdventures.GrammarTools.{TokenGrammar, ParserGrammar, CrossValidator}

  # The main/1 function is the escript entry point.
  # argv is the list of command-line arguments as strings.
  def main(argv) do
    root = find_root()
    spec_path = Path.join([root, "code", "specs", "grammar-tools.json"])

    # cli-builder expects argv[0] to be the program name.
    case CliBuilder.parse(spec_path, ["grammar-tools" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: v}} ->
        IO.puts(v)

      {:ok, %ParseResult{} = result} ->
        command = result.arguments["command"]
        files = result.arguments["files"] || []
        exit_code = dispatch(command, files)
        if exit_code != 0, do: System.halt(exit_code)

      {:error, %ParseErrors{message: msg}} ->
        IO.puts(:stderr, "error: #{msg}")
        System.halt(2)
    end
  end

  # ---------------------------------------------------------------------------
  # Command dispatch
  # ---------------------------------------------------------------------------

  # Dispatch to the appropriate validation function based on the command name.
  #
  # Returns 0 on success, 1 on validation errors, 2 on usage errors.
  defp dispatch("validate", [tokens_path, grammar_path]) do
    validate_command(tokens_path, grammar_path)
  end

  defp dispatch("validate", _) do
    IO.puts(:stderr, "Error: 'validate' requires two arguments: <tokens> <grammar>")
    IO.puts(:stderr, "")
    print_usage()
    2
  end

  defp dispatch("validate-tokens", [tokens_path]) do
    validate_tokens_only(tokens_path)
  end

  defp dispatch("validate-tokens", _) do
    IO.puts(:stderr, "Error: 'validate-tokens' requires one argument: <tokens>")
    IO.puts(:stderr, "")
    print_usage()
    2
  end

  defp dispatch("validate-grammar", [grammar_path]) do
    validate_grammar_only(grammar_path)
  end

  defp dispatch("validate-grammar", _) do
    IO.puts(:stderr, "Error: 'validate-grammar' requires one argument: <grammar>")
    IO.puts(:stderr, "")
    print_usage()
    2
  end

  defp dispatch(unknown, _) do
    IO.puts(:stderr, "Error: Unknown command '#{unknown}'")
    IO.puts(:stderr, "")
    print_usage()
    2
  end

  # ---------------------------------------------------------------------------
  # validate — cross-validate a .tokens and .grammar file pair
  # ---------------------------------------------------------------------------

  @doc """
  Validate a `.tokens` and `.grammar` file pair.

  Runs three checks in sequence:
  1. Parse and validate the `.tokens` file (syntax, duplicates, bad regexes)
  2. Parse and validate the `.grammar` file (undefined references, duplicates)
  3. Cross-validate the two for consistency (missing/extra token definitions)

  Returns 0 on success, 1 if any errors are found.
  """
  def validate_command(tokens_path, grammar_path) do
    total_issues = 0

    # Step 1: parse and validate the .tokens file
    unless File.exists?(tokens_path) do
      IO.puts(:stderr, "Error: File not found: #{tokens_path}")
      System.halt(1)
    end

    IO.write("Validating #{Path.basename(tokens_path)} ... ")

    {token_grammar, total_issues} =
      case TokenGrammar.parse(File.read!(tokens_path)) do
        {:error, msg} ->
          IO.puts("PARSE ERROR")
          IO.puts("  #{msg}")
          System.halt(1)

        {:ok, tg} ->
          token_issues = TokenGrammar.validate_token_grammar(tg)
          n_tokens = length(tg.definitions)
          n_skip = length(tg.skip_definitions)
          n_error = length(tg.error_definitions)
          token_errors = count_errors(token_issues)

          if token_errors > 0 do
            IO.puts("#{token_errors} error(s)")
            print_issues(token_issues)
            {tg, total_issues + token_errors}
          else
            parts =
              ["#{n_tokens} tokens"]
              |> append_if(n_skip > 0, "#{n_skip} skip")
              |> append_if(n_error > 0, "#{n_error} error")

            IO.puts("OK (#{Enum.join(parts, ", ")})")
            {tg, total_issues}
          end
      end

    # Step 2: parse and validate the .grammar file
    unless File.exists?(grammar_path) do
      IO.puts(:stderr, "Error: File not found: #{grammar_path}")
      System.halt(1)
    end

    IO.write("Validating #{Path.basename(grammar_path)} ... ")

    {parser_grammar, total_issues} =
      case ParserGrammar.parse(File.read!(grammar_path)) do
        {:error, msg} ->
          IO.puts("PARSE ERROR")
          IO.puts("  #{msg}")
          System.halt(1)

        {:ok, pg} ->
          tg_token_names = TokenGrammar.token_names(token_grammar)
          parser_issues = ParserGrammar.validate_parser_grammar(pg, tg_token_names)
          n_rules = length(pg.rules)
          parser_errors = count_errors(parser_issues)

          if parser_errors > 0 do
            IO.puts("#{parser_errors} error(s)")
            print_issues(parser_issues)
            {pg, total_issues + parser_errors}
          else
            IO.puts("OK (#{n_rules} rules)")
            {pg, total_issues}
          end
      end

    # Step 3: cross-validate
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

    IO.puts("")

    if total_issues > 0 do
      IO.puts("Found #{total_issues} error(s). Fix them and try again.")
      1
    else
      IO.puts("All checks passed.")
      0
    end
  end

  # ---------------------------------------------------------------------------
  # validate-tokens — validate just a .tokens file
  # ---------------------------------------------------------------------------

  @doc """
  Validate a `.tokens` file without a corresponding grammar file.

  Only lexical checks are performed: syntax, duplicate names, bad regex
  patterns, invalid aliases. Cross-file consistency checks are skipped.

  Returns 0 on success, 1 if any errors are found.
  """
  def validate_tokens_only(tokens_path) do
    unless File.exists?(tokens_path) do
      IO.puts(:stderr, "Error: File not found: #{tokens_path}")
      System.halt(1)
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
          IO.puts("#{errors} error(s)")
          print_issues(issues)
          IO.puts("")
          IO.puts("Found #{errors} error(s). Fix them and try again.")
          1
        else
          parts =
            ["#{n_tokens} tokens"]
            |> append_if(n_skip > 0, "#{n_skip} skip")
            |> append_if(n_error > 0, "#{n_error} error")

          IO.puts("OK (#{Enum.join(parts, ", ")})")
          IO.puts("")
          IO.puts("All checks passed.")
          0
        end
    end
  end

  # ---------------------------------------------------------------------------
  # validate-grammar — validate just a .grammar file
  # ---------------------------------------------------------------------------

  @doc """
  Validate a `.grammar` file without a corresponding tokens file.

  Only rule-level checks are performed: duplicate rule names, non-lowercase
  names, and unreachable rules. Token reference checks are skipped because
  there is no tokens file to check against.

  Returns 0 on success, 1 if any errors are found.
  """
  def validate_grammar_only(grammar_path) do
    unless File.exists?(grammar_path) do
      IO.puts(:stderr, "Error: File not found: #{grammar_path}")
      System.halt(1)
    end

    IO.write("Validating #{Path.basename(grammar_path)} ... ")

    case ParserGrammar.parse(File.read!(grammar_path)) do
      {:error, msg} ->
        IO.puts("PARSE ERROR")
        IO.puts("  #{msg}")
        1

      {:ok, pg} ->
        issues = ParserGrammar.validate_parser_grammar(pg)
        n_rules = length(pg.rules)
        errors = count_errors(issues)

        if errors > 0 do
          IO.puts("#{errors} error(s)")
          print_issues(issues)
          IO.puts("")
          IO.puts("Found #{errors} error(s). Fix them and try again.")
          1
        else
          IO.puts("OK (#{n_rules} rules)")
          IO.puts("")
          IO.puts("All checks passed.")
          0
        end
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  # Count how many issues are actual errors (not informational warnings).
  #
  # Issues that start with "Warning:" are shown to the user but do not
  # increment the error count or cause a non-zero exit code.
  defp count_errors(issues) do
    Enum.count(issues, fn issue -> not String.starts_with?(issue, "Warning:") end)
  end

  # Print a list of issues with two-space indentation so they stand out
  # visually from the "Validating ... " lines above them.
  defp print_issues(issues, indent \\ "  ") do
    Enum.each(issues, fn issue -> IO.puts("#{indent}#{issue}") end)
  end

  # Conditionally append an element to a list.
  # Used to build the "OK (N tokens, M skip)" summary string.
  defp append_if(list, false, _), do: list
  defp append_if(list, true, item), do: list ++ [item]

  # Print a short usage summary to stderr.
  defp print_usage do
    IO.puts(:stderr, "Usage: grammar-tools <command> [args...]")
    IO.puts(:stderr, "")
    IO.puts(:stderr, "Commands:")
    IO.puts(:stderr, "  validate <file.tokens> <file.grammar>  Validate a token/grammar pair")
    IO.puts(:stderr, "  validate-tokens <file.tokens>           Validate just a .tokens file")
    IO.puts(:stderr, "  validate-grammar <file.grammar>         Validate just a .grammar file")
    IO.puts(:stderr, "")
    IO.puts(:stderr, "Run 'grammar-tools --help' for full help text.")
  end

  # Walk up the directory tree to find the repo root.
  #
  # The repo root is identified by the presence of `code/specs/grammar-tools.json`.
  # This allows the escript to be run from any directory inside the repo.
  defp find_root do
    Path.expand(".")
    |> Path.split()
    |> Enum.reverse()
    |> find_root_recursive()
  end

  defp find_root_recursive([]), do: "."

  defp find_root_recursive(parts) do
    path = parts |> Enum.reverse() |> Path.join()

    if File.exists?(Path.join(path, "code/specs/grammar-tools.json")) do
      path
    else
      find_root_recursive(tl(parts))
    end
  end
end
