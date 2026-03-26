# frozen_string_literal: true

require_relative "test_helper"
require "open3"
require "tmpdir"

# ==========================================================================
# Tests for bin/grammar-tools CLI
# ==========================================================================
#
# These tests invoke the grammar-tools binary as a subprocess, just like a
# real user would. Using Open3.capture3 captures stdout, stderr, and the exit
# status so we can assert on all three independently.
#
# Why subprocess tests?
# ---------------------
#
# The CLI binary is a thin wrapper around library functions that are already
# covered by unit tests. The subprocess tests verify:
#   - The shebang line is correct and the binary is executable
#   - Exit codes are correct (0, 1, 2)
#   - Output format matches the specified contract
#   - ARGV dispatch routes to the right handler
#
# Design note: we write fixture files to a temporary directory so tests are
# hermetic -- they do not depend on any real grammar files that might not
# exist in the test environment.
# ==========================================================================
class TestCli < Minitest::Test
  BIN = File.expand_path("../bin/grammar-tools", __dir__)

  TOKENS_SOURCE = <<~TOKENS
    NUMBER = /[0-9]+/
    PLUS   = "+"
    MINUS  = "-"
  TOKENS

  GRAMMAR_SOURCE = <<~GRAMMAR
    expression = NUMBER { ( PLUS | MINUS ) NUMBER } ;
  GRAMMAR

  # Helper: write content to a temp file and return its path.
  def write_temp(dir, name, content)
    path = File.join(dir, name)
    File.write(path, content)
    path
  end

  # Helper: run the CLI binary with the given arguments.
  # Returns [stdout, stderr, exit_status_integer].
  def run_cli(*args)
    stdout, stderr, status = Open3.capture3("ruby", BIN, *args)
    [stdout, stderr, status.exitstatus]
  end

  # -------------------------------------------------------------------------
  # --help / no arguments
  # -------------------------------------------------------------------------

  def test_help_flag
    out, _err, code = run_cli("--help")
    assert_equal 0, code
    assert_includes out, "Usage:"
    assert_includes out, "validate"
    assert_includes out, "validate-tokens"
    assert_includes out, "validate-grammar"
  end

  def test_short_help_flag
    out, _err, code = run_cli("-h")
    assert_equal 0, code
    assert_includes out, "Usage:"
  end

  def test_help_subcommand
    out, _err, code = run_cli("help")
    assert_equal 0, code
    assert_includes out, "Usage:"
  end

  def test_no_arguments_shows_usage
    out, _err, code = run_cli
    assert_equal 0, code
    assert_includes out, "Usage:"
  end

  # -------------------------------------------------------------------------
  # Unknown command
  # -------------------------------------------------------------------------

  def test_unknown_command_exits_2
    _out, _err, code = run_cli("frobnicate")
    assert_equal 2, code
  end

  def test_unknown_command_message
    out, _err, _code = run_cli("frobnicate")
    assert_includes out, "Unknown command"
    assert_includes out, "frobnicate"
  end

  # -------------------------------------------------------------------------
  # validate-tokens
  # -------------------------------------------------------------------------

  def test_validate_tokens_ok
    Dir.mktmpdir do |dir|
      tokens_file = write_temp(dir, "test.tokens", TOKENS_SOURCE)
      out, _err, code = run_cli("validate-tokens", tokens_file)
      assert_equal 0, code
      assert_includes out, "OK"
      assert_includes out, "All checks passed."
    end
  end

  def test_validate_tokens_output_format
    Dir.mktmpdir do |dir|
      tokens_file = write_temp(dir, "test.tokens", TOKENS_SOURCE)
      out, _err, _code = run_cli("validate-tokens", tokens_file)
      # First line: "Validating test.tokens ... OK (N tokens)"
      first_line = out.lines.first.chomp
      assert_match(/\AValidating test\.tokens \.\.\. OK \(\d+ tokens\)/, first_line)
    end
  end

  def test_validate_tokens_duplicate_token_exits_1
    bad_tokens = <<~TOKENS
      NUMBER = /[0-9]+/
      NUMBER = /[0-9]+/
    TOKENS
    Dir.mktmpdir do |dir|
      tokens_file = write_temp(dir, "bad.tokens", bad_tokens)
      out, _err, code = run_cli("validate-tokens", tokens_file)
      assert_equal 1, code
      assert_includes out, "error(s)"
      assert_includes out, "Fix them and try again."
    end
  end

  def test_validate_tokens_missing_file_exits_1
    _out, _err, code = run_cli("validate-tokens", "/nonexistent/path/missing.tokens")
    assert_equal 1, code
  end

  def test_validate_tokens_wrong_arg_count_exits_2
    _out, _err, code = run_cli("validate-tokens")
    assert_equal 2, code
  end

  def test_validate_tokens_too_many_args_exits_2
    _out, _err, code = run_cli("validate-tokens", "a.tokens", "extra")
    assert_equal 2, code
  end

  # -------------------------------------------------------------------------
  # validate-grammar
  # -------------------------------------------------------------------------

  def test_validate_grammar_ok
    Dir.mktmpdir do |dir|
      grammar_file = write_temp(dir, "test.grammar", GRAMMAR_SOURCE)
      out, _err, code = run_cli("validate-grammar", grammar_file)
      assert_equal 0, code
      assert_includes out, "OK"
      assert_includes out, "All checks passed."
    end
  end

  def test_validate_grammar_output_format
    Dir.mktmpdir do |dir|
      grammar_file = write_temp(dir, "test.grammar", GRAMMAR_SOURCE)
      out, _err, _code = run_cli("validate-grammar", grammar_file)
      first_line = out.lines.first.chomp
      assert_match(/\AValidating test\.grammar \.\.\. OK \(\d+ rules?\)/, first_line)
    end
  end

  def test_validate_grammar_missing_file_exits_1
    _out, _err, code = run_cli("validate-grammar", "/nonexistent/path/missing.grammar")
    assert_equal 1, code
  end

  def test_validate_grammar_wrong_arg_count_exits_2
    _out, _err, code = run_cli("validate-grammar")
    assert_equal 2, code
  end

  # -------------------------------------------------------------------------
  # validate (full pair)
  # -------------------------------------------------------------------------

  def test_validate_pair_ok
    Dir.mktmpdir do |dir|
      tokens_file = write_temp(dir, "test.tokens", TOKENS_SOURCE)
      grammar_file = write_temp(dir, "test.grammar", GRAMMAR_SOURCE)
      out, _err, code = run_cli("validate", tokens_file, grammar_file)
      assert_equal 0, code
      assert_includes out, "All checks passed."
    end
  end

  def test_validate_pair_output_order
    Dir.mktmpdir do |dir|
      tokens_file = write_temp(dir, "test.tokens", TOKENS_SOURCE)
      grammar_file = write_temp(dir, "test.grammar", GRAMMAR_SOURCE)
      out, _err, _code = run_cli("validate", tokens_file, grammar_file)
      lines = out.lines.map(&:chomp)
      # Lines must appear in the right order.
      tokens_idx = lines.index { |l| l.include?("test.tokens") }
      grammar_idx = lines.index { |l| l.include?("test.grammar") }
      cross_idx = lines.index { |l| l.include?("Cross-validating") }
      assert tokens_idx < grammar_idx
      assert grammar_idx < cross_idx
    end
  end

  def test_validate_pair_missing_tokens_exits_1
    Dir.mktmpdir do |dir|
      grammar_file = write_temp(dir, "test.grammar", GRAMMAR_SOURCE)
      _out, _err, code = run_cli("validate", "/no/such.tokens", grammar_file)
      assert_equal 1, code
    end
  end

  def test_validate_pair_missing_grammar_exits_1
    Dir.mktmpdir do |dir|
      tokens_file = write_temp(dir, "test.tokens", TOKENS_SOURCE)
      _out, _err, code = run_cli("validate", tokens_file, "/no/such.grammar")
      assert_equal 1, code
    end
  end

  def test_validate_pair_wrong_arg_count_exits_2
    _out, _err, code = run_cli("validate", "only_one_arg.tokens")
    assert_equal 2, code
  end

  def test_validate_pair_cross_validation_error
    # Grammar references STAR which is not defined in the tokens file.
    Dir.mktmpdir do |dir|
      tokens_file = write_temp(dir, "test.tokens", TOKENS_SOURCE)
      bad_grammar = "expression = NUMBER STAR NUMBER ;"
      grammar_file = write_temp(dir, "test.grammar", bad_grammar)
      out, _err, code = run_cli("validate", tokens_file, grammar_file)
      assert_equal 1, code
      assert_includes out, "error(s)"
      assert_includes out, "Fix them and try again."
    end
  end

  def test_validate_pair_tokens_error_reports_count
    bad_tokens = <<~TOKENS
      NUMBER = /[0-9]+/
      NUMBER = /[0-9]+/
    TOKENS
    Dir.mktmpdir do |dir|
      tokens_file = write_temp(dir, "bad.tokens", bad_tokens)
      grammar_file = write_temp(dir, "test.grammar", GRAMMAR_SOURCE)
      out, _err, code = run_cli("validate", tokens_file, grammar_file)
      assert_equal 1, code
      first_line = out.lines.first.chomp
      assert_match(/error\(s\)/, first_line)
    end
  end
end
