# frozen_string_literal: true

# test_pwd.rb -- Tests for the Ruby pwd tool
# ===========================================
#
# === What These Tests Verify ===
#
# These tests exercise the full CLI Builder integration. We construct a
# Parser with our pwd.json spec and various argv values, then verify that
# the parser returns the correct result type and that the business logic
# produces the expected output.
#
# === Why We Test Through CLI Builder ===
#
# The point of CLI Builder is that developers don't write parsing code.
# So our tests verify the *integration*: does our JSON spec, combined with
# CLI Builder's parser, produce the right behavior? This catches spec
# errors (wrong flag names, missing fields) as well as logic errors.

# SimpleCov must be started BEFORE requiring any application code, otherwise
# the code loaded before SimpleCov starts won't be tracked.
require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
  minimum_coverage 80
end

require "minitest/autorun"
require "pathname"
require "coding_adventures_cli_builder"

# Load the pwd_tool module so we can test the business logic functions.
# We use require_relative to find the file relative to this test file.
require_relative "../pwd_tool"

# ---------------------------------------------------------------------------
# Helper: locate the spec file
# ---------------------------------------------------------------------------
# The spec file lives alongside the main script, one directory up from test/.

SPEC_PATH = File.join(File.dirname(__FILE__), "..", "pwd.json")

# ---------------------------------------------------------------------------
# Helper: parse an argv list against the pwd spec
# ---------------------------------------------------------------------------
# This wraps the CLI Builder Parser so each test doesn't have to repeat
# the boilerplate. We prepend "pwd" to the argv because CLI Builder
# expects argv[0] to be the program name.

def parse_argv(argv)
  CodingAdventures::CliBuilder::Parser.new(SPEC_PATH, ["pwd"] + argv).parse
end

# ===========================================================================
# Test: Default behavior (no flags) returns ParseResult
# ===========================================================================

class TestDefaultBehavior < Minitest::Test
  # When invoked with no flags, pwd should return a ParseResult
  # with `physical` not set (the default is logical mode).

  def test_no_flags_returns_parse_result
    result = parse_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_no_flags_physical_is_not_set
    result = parse_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
    # When no flags are given, physical should not be truthy.
    refute result.flags["physical"], "physical flag should not be set by default"
  end

  def test_no_flags_logical_is_not_set
    result = parse_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
    # When neither flag is given, both default to falsy.
    refute result.flags["logical"], "logical flag should not be set by default"
  end
end

# ===========================================================================
# Test: -P flag
# ===========================================================================

