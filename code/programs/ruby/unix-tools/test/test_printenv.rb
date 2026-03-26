# frozen_string_literal: true

# test_printenv.rb -- Tests for the Ruby printenv tool
# =====================================================
#
# === What These Tests Verify ===
#
# These tests exercise the printenv tool's CLI Builder integration and
# business logic. We test:
# - Printing specific environment variables
# - Printing all environment variables
# - Exit status for missing variables
# - Null-terminated output (-0)
# - The main function behavior

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "stringio"
require "coding_adventures_cli_builder"

# Load the printenv_tool module so we can test the business logic functions.
require_relative "../printenv_tool"

# ---------------------------------------------------------------------------
# Helper module: shared spec path and parse method for printenv tests
# ---------------------------------------------------------------------------

module PrintenvTestHelper
  PRINTENV_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "printenv.json")

  def parse_printenv_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(PRINTENV_TEST_SPEC, ["printenv"] + argv).parse
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestPrintenvCliIntegration < Minitest::Test
  include PrintenvTestHelper

  def test_no_flags_returns_parse_result
    result = parse_printenv_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_help_returns_help_result
    result = parse_printenv_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
    assert_includes result.text, "printenv"
  end

  def test_version_returns_version_result
    result = parse_printenv_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_null_flag
    result = parse_printenv_argv(["-0"])
    assert result.flags["null"]
  end

  def test_variable_arguments
    result = parse_printenv_argv(["HOME", "PATH"])
    assert_equal ["HOME", "PATH"], result.arguments["variables"]
  end

  def test_no_arguments
    result = parse_printenv_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end
end

# ===========================================================================
# Test: Main function - printing specific variables
# ===========================================================================

class TestPrintenvSpecificVars < Minitest::Test
  include PrintenvTestHelper

  def test_print_home
    old_argv = ARGV.dup
    ARGV.replace(["HOME"])
    output = capture_io { printenv_main }[0]
    assert_equal ENV["HOME"] + "\n", output
  ensure
    ARGV.replace(old_argv)
  end

  def test_print_multiple_vars
    old_argv = ARGV.dup
    ARGV.replace(["HOME", "PATH"])
    output = capture_io { printenv_main }[0]
    lines = output.split("\n")
    assert_equal ENV["HOME"], lines[0]
    assert_equal ENV["PATH"], lines[1]
  ensure
    ARGV.replace(old_argv)
  end

  def test_missing_var_exits_1
    old_argv = ARGV.dup
    ARGV.replace(["TOTALLY_NONEXISTENT_VAR_12345"])
    err = assert_raises(SystemExit) do
      capture_io { printenv_main }
    end
    assert_equal 1, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_custom_var
    ENV["TEST_PRINTENV_VAR"] = "test_value_42"
    old_argv = ARGV.dup
    ARGV.replace(["TEST_PRINTENV_VAR"])
    output = capture_io { printenv_main }[0]
    assert_equal "test_value_42\n", output
  ensure
    ENV.delete("TEST_PRINTENV_VAR")
    ARGV.replace(old_argv)
  end

  def test_mixed_existing_and_missing
    ENV["TEST_PRINTENV_EXISTS"] = "found"
    old_argv = ARGV.dup
    ARGV.replace(["TEST_PRINTENV_EXISTS", "NONEXISTENT_VAR_XYZ"])
    err = assert_raises(SystemExit) do
      output = capture_io { printenv_main }[0]
      assert_includes output, "found"
    end
    assert_equal 1, err.status
  ensure
    ENV.delete("TEST_PRINTENV_EXISTS")
    ARGV.replace(old_argv)
  end

  def test_empty_value
    ENV["TEST_PRINTENV_EMPTY"] = ""
    old_argv = ARGV.dup
    ARGV.replace(["TEST_PRINTENV_EMPTY"])
    output = capture_io { printenv_main }[0]
    assert_equal "\n", output
  ensure
    ENV.delete("TEST_PRINTENV_EMPTY")
    ARGV.replace(old_argv)
  end
end

# ===========================================================================
# Test: Main function - printing all variables
# ===========================================================================

class TestPrintenvAllVars < Minitest::Test
  include PrintenvTestHelper

  def test_print_all_includes_home
    old_argv = ARGV.dup
    ARGV.replace([])
    output = capture_io { printenv_main }[0]
    assert_includes output, "HOME="
  ensure
    ARGV.replace(old_argv)
  end

  def test_print_all_includes_path
    old_argv = ARGV.dup
    ARGV.replace([])
    output = capture_io { printenv_main }[0]
    assert_includes output, "PATH="
  ensure
    ARGV.replace(old_argv)
  end

  def test_print_all_sorted
    old_argv = ARGV.dup
    ARGV.replace([])
    output = capture_io { printenv_main }[0]
    lines = output.split("\n")
    keys = lines.map { |l| l.split("=", 2)[0] }
    assert_equal keys.sort, keys
  ensure
    ARGV.replace(old_argv)
  end
end

# ===========================================================================
# Test: Help and version
# ===========================================================================

class TestPrintenvHelpVersion < Minitest::Test
  include PrintenvTestHelper

  def test_main_help_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) do
      capture_io { printenv_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) do
      capture_io { printenv_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end
end
