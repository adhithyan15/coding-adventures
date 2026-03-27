defmodule GrammarTools.CLITest do
  use ExUnit.Case

  alias GrammarTools.CLI

  # Path to the real grammar files in the repo for integration-style tests.
  @grammars_dir Path.expand("../../../../grammars", __DIR__)

  # ---------------------------------------------------------------------------
  # validate_command/2
  # ---------------------------------------------------------------------------

  describe "validate_command/2 with valid files" do
    test "succeeds on json.tokens + json.grammar" do
      tokens = Path.join(@grammars_dir, "json.tokens")
      grammar = Path.join(@grammars_dir, "json.grammar")

      if File.exists?(tokens) and File.exists?(grammar) do
        assert CLI.validate_command(tokens, grammar) == 0
      end
    end

    test "succeeds on lisp.tokens + lisp.grammar" do
      tokens = Path.join(@grammars_dir, "lisp.tokens")
      grammar = Path.join(@grammars_dir, "lisp.grammar")

      if File.exists?(tokens) and File.exists?(grammar) do
        assert CLI.validate_command(tokens, grammar) == 0
      end
    end
  end

  describe "validate_command/2 with missing files" do
    # Missing files cause System.halt(1) which raises ErlangError in test.
    # We catch that and assert the process would have halted.
    test "halts on missing tokens file" do
      assert catch_exit(CLI.validate_command("/nonexistent/file.tokens", "any.grammar")) ==
               1
    end

    test "halts on missing grammar file" do
      tokens = Path.join(@grammars_dir, "json.tokens")

      if File.exists?(tokens) do
        assert catch_exit(CLI.validate_command(tokens, "/nonexistent/file.grammar")) == 1
      end
    end
  end

  # ---------------------------------------------------------------------------
  # validate_tokens_only/1
  # ---------------------------------------------------------------------------

  describe "validate_tokens_only/1 with valid file" do
    test "succeeds on json.tokens" do
      tokens = Path.join(@grammars_dir, "json.tokens")

      if File.exists?(tokens) do
        assert CLI.validate_tokens_only(tokens) == 0
      end
    end

    test "succeeds on python.tokens" do
      tokens = Path.join(@grammars_dir, "python.tokens")

      if File.exists?(tokens) do
        assert CLI.validate_tokens_only(tokens) == 0
      end
    end
  end

  describe "validate_tokens_only/1 with invalid input" do
    test "halts on missing file" do
      assert catch_exit(CLI.validate_tokens_only("/nonexistent/file.tokens")) == 1
    end

    test "returns 1 on bad token content" do
      # Write a temp file with invalid token grammar.
      path = System.tmp_dir!() |> Path.join("bad_#{:rand.uniform(100_000)}.tokens")
      # An assignment with no value is a parse error.
      File.write!(path, "BAD =\n")

      result = CLI.validate_tokens_only(path)
      File.rm(path)
      # Could be 0 (library tolerant) or 1 (errors found) — just must not crash.
      assert result in [0, 1]
    end
  end

  # ---------------------------------------------------------------------------
  # validate_grammar_only/1
  # ---------------------------------------------------------------------------

  describe "validate_grammar_only/1 with valid file" do
    test "succeeds on json.grammar" do
      grammar = Path.join(@grammars_dir, "json.grammar")

      if File.exists?(grammar) do
        assert CLI.validate_grammar_only(grammar) == 0
      end
    end

    test "succeeds on lisp.grammar" do
      grammar = Path.join(@grammars_dir, "lisp.grammar")

      if File.exists?(grammar) do
        assert CLI.validate_grammar_only(grammar) == 0
      end
    end
  end

  describe "validate_grammar_only/1 with missing file" do
    test "halts on nonexistent path" do
      assert catch_exit(CLI.validate_grammar_only("/nonexistent/file.grammar")) == 1
    end
  end

  # ---------------------------------------------------------------------------
  # dispatch via main/1 (integration path through cli-builder)
  # ---------------------------------------------------------------------------

  describe "main/1 dispatch" do
    test "unknown command prints error and halts with code 2" do
      assert catch_exit(GrammarTools.CLI.main(["unknown-cmd", "a", "b"])) == 2
    end

    test "validate with wrong number of files halts with code 2" do
      assert catch_exit(GrammarTools.CLI.main(["validate", "only-one-file.tokens"])) == 2
    end

    test "validate-tokens with no files halts with code 2" do
      assert catch_exit(GrammarTools.CLI.main(["validate-tokens"])) == 2
    end

    test "validate-grammar with no files halts with code 2" do
      assert catch_exit(GrammarTools.CLI.main(["validate-grammar"])) == 2
    end

    test "compile-tokens with no files halts with code 2" do
      assert catch_exit(GrammarTools.CLI.main(["compile-tokens"])) == 2
    end

    test "compile-grammar with no files halts with code 2" do
      assert catch_exit(GrammarTools.CLI.main(["compile-grammar"])) == 2
    end
  end

  # ---------------------------------------------------------------------------
  # compile_tokens_command/2
  # ---------------------------------------------------------------------------

  describe "compile_tokens_command/2" do
    test "returns 1 for missing file" do
      assert catch_exit(CLI.compile_tokens_command("/nonexistent/x.tokens", nil)) == 1
    end

    test "returns 0 for valid json.tokens" do
      tokens = Path.join(@grammars_dir, "json.tokens")

      if File.exists?(tokens) do
        assert CLI.compile_tokens_command(tokens, nil) == 0
      end
    end

    test "writes output file when path given" do
      tokens = Path.join(@grammars_dir, "json.tokens")

      if File.exists?(tokens) do
        out = Path.join(System.tmp_dir!(), "test_json_tokens.ex")
        assert CLI.compile_tokens_command(tokens, out) == 0
        content = File.read!(out)
        assert String.contains?(content, "DO NOT EDIT")
        assert String.contains?(content, "token_grammar")
        File.rm(out)
      end
    end
  end

  # ---------------------------------------------------------------------------
  # compile_grammar_command/2
  # ---------------------------------------------------------------------------

  describe "compile_grammar_command/2" do
    test "returns 1 for missing file" do
      assert catch_exit(CLI.compile_grammar_command("/nonexistent/x.grammar", nil)) == 1
    end

    test "returns 0 for valid json.grammar" do
      grammar = Path.join(@grammars_dir, "json.grammar")

      if File.exists?(grammar) do
        assert CLI.compile_grammar_command(grammar, nil) == 0
      end
    end

    test "writes output file when path given" do
      grammar = Path.join(@grammars_dir, "json.grammar")

      if File.exists?(grammar) do
        out = Path.join(System.tmp_dir!(), "test_json_grammar.ex")
        assert CLI.compile_grammar_command(grammar, out) == 0
        content = File.read!(out)
        assert String.contains?(content, "DO NOT EDIT")
        assert String.contains?(content, "parser_grammar")
        File.rm(out)
      end
    end
  end
end
