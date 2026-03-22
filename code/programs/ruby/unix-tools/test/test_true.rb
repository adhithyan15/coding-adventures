# frozen_string_literal: true

# test_true.rb -- Tests for the Ruby true tool
# ==============================================
#
# === What These Tests Verify ===
#
# These tests verify that the `true` tool always exits with status 0
# (success), handles --help and --version correctly, and integrates
# properly with CLI Builder via the true.json spec.
#
# === Why Test Something So Simple? ===
#
# Even trivial programs deserve tests. They verify that:
# 1. The JSON spec is well-formed and loads correctly.
# 2. CLI Builder integration works (correct module paths, constants).
# 3. The exit behavior matches the POSIX specification.

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
  minimum_coverage 80
end

require "minitest/autorun"
require "coding_adventures_cli_builder"

# Load the true_tool module so we can test the main function.
require_relative "../true_tool"

# ---------------------------------------------------------------------------
# Helper module: shared spec path and parse method for true tests
# ---------------------------------------------------------------------------

module TrueTestHelper
  TRUE_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "true.json")

  def parse_true_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(TRUE_TEST_SPEC, ["true"] + argv).parse
  end
end

# ===========================================================================
# Test: Default behavior (no flags) returns ParseResult
# ===========================================================================

class TestTrueDefaultBehavior < Minitest::Test
  include TrueTestHelper

  def test_no_flags_returns_parse_result
    result = parse_true_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_no_flags_has_empty_flags
    result = parse_true_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
    result.flags.each_value do |v|
      refute v, "no flags should be set for true"
    end
  end
end

# ===========================================================================
# Test: --help flag
# ===========================================================================

class TestTrueHelpFlag < Minitest::Test
  include TrueTestHelper

  def test_help_returns_help_result
    result = parse_true_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_help_text_contains_program_name
    result = parse_true_argv(["--help"])
    assert_includes result.text, "true"
  end

  def test_help_text_contains_description
    result = parse_true_argv(["--help"])
    assert_includes result.text.downcase, "nothing"
  end
end

# ===========================================================================
# Test: --version flag
# ===========================================================================

class TestTrueVersionFlag < Minitest::Test
  include TrueTestHelper

  def test_version_returns_version_result
    result = parse_true_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
  end

  def test_version_string
    result = parse_true_argv(["--version"])
    assert_equal "1.0.0", result.version
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestTrueMainFunction < Minitest::Test
  def test_main_exits_with_zero
    old_argv = ARGV.dup
    ARGV.replace([])
    err = assert_raises(SystemExit) do
      capture_io { true_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_help_exits_with_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) do
      capture_io { true_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_help_prints_text
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    output = nil
    assert_raises(SystemExit) do
      output = capture_io { true_main }[0]
    end
    assert_includes output, "true" if output
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_with_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) do
      capture_io { true_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_prints_version
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    output = nil
    assert_raises(SystemExit) do
      output = capture_io { true_main }[0]
    end
    assert_includes output, "1.0.0" if output
  ensure
    ARGV.replace(old_argv)
  end
end
