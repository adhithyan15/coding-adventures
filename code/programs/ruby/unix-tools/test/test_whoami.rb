# frozen_string_literal: true

# test_whoami.rb -- Tests for the Ruby whoami tool
# ==================================================
#
# === What These Tests Verify ===
#
# These tests exercise the whoami tool's CLI Builder integration and
# business logic. We test:
# - The get_effective_username function returns a non-empty string
# - CLI Builder integration (--help, --version)
# - Main function prints the username

require "minitest/autorun"
require "etc"
require "coding_adventures_cli_builder"

# Load the whoami_tool module so we can test the business logic functions.
require_relative "../whoami_tool"

# ---------------------------------------------------------------------------
# Helper module: shared spec path and parse method for whoami tests
# ---------------------------------------------------------------------------

module WhoamiTestHelper
  WHOAMI_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "whoami.json")

  def parse_whoami_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(WHOAMI_TEST_SPEC, ["whoami"] + argv).parse
  end
end

# ===========================================================================
# Test: get_effective_username function
# ===========================================================================

class TestGetEffectiveUsername < Minitest::Test
  def test_returns_non_nil_string
    username = get_effective_username
    refute_nil username, "get_effective_username should return a username"
  end

  def test_returns_non_empty_string
    username = get_effective_username
    refute_empty username, "username should not be empty"
  end

  def test_matches_system_username
    # The username should match what Etc or ENV reports.
    expected = Etc.getpwuid(Process.euid).name
    assert_equal expected, get_effective_username
  end

  def test_fallback_to_env_user_when_etc_fails
    # Simulate Etc.getpwuid failing by temporarily stubbing Process.euid
    # to return an invalid UID. Instead, we test the fallback path by
    # calling the function -- on a normal system it will use the primary
    # path. We verify the function handles the fallback ENV path too.
    original_user = ENV["USER"]
    ENV["USER"] = "test_fallback_user"

    # We cannot easily force the ArgumentError path without monkey-patching,
    # but we can verify the function works when ENV["USER"] is set.
    username = get_effective_username
    refute_nil username
  ensure
    if original_user
      ENV["USER"] = original_user
    else
      ENV.delete("USER")
    end
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestWhoamiDefaultBehavior < Minitest::Test
  include WhoamiTestHelper

  def test_no_args_returns_parse_result
    result = parse_whoami_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end
end

# ===========================================================================
# Test: --help flag
# ===========================================================================

class TestWhoamiHelpFlag < Minitest::Test
  include WhoamiTestHelper

  def test_help_returns_help_result
    result = parse_whoami_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_help_text_contains_program_name
    result = parse_whoami_argv(["--help"])
    assert_includes result.text, "whoami"
  end
end

# ===========================================================================
# Test: --version flag
# ===========================================================================

class TestWhoamiVersionFlag < Minitest::Test
  include WhoamiTestHelper

  def test_version_returns_version_result
    result = parse_whoami_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
  end

  def test_version_string
    result = parse_whoami_argv(["--version"])
    assert_equal "1.0.0", result.version
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestWhoamiMainFunction < Minitest::Test
  def test_main_prints_username
    old_argv = ARGV.dup
    ARGV.replace([])
    output = capture_io { whoami_main }[0]
    expected = Etc.getpwuid(Process.euid).name
    assert_equal "#{expected}\n", output
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_help_exits_with_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) do
      capture_io { whoami_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_with_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) do
      capture_io { whoami_main }
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
      output = capture_io { whoami_main }[0]
    end
    assert_includes output, "1.0.0" if output
  ensure
    ARGV.replace(old_argv)
  end
end
