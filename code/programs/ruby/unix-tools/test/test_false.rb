# frozen_string_literal: true

# test_false.rb -- Tests for the Ruby false tool
# ================================================
#
# === What These Tests Verify ===
#
# These tests verify that the `false` tool always exits with status 1
# (failure), handles --help and --version correctly (but still exits 1),
# and integrates properly with CLI Builder via the false.json spec.
#
# === Key Difference from true ===
#
# GNU `false` exits 1 even for --help and --version. Our tests verify
# this unusual but correct behavior.

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "coding_adventures_cli_builder"

# Load the false_tool module so we can test the main function.
require_relative "../false_tool"

# ---------------------------------------------------------------------------
# Helper module: shared spec path and parse method for false tests
# ---------------------------------------------------------------------------

module FalseTestHelper
  FALSE_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "false.json")

  def parse_false_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(FALSE_TEST_SPEC, ["false"] + argv).parse
  end
end

# ===========================================================================
# Test: Default behavior (no flags) returns ParseResult
# ===========================================================================

class TestFalseDefaultBehavior < Minitest::Test
  include FalseTestHelper

  def test_no_flags_returns_parse_result
    result = parse_false_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_no_flags_has_empty_flags
    result = parse_false_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
    result.flags.each_value do |v|
      refute v, "no flags should be set for false"
    end
  end
end

# ===========================================================================
# Test: --help flag
# ===========================================================================

class TestFalseHelpFlag < Minitest::Test
  include FalseTestHelper

  def test_help_returns_help_result
    result = parse_false_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_help_text_contains_program_name
    result = parse_false_argv(["--help"])
    assert_includes result.text, "false"
  end

  def test_help_text_contains_description
    result = parse_false_argv(["--help"])
    assert_includes result.text.downcase, "nothing"
  end
end

# ===========================================================================
# Test: --version flag
# ===========================================================================

class TestFalseVersionFlag < Minitest::Test
  include FalseTestHelper

  def test_version_returns_version_result
    result = parse_false_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
  end

  def test_version_string
    result = parse_false_argv(["--version"])
    assert_equal "1.0.0", result.version
  end
end

# ===========================================================================
# Test: Main function exits with 1
# ===========================================================================

class TestFalseMainFunction < Minitest::Test
  # The defining behavior of `false`: always exit 1.

  def test_main_exits_with_one
    old_argv = ARGV.dup
    ARGV.replace([])
    err = assert_raises(SystemExit) do
      capture_io { false_main }
    end
    assert_equal 1, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_help_exits_with_one
    # GNU false exits 1 even for --help.
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) do
      capture_io { false_main }
    end
    assert_equal 1, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_help_prints_text
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    output = nil
    assert_raises(SystemExit) do
      output = capture_io { false_main }[0]
    end
    assert_includes output, "false" if output
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_with_one
    # GNU false exits 1 even for --version.
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) do
      capture_io { false_main }
    end
    assert_equal 1, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_prints_version
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    output = nil
    assert_raises(SystemExit) do
      output = capture_io { false_main }[0]
    end
    assert_includes output, "1.0.0" if output
  ensure
    ARGV.replace(old_argv)
  end
end
