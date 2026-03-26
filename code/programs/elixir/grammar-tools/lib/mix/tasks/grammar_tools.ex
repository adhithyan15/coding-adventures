defmodule Mix.Tasks.GrammarTools do
  @shortdoc "Unified CLI for grammar-tools"
  @moduledoc """
  Unified CLI for grammar-tools, using cli_builder.
  """
  use Mix.Task

  alias CodingAdventures.CliBuilder.{Parser, ParseResult, HelpResult, VersionResult, ParseErrors}
  alias CodingAdventures.GrammarTools.{TokenGrammar, ParserGrammar, CrossValidator, Compiler}

  @impl Mix.Task
  def run(args) do
    spec_path = Path.expand("../../../../../../specs/grammar-tools.cli.json", __DIR__)
    
    case Parser.parse(spec_path, ["grammar-tools" | args]) do
      {:ok, %ParseResult{} = result} ->
        cmd = List.last(result.command_path)
        exit_code = execute_command(cmd, result)
        if exit_code != 0, do: System.stop(exit_code)

      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)
        System.stop(0)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)
        System.stop(0)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn err ->
          IO.puts(:stderr, "Error: #{err.message}")
          if err.suggestion do
            IO.puts(:stderr, "  #{err.suggestion}")
          end
        end)
        System.stop(2)
        
      {:error, err} ->
        IO.puts(:stderr, "Parse error: #{inspect(err)}")
        System.stop(1)
    end
  end

  defp execute_command("validate", result) do
    tokens = result.arguments["tokens_file"]
    grammar = result.arguments["grammar_file"]
    validate_command(tokens, grammar)
  end

  defp execute_command("validate-tokens", result) do
    tokens = result.arguments["tokens_file"]
    validate_tokens_only(tokens)
  end

  defp execute_command("validate-grammar", result) do
    grammar = result.arguments["grammar_file"]
    validate_grammar_only(grammar)
  end

  defp execute_command("compile-tokens", result) do
    tokens = result.arguments["tokens_file"]
    export_name = result.arguments["export_name"]
    compile_tokens_only(tokens, export_name)
  end

  defp execute_command("compile-grammar", result) do
    grammar = result.arguments["grammar_file"]
    export_name = result.arguments["export_name"]
    compile_grammar_only(grammar, export_name)
  end

  defp execute_command("generate", _result) do
    generate_command()
  end

  defp execute_command(cmd, _) do
    IO.puts(:stderr, "Unknown subcommand: #{cmd}")
    2
  end
  
  # =========================================================================
  # Logic functions
  # =========================================================================

  defp validate_command(tokens_path, grammar_path) do
    total_issues = 0

    tokens_file = tokens_path
    unless File.exists?(tokens_file) do
      IO.puts("Error: File not found: #{tokens_path}")
      System.stop(1)
    end

    IO.write("Validating #{Path.basename(tokens_file)} ... ")

    {token_grammar, total_issues} =
      case TokenGrammar.parse(File.read!(tokens_file)) do
        {:error, msg} ->
          IO.puts("PARSE ERROR\n  #{msg}")
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

    grammar_file = grammar_path
    unless File.exists?(grammar_file) do
      IO.puts("Error: File not found: #{grammar_path}")
      System.stop(1)
    end

    IO.write("Validating #{Path.basename(grammar_file)} ... ")

    {parser_grammar, total_issues} =
      case ParserGrammar.parse(File.read!(grammar_file)) do
        {:error, msg} ->
          IO.puts("PARSE ERROR\n  #{msg}")
          System.stop(1)
        {:ok, pg} ->
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

    if total_issues > 0 do
      IO.puts("\nFound #{total_issues} error(s). Fix them and try again.")
      1
    else
      IO.puts("\nAll checks passed.")
      0
    end
  end

  defp validate_tokens_only(tokens_path) do
    unless File.exists?(tokens_path) do
      IO.puts("Error: File not found: #{tokens_path}")
      System.stop(1)
    end

    IO.write("Validating #{Path.basename(tokens_path)} ... ")

    case TokenGrammar.parse(File.read!(tokens_path)) do
      {:error, msg} ->
        IO.puts("PARSE ERROR\n  #{msg}")
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

  defp validate_grammar_only(grammar_path) do
    unless File.exists?(grammar_path) do
      IO.puts("Error: File not found: #{grammar_path}")
      System.stop(1)
    end

    IO.write("Validating #{Path.basename(grammar_path)} ... ")

    case ParserGrammar.parse(File.read!(grammar_path)) do
      {:error, msg} ->
        IO.puts("PARSE ERROR\n  #{msg}")
        1
      {:ok, pg} ->
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

  defp compile_tokens_only(tokens_path, export_name) do
    unless File.exists?(tokens_path) do
      IO.puts(:stderr, "Error: File not found: #{tokens_path}")
      System.stop(1)
    end

    case TokenGrammar.parse(File.read!(tokens_path)) do
      {:error, msg} ->
        IO.puts(:stderr, "PARSE ERROR\n  #{msg}")
        1
      {:ok, tg} ->
        issues = TokenGrammar.validate_token_grammar(tg)
        errors = count_errors(issues)

        if errors > 0 do
          print_issues(issues)
          IO.puts(:stderr, "Error: Cannot compile invalid grammar file.")
          1
        else
          IO.write(Compiler.compile_tokens_to_elixir(tg, export_name))
          0
        end
    end
  end

  defp compile_grammar_only(grammar_path, export_name) do
    unless File.exists?(grammar_path) do
      IO.puts(:stderr, "Error: File not found: #{grammar_path}")
      System.stop(1)
    end

    case ParserGrammar.parse(File.read!(grammar_path)) do
      {:error, msg} ->
        IO.puts(:stderr, "PARSE ERROR\n  #{msg}")
        1
      {:ok, pg} ->
        issues = ParserGrammar.validate_parser_grammar(pg)
        errors = count_errors(issues)

        if errors > 0 do
          print_issues(issues)
          IO.puts(:stderr, "Error: Cannot compile invalid grammar file.")
          1
        else
          IO.write(Compiler.compile_parser_to_elixir(pg, export_name))
          0
        end
    end
  end
  
  defp to_camel_case(str) do
    str
    |> String.replace("-", "_")
    |> String.split("_", trim: true)
    |> Enum.map(&String.capitalize/1)
    |> Enum.join("")
  end

  defp generate_command() do
    root_dir = Path.expand("../../../../../../", __DIR__)
    grammars_dir = Path.join(root_dir, "grammars")
    elixir_pkg_dir = Path.join([root_dir, "packages", "elixir"])

    
    unless File.dir?(grammars_dir) do
      IO.puts(:stderr, "Error: could not find grammars dir #{grammars_dir}")
      System.stop(1)
    end
    
    entries = case File.ls(grammars_dir) do
      {:ok, list} -> list
      _ -> []
    end
    
    result = Enum.reduce(entries, 0, fn fname, acc_err -> 
      path = Path.join(grammars_dir, fname)
      
      cond do
        !File.regular?(path) -> acc_err
        
        String.ends_with?(fname, ".tokens") or String.ends_with?(fname, ".grammar") ->
          is_tokens = String.ends_with?(fname, ".tokens")
          kind = if is_tokens, do: "lexer", else: "parser"
          
          # extract stem
          # "ruby.tokens" -> "ruby"
          # regex /^(.*)\.(tokens|grammar)$/
          stem = String.replace(fname, ~r/\.(tokens|grammar)$/, "")
          
          pkg_name_hyphen = "#{stem}-#{kind}"
          pkg_name_under = "#{stem}_#{kind}"
          
          target_dir = cond do
            File.dir?(Path.join([elixir_pkg_dir, pkg_name_hyphen, "lib"])) -> Path.join(elixir_pkg_dir, pkg_name_hyphen)
            File.dir?(Path.join([elixir_pkg_dir, pkg_name_under, "lib"])) -> Path.join(elixir_pkg_dir, pkg_name_under)
            true -> nil
          end
          
          if target_dir do
            IO.puts("Generating for #{fname} ...")
            
            source = File.read!(path)
            export_name = to_camel_case(stem) <> if(is_tokens, do: "Tokens", else: "Grammar")
            fname_base = String.replace(stem, "-", "_") <> if(is_tokens, do: "_tokens.ex", else: "_grammar.ex")
            out_path = Path.join([target_dir, "lib", fname_base])
            
            error_val = if is_tokens do
              case TokenGrammar.parse(source) do
                {:ok, tg} ->
                  issues = TokenGrammar.validate_token_grammar(tg)
                  if count_errors(issues) > 0 do
                    IO.puts(:stderr, "Error: Cannot compile invalid grammar file #{fname}")
                    print_issues(issues)
                    1
                  else
                    code = Compiler.compile_tokens_to_elixir(tg, export_name)
                    File.write!(out_path, code)
                    IO.puts("  -> Saved #{out_path}")
                    0
                  end
                {:error, e} ->
                  IO.puts(:stderr, "Error: parse failed for #{fname}: #{e}")
                  1
              end
            else
              case ParserGrammar.parse(source) do
                {:ok, pg} ->
                  issues = ParserGrammar.validate_parser_grammar(pg)
                  if count_errors(issues) > 0 do
                    IO.puts(:stderr, "Error: Cannot compile invalid grammar file #{fname}")
                    print_issues(issues)
                    1
                  else
                    code = Compiler.compile_parser_to_elixir(pg, export_name)
                    File.write!(out_path, code)
                    IO.puts("  -> Saved #{out_path}")
                    0
                  end
                {:error, e} ->
                  IO.puts(:stderr, "Error: parse failed for #{fname}: #{e}")
                  1
              end
            end
            
            if error_val > 0, do: 1, else: acc_err
          else
            acc_err
          end
          
        true -> acc_err
      end
    end)
    
    result
  end

  defp count_errors(issues) do
    Enum.count(issues, fn issue -> not String.starts_with?(issue, "Warning:") end)
  end

  defp print_issues(issues, indent \\ "  ") do
    Enum.each(issues, fn issue -> IO.puts("#{indent}#{issue}") end)
  end
end
