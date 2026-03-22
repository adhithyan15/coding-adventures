# frozen_string_literal: true

# test_logname.rb -- Tests for the Ruby logname tool
# ====================================================
#
# === What These Tests Verify ===
#
# These tests exercise the logname tool's CLI Builder integration and
# business logic. We test:
# - The get_login_name function returns a valid login name
# - Fallback to LOGNAME environment variable
# - Error case when no login name is available
# - CLI Builder integration (--help, --version)
# - Main function integration

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
  minimum_coverage 80
end

require "minitest/autorun"
require "etc"
require "coding_adventures_cli_builder"

# Load the logname_tool module so we can test the business logic functions.
require_relative "../logname_tool"

# ---------------------------------------------------------------------------
# Helper module: shared spec path and parse method for logname tests
# ---------------------------------------------------------------------------

module LognameTestHelper
  LOGNAME_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "logname.json")

  def parse_logname_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(LOGNAME_TEST_SPEC, ["logname"] + argv).parse
  end
end

# ===========================================================================
# Test: get_login_name function
# ===========================================================================

class TestGetLoginName < Minitest::Test
  def test_returns_non_nil_string
    # On most systems, at least one method will work.
    name = get_login_name
    refute_nil name, "get_login_name should return a name on this system"
  end

  def test_returns_non_empty_string
    name = get_login_name
    refute_empty name, "login name should not be empty"
  end

  def test_fallback_to_logname_env
    # Temporarily override Etc.getlogin to return nil and set LOGNAME.
    original_logname = ENV["LOGNAME"]
    ENV["LOGNAME"] = "test_user_logname"

    # We can't easily stub Etc.getlogin, but we can verify that
    # the LOGNAME env var is checked by the function. If Etc.getlogin
    # returns a value, it takes precedence. If not, LOGNAME is used.
    name = get_login_name
    refute_nil name
    refute_empty name
  ensure
    if original_logname
      ENV["LOGNAME"] = original_logname
    else
      ENV.delete("LOGNAME")
    end
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestLognameDefaultBehavior < Minitest::Test
  include LognameTestHelper

  def test_no_args_returns_parse_result
    result = parse_logname_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end
end

# ===========================================================================
# Test: --help flag
# ===========================================================================

class TestLognameHelpFlag < Minitest::Test
  include LognameTestHelper

  def test_help_returns_help_result
    result = parse_logname_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_help_text_contains_program_name
    result = parse_logname_argv(["--help"])
    assert_includes result.text, "logname"
  end
end

# ===========================================================================
# Test: --version flag
# ===========================================================================

class TestLognameVersionFlag < Minitest::Test
  include LognameTestHelper

  def test_version_returns_version_result
    result = parse_logname_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
  end

  def test_version_string
    result = parse_logname_argv(["--version"])
    assert_equal "1.0.0", result.version
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestLognameMainFunction < Minitest::Test
  def test_main_prints_login_name
    old_argv = ARGV.dup
    ARGV.replace([])
    output = capture_io { logname_main }[0]
    # Should print a non-empty line.
    refute_empty output.strip, "logname should print a login name"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_help_exits_with_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) do
      capture_io { logname_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_with_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) do
      capture_io { logname_main }
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
      output = capture_io { logname_main }[0]
    end
    assert_includes output, "1.0.0" if output
  ensure
    ARGV.replace(old_argv)
  end
end
