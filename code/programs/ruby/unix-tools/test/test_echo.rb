# frozen_string_literal: true

# test_echo.rb -- Tests for the Ruby echo tool
# ==============================================
#
# === What These Tests Verify ===
#
# These tests exercise the echo tool's CLI Builder integration and
# business logic. We test:
# - Basic string output (joining args with spaces)
# - The -n flag (suppress trailing newline)
# - The -e flag (enable escape interpretation)
# - The -E flag (disable escape interpretation, the default)
# - All supported escape sequences (\n, \t, \a, \b, \c, etc.)
# - The interpret_escapes function directly

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
  minimum_coverage 80
end

require "minitest/autorun"
require "coding_adventures_cli_builder"

# Load the echo_tool module so we can test the business logic functions.
require_relative "../echo_tool"

# ---------------------------------------------------------------------------
# Helper module: shared spec path and parse method for echo tests
# ---------------------------------------------------------------------------

module EchoTestHelper
  ECHO_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "echo.json")

  def parse_echo_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(ECHO_TEST_SPEC, ["echo"] + argv).parse
  end
end

# ===========================================================================
# Test: Default behavior (no flags)
# ===========================================================================

class TestEchoDefaultBehavior < Minitest::Test
  include EchoTestHelper

  def test_no_args_returns_parse_result
    result = parse_echo_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_no_args_has_empty_strings
    result = parse_echo_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
    strings = result.arguments.fetch("strings", [])
    assert_empty strings
  end

  def test_single_string
    result = parse_echo_argv(["hello"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
    assert_equal ["hello"], result.arguments["strings"]
  end

  def test_multiple_strings
    result = parse_echo_argv(["hello", "world"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
    assert_equal ["hello", "world"], result.arguments["strings"]
  end
end

# ===========================================================================
# Test: -n flag
# ===========================================================================

class TestEchoNoNewlineFlag < Minitest::Test
  include EchoTestHelper

  def test_n_flag_is_set
    result = parse_echo_argv(["-n", "hello"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
    assert result.flags["no_newline"], "-n should set no_newline flag"
  end
end

# ===========================================================================
# Test: -e flag
# ===========================================================================

class TestEchoEnableEscapesFlag < Minitest::Test
  include EchoTestHelper

  def test_e_flag_is_set
    result = parse_echo_argv(["-e", "hello"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
    assert result.flags["enable_escapes"], "-e should set enable_escapes flag"
  end
end

# ===========================================================================
# Test: -E flag
# ===========================================================================

class TestEchoDisableEscapesFlag < Minitest::Test
  include EchoTestHelper

  def test_uppercase_e_flag_is_set
    result = parse_echo_argv(["-E", "hello"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
    assert result.flags["disable_escapes"], "-E should set disable_escapes flag"
  end
end

# ===========================================================================
# Test: --help flag
# ===========================================================================

class TestEchoHelpFlag < Minitest::Test
  include EchoTestHelper

  def test_help_returns_help_result
    result = parse_echo_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_help_text_contains_program_name
    result = parse_echo_argv(["--help"])
    assert_includes result.text, "echo"
  end
end

# ===========================================================================
# Test: --version flag
# ===========================================================================

class TestEchoVersionFlag < Minitest::Test
  include EchoTestHelper

  def test_version_returns_version_result
    result = parse_echo_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
  end

  def test_version_string
    result = parse_echo_argv(["--version"])
    assert_equal "1.0.0", result.version
  end
end

# ===========================================================================
# Test: interpret_escapes function
# ===========================================================================

class TestInterpretEscapes < Minitest::Test
  # --- Basic escapes -------------------------------------------------------

  def test_newline_escape
    result, stop = interpret_escapes("hello\\nworld")
    assert_equal "hello\nworld", result
    refute stop
  end

  def test_tab_escape
    result, stop = interpret_escapes("hello\\tworld")
    assert_equal "hello\tworld", result
    refute stop
  end

  def test_backslash_escape
    result, stop = interpret_escapes("hello\\\\world")
    assert_equal "hello\\world", result
    refute stop
  end

  def test_alert_escape
    result, stop = interpret_escapes("\\a")
    assert_equal "\a", result
    refute stop
  end

  def test_backspace_escape
    result, stop = interpret_escapes("\\b")
    assert_equal "\b", result
    refute stop
  end

  def test_form_feed_escape
    result, stop = interpret_escapes("\\f")
    assert_equal "\f", result
    refute stop
  end

  def test_carriage_return_escape
    result, stop = interpret_escapes("\\r")
    assert_equal "\r", result
    refute stop
  end

  def test_vertical_tab_escape
    result, stop = interpret_escapes("\\v")
    assert_equal "\v", result
    refute stop
  end

  # --- \c (stop output) ----------------------------------------------------

  def test_c_escape_stops_output
    result, stop = interpret_escapes("hello\\cworld")
    assert_equal "hello", result
    assert stop, "\\c should signal stop"
  end

  # --- Octal escapes -------------------------------------------------------

  def test_octal_escape_null
    result, stop = interpret_escapes("\\0")
    assert_equal "\0", result
    refute stop
  end

  def test_octal_escape_A
    # \0101 = 65 decimal = 'A'
    result, stop = interpret_escapes("\\0101")
    assert_equal "A", result
    refute stop
  end

  def test_octal_escape_newline
    # \012 = 10 decimal = newline
    result, stop = interpret_escapes("\\012")
    assert_equal "\n", result
    refute stop
  end

  # --- Hex escapes ---------------------------------------------------------

  def test_hex_escape_A
    # \x41 = 65 decimal = 'A'
    result, stop = interpret_escapes("\\x41")
    assert_equal "A", result
    refute stop
  end

  def test_hex_escape_newline
    # \x0a = 10 decimal = newline
    result, stop = interpret_escapes("\\x0a")
    assert_equal "\n", result
    refute stop
  end

  def test_hex_escape_no_digits
    # \x with no valid hex digits should be literal \x
    result, stop = interpret_escapes("\\xzz")
    assert_equal "\\xzz", result
    refute stop
  end

  # --- Unrecognized escapes ------------------------------------------------

  def test_unrecognized_escape_preserved
    result, stop = interpret_escapes("\\q")
    assert_equal "\\q", result
    refute stop
  end

  # --- No escapes ----------------------------------------------------------

  def test_plain_text_unchanged
    result, stop = interpret_escapes("hello world")
    assert_equal "hello world", result
    refute stop
  end

  # --- Trailing backslash --------------------------------------------------

  def test_trailing_backslash
    result, stop = interpret_escapes("hello\\")
    assert_equal "hello\\", result
    refute stop
  end

  # --- Empty string --------------------------------------------------------

  def test_empty_string
    result, stop = interpret_escapes("")
    assert_equal "", result
    refute stop
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestEchoMainFunction < Minitest::Test
  def test_main_echo_no_args_prints_empty_line
    old_argv = ARGV.dup
    ARGV.replace([])
    output = capture_io { echo_main }[0]
    assert_equal "\n", output
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_echo_single_arg
    old_argv = ARGV.dup
    ARGV.replace(["hello"])
    output = capture_io { echo_main }[0]
    assert_equal "hello\n", output
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_echo_multiple_args
    old_argv = ARGV.dup
    ARGV.replace(["hello", "world"])
    output = capture_io { echo_main }[0]
    assert_equal "hello world\n", output
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_echo_n_flag
    old_argv = ARGV.dup
    ARGV.replace(["-n", "hello"])
    output = capture_io { echo_main }[0]
    assert_equal "hello", output
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_echo_e_flag
    old_argv = ARGV.dup
    ARGV.replace(["-e", "hello\\nworld"])
    output = capture_io { echo_main }[0]
    assert_equal "hello\nworld\n", output
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_help_exits_with_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) do
      capture_io { echo_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_with_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) do
      capture_io { echo_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end
end
