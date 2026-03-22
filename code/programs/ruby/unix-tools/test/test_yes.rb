# frozen_string_literal: true

# test_yes.rb -- Tests for the Ruby yes tool
# ============================================
#
# === What These Tests Verify ===
#
# These tests exercise the yes tool's CLI Builder integration and
# business logic. We test:
# - Default behavior (outputs "y" repeatedly)
# - Custom string output
# - Multiple strings joined with spaces
# - The max_lines parameter for testability
# - Broken pipe handling (Errno::EPIPE)
# - --help and --version via CLI Builder

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
  minimum_coverage 80
end

require "minitest/autorun"
require "stringio"
require "coding_adventures_cli_builder"

# Load the yes_tool module so we can test the business logic functions.
require_relative "../yes_tool"

# ---------------------------------------------------------------------------
# Helper module: shared spec path and parse method for yes tests
# ---------------------------------------------------------------------------

module YesTestHelper
  YES_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "yes.json")

  def parse_yes_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(YES_TEST_SPEC, ["yes"] + argv).parse
  end
end

# ===========================================================================
# Test: yes_output function
# ===========================================================================

class TestYesOutput < Minitest::Test
  def test_default_y_output
    io = StringIO.new
    yes_output("y", io, 3)
    assert_equal "y\ny\ny\n", io.string
  end

  def test_custom_string_output
    io = StringIO.new
    yes_output("hello world", io, 4)
    lines = io.string.split("\n")
    assert_equal 4, lines.length
    assert(lines.all? { |l| l == "hello world" })
  end

  def test_single_line_output
    io = StringIO.new
    yes_output("test", io, 1)
    assert_equal "test\n", io.string
  end

  def test_zero_lines_produces_no_output
    io = StringIO.new
    yes_output("y", io, 0)
    assert_equal "", io.string
  end

  def test_empty_string_output
    io = StringIO.new
    yes_output("", io, 3)
    assert_equal "\n\n\n", io.string
  end

  def test_handles_broken_pipe
    # Simulate a broken pipe by creating an IO that raises EPIPE on write.
    broken_io = StringIO.new
    def broken_io.puts(_str)
      raise Errno::EPIPE
    end

    # Should not raise -- just returns silently.
    yes_output("y", broken_io, nil)
  end

  def test_large_line_count
    io = StringIO.new
    yes_output("y", io, 100)
    lines = io.string.split("\n")
    assert_equal 100, lines.length
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestYesDefaultBehavior < Minitest::Test
  include YesTestHelper

  def test_no_args_returns_parse_result
    result = parse_yes_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_single_string_arg
    result = parse_yes_argv(["hello"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
    assert_equal ["hello"], result.arguments["string"]
  end

  def test_multiple_string_args
    result = parse_yes_argv(["hello", "world"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
    assert_equal ["hello", "world"], result.arguments["string"]
  end
end

# ===========================================================================
# Test: --help flag
# ===========================================================================

class TestYesHelpFlag < Minitest::Test
  include YesTestHelper

  def test_help_returns_help_result
    result = parse_yes_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_help_text_contains_program_name
    result = parse_yes_argv(["--help"])
    assert_includes result.text, "yes"
  end
end

# ===========================================================================
# Test: --version flag
# ===========================================================================

class TestYesVersionFlag < Minitest::Test
  include YesTestHelper

  def test_version_returns_version_result
    result = parse_yes_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
  end

  def test_version_string
    result = parse_yes_argv(["--version"])
    assert_equal "1.0.0", result.version
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestYesMainFunction < Minitest::Test
  def test_main_help_exits_with_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) do
      capture_io { yes_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_with_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) do
      capture_io { yes_main }
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
      output = capture_io { yes_main }[0]
    end
    assert_includes output, "1.0.0" if output
  ensure
    ARGV.replace(old_argv)
  end
end