class TestPhysicalFlag < Minitest::Test
  # The `-P` flag should set `physical` to true.

  def test_short_flag
    result = parse_argv(["-P"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
    assert result.flags["physical"], "physical flag should be set with -P"
  end

  def test_long_flag
    result = parse_argv(["--physical"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
    assert result.flags["physical"], "physical flag should be set with --physical"
  end
end

# ===========================================================================
# Test: -L flag
# ===========================================================================

class TestLogicalFlag < Minitest::Test
  # The `-L` flag should set `logical` to true.

  def test_short_flag
    result = parse_argv(["-L"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
    assert result.flags["logical"], "logical flag should be set with -L"
  end

  def test_long_flag
    result = parse_argv(["--logical"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
    assert result.flags["logical"], "logical flag should be set with --logical"
  end
end

# ===========================================================================
# Test: --help flag
# ===========================================================================

class TestHelpFlag < Minitest::Test
  # `--help` should return a HelpResult with non-empty text.

  def test_help_returns_help_result
    result = parse_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_help_text_contains_program_name
    result = parse_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
    assert_includes result.text, "pwd"
  end

  def test_help_text_contains_description
    result = parse_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
    assert_includes result.text.downcase, "working directory"
  end
end

# ===========================================================================
# Test: --version flag
# ===========================================================================

class TestVersionFlag < Minitest::Test
  # `--version` should return a VersionResult with "1.0.0".

  def test_version_returns_version_result
    result = parse_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
  end

  def test_version_string
    result = parse_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end
end

# ===========================================================================
# Test: Unknown flags produce errors
# ===========================================================================

class TestUnknownFlags < Minitest::Test
  # Unknown flags should raise ParseErrors.

  def test_unknown_flag_raises
    assert_raises(CodingAdventures::CliBuilder::ParseErrors) do
      parse_argv(["--unknown"])
    end
  end

  def test_unknown_short_flag_raises
    assert_raises(CodingAdventures::CliBuilder::ParseErrors) do
      parse_argv(["-x"])
    end
  end
end

# ===========================================================================
# Test: Business logic functions
# ===========================================================================

class TestBusinessLogic < Minitest::Test
  # Test the pwd business logic functions directly.

  def test_get_physical_pwd_returns_string
    result = get_physical_pwd
    assert_kind_of String, result
    assert Pathname.new(result).absolute?, "physical pwd should be an absolute path"
  end

  def test_get_logical_pwd_returns_string
    result = get_logical_pwd
    assert_kind_of String, result
    assert Pathname.new(result).absolute?, "logical pwd should be an absolute path"
  end

  def test_logical_pwd_uses_env_when_valid
    # When $PWD matches the real cwd, get_logical_pwd should return it.
    real = File.realpath(".")
    old_pwd = ENV["PWD"]
    begin
      ENV["PWD"] = real
      result = get_logical_pwd
      assert_equal real, result
    ensure
      if old_pwd
        ENV["PWD"] = old_pwd
      else
        ENV.delete("PWD")
      end
    end
  end

  def test_logical_pwd_falls_back_when_env_unset
    # When $PWD is not set, get_logical_pwd should return the physical path.
    old_pwd = ENV["PWD"]
    begin
      ENV.delete("PWD")
      result = get_logical_pwd
      expected = Pathname.new(".").realpath.to_s
      assert_equal expected, result
    ensure
      ENV["PWD"] = old_pwd if old_pwd
    end
  end

  def test_logical_pwd_falls_back_when_env_stale
    # When $PWD points to a different directory, fall back to physical.
    old_pwd = ENV["PWD"]
    begin
      ENV["PWD"] = "/nonexistent/path/that/does/not/exist"
      result = get_logical_pwd
      expected = Pathname.new(".").realpath.to_s
      assert_equal expected, result
    ensure
      if old_pwd
        ENV["PWD"] = old_pwd
      else
        ENV.delete("PWD")
      end
    end
  end

  def test_physical_pwd_resolves_symlinks
    # The physical path should be fully resolved (no symlinks).
    result = get_physical_pwd
    # A fully resolved path should equal itself when resolved again.
    assert_equal File.realpath(result), result
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestMainFunction < Minitest::Test
  # Test the main function by temporarily replacing ARGV and capturing
  # stdout. This exercises the full code path from argument parsing
  # through business logic output.

  def test_main_default_prints_logical_pwd
    # With no flags, main should print the logical working directory.
    old_argv = ARGV.dup
    ARGV.replace([])
    output = capture_io { main }[0]
    assert output.strip.length.positive?, "main should print a non-empty path"
    assert Pathname.new(output.strip).absolute?, "output should be an absolute path"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_physical_flag_prints_resolved_path
    # With -P, main should print the physical (symlink-resolved) path.
    old_argv = ARGV.dup
    ARGV.replace(["-P"])
    output = capture_io { main }[0]
    expected = Pathname.new(".").realpath.to_s
    assert_equal expected, output.strip
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_logical_flag_prints_logical_path
    # With -L, main should print the logical path.
    old_argv = ARGV.dup
    ARGV.replace(["-L"])
    output = capture_io { main }[0]
    assert output.strip.length.positive?, "main should print a non-empty path"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_help_flag
    # With --help, main should print help text and exit 0.
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) do
      capture_io { main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_flag
    # With --version, main should print version and exit 0.
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) do
      capture_io { main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_unknown_flag_exits_with_error
    # With an unknown flag, main should exit with status 1.
    old_argv = ARGV.dup
    ARGV.replace(["--bogus"])
    err = assert_raises(SystemExit) do
      capture_io { main }
    end
    assert_equal 1, err.status
  ensure
    ARGV.replace(old_argv)
  end
end
