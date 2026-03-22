# frozen_string_literal: true

# test_dirname.rb -- Tests for the Ruby dirname tool
# ====================================================
#
# === What These Tests Verify ===
#
# These tests exercise the dirname tool's CLI Builder integration and
# business logic. We test:
# - Simple directory extraction
# - Root paths
# - Bare filenames (no directory component)
# - Trailing slashes
# - Edge cases (empty strings, all-slash paths)
# - The compute_dirname function directly

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
  minimum_coverage 80
end

require "minitest/autorun"
require "stringio"
require "tempfile"
require "coding_adventures_cli_builder"

# Load the dirname_tool module so we can test the business logic functions.
require_relative "../dirname_tool"

# ---------------------------------------------------------------------------
# Helper module: shared spec path and parse method for dirname tests
# ---------------------------------------------------------------------------

module DirnameTestHelper
  DIRNAME_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "dirname.json")

  def parse_dirname_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(DIRNAME_TEST_SPEC, ["dirname"] + argv).parse
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestDirnameCliIntegration < Minitest::Test
  include DirnameTestHelper

  def test_help_returns_help_result
    result = parse_dirname_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
    assert_includes result.text, "dirname"
  end

  def test_version_returns_version_result
    result = parse_dirname_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_single_arg_returns_parse_result
    result = parse_dirname_argv(["/usr/bin/sort"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_zero_flag
    result = parse_dirname_argv(["-z", "/usr/bin/sort"])
    assert result.flags["zero"]
  end

  def test_multiple_args
    result = parse_dirname_argv(["/usr/bin", "/usr/lib"])
    assert_equal ["/usr/bin", "/usr/lib"], result.arguments["names"]
  end

  def test_names_argument_populated
    result = parse_dirname_argv(["/path/to/file"])
    assert_equal ["/path/to/file"], result.arguments["names"]
  end
end

# ===========================================================================
# Test: compute_dirname function
# ===========================================================================

class TestComputeDirname < Minitest::Test
  def test_simple_path
    assert_equal "/usr/bin", compute_dirname("/usr/bin/sort")
  end

  def test_bare_filename
    assert_equal ".", compute_dirname("stdio.h")
  end

  def test_root_path
    assert_equal "/", compute_dirname("/")
  end

  def test_trailing_slash
    assert_equal "/usr", compute_dirname("/usr/bin/")
  end

  def test_empty_string
    assert_equal ".", compute_dirname("")
  end

  def test_multiple_slashes
    assert_equal "/", compute_dirname("///")
  end

  def test_path_with_file
    assert_equal "/usr/include", compute_dirname("/usr/include/stdio.h")
  end

  def test_relative_path
    assert_equal "foo", compute_dirname("foo/bar")
  end

  def test_deep_path
    assert_equal "/a/b/c/d", compute_dirname("/a/b/c/d/e")
  end

  def test_single_component_with_slash
    assert_equal "/", compute_dirname("/usr")
  end

  def test_double_trailing_slash
    assert_equal "/path", compute_dirname("/path/foo//")
  end

  def test_dot_directory
    assert_equal ".", compute_dirname(".")
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestDirnameMainFunction < Minitest::Test
  include DirnameTestHelper

  def test_main_simple
    old_argv = ARGV.dup
    ARGV.replace(["/usr/bin/sort"])
    output = capture_io { dirname_main }[0]
    assert_equal "/usr/bin\n", output
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_bare_filename
    old_argv = ARGV.dup
    ARGV.replace(["file.txt"])
    output = capture_io { dirname_main }[0]
    assert_equal ".\n", output
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_multiple_args
    old_argv = ARGV.dup
    ARGV.replace(["/usr/bin/sort", "/usr/lib/libc.so"])
    output = capture_io { dirname_main }[0]
    lines = output.split("\n")
    assert_equal "/usr/bin", lines[0]
    assert_equal "/usr/lib", lines[1]
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_help_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) do
      capture_io { dirname_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) do
      capture_io { dirname_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_root
    old_argv = ARGV.dup
    ARGV.replace(["/"])
    output = capture_io { dirname_main }[0]
    assert_equal "/\n", output
  ensure
    ARGV.replace(old_argv)
  end
end
