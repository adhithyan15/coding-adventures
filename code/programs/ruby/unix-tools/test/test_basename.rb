# frozen_string_literal: true

# test_basename.rb -- Tests for the Ruby basename tool
# =====================================================
#
# === What These Tests Verify ===
#
# These tests exercise the basename tool's CLI Builder integration and
# business logic. We test:
# - Simple directory stripping
# - Suffix removal
# - Multiple mode (-a)
# - Edge cases (trailing slashes, all-slash paths, empty strings)
# - The strip_basename function directly

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "stringio"
require "tempfile"
require "coding_adventures_cli_builder"

# Load the basename_tool module so we can test the business logic functions.
require_relative "../basename_tool"

# ---------------------------------------------------------------------------
# Helper module: shared spec path and parse method for basename tests
# ---------------------------------------------------------------------------

module BasenameTestHelper
  BASENAME_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "basename.json")

  def parse_basename_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(BASENAME_TEST_SPEC, ["basename"] + argv).parse
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestBasenameCliIntegration < Minitest::Test
  include BasenameTestHelper

  def test_help_returns_help_result
    result = parse_basename_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
    assert_includes result.text, "basename"
  end

  def test_version_returns_version_result
    result = parse_basename_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_single_arg_returns_parse_result
    result = parse_basename_argv(["/usr/bin/sort"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_multiple_flag
    result = parse_basename_argv(["-a", "a", "b"])
    assert result.flags["multiple"]
  end

  def test_suffix_flag
    result = parse_basename_argv(["-s", ".txt", "file.txt"])
    assert_equal ".txt", result.flags["suffix"]
  end

  def test_zero_flag
    result = parse_basename_argv(["-z", "file"])
    assert result.flags["zero"]
  end
end

# ===========================================================================
# Test: strip_basename function
# ===========================================================================

class TestStripBasename < Minitest::Test
  def test_simple_path
    assert_equal "sort", strip_basename("/usr/bin/sort")
  end

  def test_filename_only
    assert_equal "file.txt", strip_basename("file.txt")
  end

  def test_trailing_slash
    assert_equal "bin", strip_basename("/usr/bin/")
  end

  def test_root_path
    assert_equal "/", strip_basename("/")
  end

  def test_multiple_slashes
    assert_equal "/", strip_basename("///")
  end

  def test_empty_string
    assert_equal "", strip_basename("")
  end

  def test_suffix_removal
    assert_equal "stdio", strip_basename("/usr/include/stdio.h", ".h")
  end

  def test_suffix_is_entire_name
    # When the name equals the suffix, the suffix is NOT removed.
    assert_equal ".h", strip_basename(".h", ".h")
  end

  def test_suffix_not_present
    assert_equal "file.txt", strip_basename("/path/to/file.txt", ".rb")
  end

  def test_deep_path
    assert_equal "deep", strip_basename("/a/b/c/d/e/deep")
  end

  def test_relative_path
    assert_equal "bar", strip_basename("foo/bar")
  end

  def test_double_trailing_slash
    assert_equal "foo", strip_basename("/path/foo//")
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestBasenameMainFunction < Minitest::Test
  include BasenameTestHelper

  def test_main_simple
    old_argv = ARGV.dup
    ARGV.replace(["/usr/bin/sort"])
    output = capture_io { basename_main }[0]
    assert_equal "sort\n", output
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_with_suffix
    old_argv = ARGV.dup
    ARGV.replace(["-s", ".h", "/usr/include/stdio.h"])
    output = capture_io { basename_main }[0]
    assert_equal "stdio\n", output
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_multiple_mode
    old_argv = ARGV.dup
    ARGV.replace(["-a", "/usr/bin/sort", "/usr/bin/cat"])
    output = capture_io { basename_main }[0]
    lines = output.split("\n")
    assert_equal "sort", lines[0]
    assert_equal "cat", lines[1]
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_help_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) do
      capture_io { basename_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) do
      capture_io { basename_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_suffix_with_multiple
    old_argv = ARGV.dup
    ARGV.replace(["-s", ".txt", "a.txt", "b.txt"])
    output = capture_io { basename_main }[0]
    lines = output.split("\n")
    assert_equal "a", lines[0]
    assert_equal "b", lines[1]
  ensure
    ARGV.replace(old_argv)
  end
end
