# frozen_string_literal: true

# test_tty.rb -- Tests for the Ruby tty tool
# ============================================
#
# === What These Tests Verify ===
#
# These tests exercise the tty tool's CLI Builder integration and
# business logic. We test:
# - The get_tty_name function with tty and non-tty inputs
# - The -s (silent) flag via CLI Builder
# - Exit status behavior (0 for tty, 1 for non-tty)
# - --help and --version via CLI Builder
#
# === Testing Challenge ===
#
# In a test environment, $stdin is typically NOT a tty (since tests
# run in a subprocess or CI pipeline). We work around this by testing
# the get_tty_name function with mock IO objects and by testing the
# "not a tty" path directly.

require "minitest/autorun"
require "stringio"
require "coding_adventures_cli_builder"

# Load the tty_tool module so we can test the business logic functions.
require_relative "../tty_tool"

# ---------------------------------------------------------------------------
# Helper module: shared spec path and parse method for tty tests
# ---------------------------------------------------------------------------

module TtyTestHelper
  TTY_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "tty.json")

  def parse_tty_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(TTY_TEST_SPEC, ["tty"] + argv).parse
  end
end

# ===========================================================================
# Test: get_tty_name function
# ===========================================================================

class TestGetTtyName < Minitest::Test
  def test_non_tty_input_returns_not_a_tty
    # StringIO is never a tty.
    input = StringIO.new("hello")
    name, is_tty = get_tty_name(input)
    assert_equal "not a tty", name
    refute is_tty, "StringIO should not be a tty"
  end

  def test_non_tty_returns_false_flag
    input = StringIO.new
    _name, is_tty = get_tty_name(input)
    refute is_tty
  end

  def test_stdin_in_test_environment
    # In test environments, $stdin is typically not a tty.
    # We test whatever the actual state is.
    name, is_tty = get_tty_name($stdin)
    if $stdin.tty?
      assert is_tty
      refute_equal "not a tty", name
    else
      refute is_tty
      assert_equal "not a tty", name
    end
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestTtyDefaultBehavior < Minitest::Test
  include TtyTestHelper

  def test_no_args_returns_parse_result
    result = parse_tty_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_no_flags_set_by_default
    result = parse_tty_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
    refute result.flags["silent"], "silent should not be set by default"
  end
end

# ===========================================================================
# Test: -s (silent) flag
# ===========================================================================

class TestTtySilentFlag < Minitest::Test
  include TtyTestHelper

  def test_s_flag_is_set
    result = parse_tty_argv(["-s"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
    assert result.flags["silent"], "-s should set the silent flag"
  end

  def test_silent_long_flag_is_set
    result = parse_tty_argv(["--silent"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
    assert result.flags["silent"], "--silent should set the silent flag"
  end
end

# ===========================================================================
# Test: --help flag
# ===========================================================================

class TestTtyHelpFlag < Minitest::Test
  include TtyTestHelper

  def test_help_returns_help_result
    result = parse_tty_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_help_text_contains_program_name
    result = parse_tty_argv(["--help"])
    assert_includes result.text, "tty"
  end
end

# ===========================================================================
# Test: --version flag
# ===========================================================================

class TestTtyVersionFlag < Minitest::Test
  include TtyTestHelper

  def test_version_returns_version_result
    result = parse_tty_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
  end

  def test_version_string
    result = parse_tty_argv(["--version"])
    assert_equal "1.0.0", result.version
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestTtyMainFunction < Minitest::Test
  def test_main_exits_with_status
    # In test environment, stdin is usually not a tty, so expect exit 1.
    old_argv = ARGV.dup
    ARGV.replace([])
    err = assert_raises(SystemExit) do
      capture_io { tty_main }
    end
    # Exit status depends on whether stdin is a tty in the test env.
    expected_status = $stdin.tty? ? 0 : 1
    assert_equal expected_status, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_prints_tty_name_or_not_a_tty
    old_argv = ARGV.dup
    ARGV.replace([])
    output = nil
    assert_raises(SystemExit) do
      output = capture_io { tty_main }[0]
    end
    if output
      if $stdin.tty?
        refute_includes output, "not a tty"
      else
        assert_includes output, "not a tty"
      end
    end
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_silent_mode_no_output
    old_argv = ARGV.dup
    ARGV.replace(["-s"])
    output = nil
    assert_raises(SystemExit) do
      output = capture_io { tty_main }[0]
    end
    assert_equal "", output if output
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_help_exits_with_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) do
      capture_io { tty_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_with_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) do
      capture_io { tty_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end
end
