# frozen_string_literal: true

# test_tee.rb -- Tests for the Ruby tee tool
# ============================================
#
# === What These Tests Verify ===
#
# These tests exercise the tee tool's CLI Builder integration and
# business logic. We test:
# - Writing to stdout
# - Writing to files
# - Append mode (-a)
# - Multiple output files
# - The tee_stream function directly

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "stringio"
require "tempfile"
require "coding_adventures_cli_builder"

# Load the tee_tool module so we can test the business logic functions.
require_relative "../tee_tool"

# ---------------------------------------------------------------------------
# Helper module: shared spec path and parse method for tee tests
# ---------------------------------------------------------------------------

module TeeTestHelper
  TEE_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "tee.json")

  def parse_tee_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(TEE_TEST_SPEC, ["tee"] + argv).parse
  end

  def with_tempfile_path
    f = Tempfile.new("tee_test")
    path = f.path
    f.close
    f.unlink
    yield path
  ensure
    File.delete(path) if File.exist?(path)
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestTeeCliIntegration < Minitest::Test
  include TeeTestHelper

  def test_no_flags_returns_parse_result
    result = parse_tee_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_help_returns_help_result
    result = parse_tee_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
    assert_includes result.text, "tee"
  end

  def test_version_returns_version_result
    result = parse_tee_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_append_flag
    result = parse_tee_argv(["-a"])
    assert result.flags["append"]
  end

  def test_ignore_interrupts_flag
    result = parse_tee_argv(["-i"])
    assert result.flags["ignore_interrupts"]
  end

  def test_file_arguments
    result = parse_tee_argv(["file1.txt", "file2.txt"])
    assert_equal ["file1.txt", "file2.txt"], result.arguments["files"]
  end
end

# ===========================================================================
# Test: tee_stream function
# ===========================================================================

class TestTeeStream < Minitest::Test
  def test_writes_to_stdout
    input = StringIO.new("hello world\n")
    output = capture_io { tee_stream(input, []) }[0]
    assert_equal "hello world\n", output
  end

  def test_writes_to_file_io
    input = StringIO.new("test data\n")
    file_io = StringIO.new
    capture_io { tee_stream(input, [file_io]) }
    assert_equal "test data\n", file_io.string
  end

  def test_writes_to_multiple_ios
    input = StringIO.new("multi\n")
    io1 = StringIO.new
    io2 = StringIO.new
    capture_io { tee_stream(input, [io1, io2]) }
    assert_equal "multi\n", io1.string
    assert_equal "multi\n", io2.string
  end

  def test_empty_input
    input = StringIO.new("")
    io1 = StringIO.new
    output = capture_io { tee_stream(input, [io1]) }[0]
    assert_equal "", output
    assert_equal "", io1.string
  end

  def test_large_input
    data = "x" * 20000 + "\n"
    input = StringIO.new(data)
    io1 = StringIO.new
    capture_io { tee_stream(input, [io1]) }
    assert_equal data, io1.string
  end

  def test_multiline_input
    input = StringIO.new("line 1\nline 2\nline 3\n")
    io1 = StringIO.new
    output = capture_io { tee_stream(input, [io1]) }[0]
    assert_equal "line 1\nline 2\nline 3\n", output
    assert_equal "line 1\nline 2\nline 3\n", io1.string
  end
end

# ===========================================================================
# Test: Main function with file writing
# ===========================================================================

class TestTeeMainFunction < Minitest::Test
  include TeeTestHelper

  def test_main_writes_to_file
    with_tempfile_path do |path|
      old_stdin = $stdin
      $stdin = StringIO.new("hello tee\n")
      old_argv = ARGV.dup
      ARGV.replace([path])
      capture_io { tee_main }
      assert_equal "hello tee\n", File.read(path)
    ensure
      $stdin = old_stdin
      ARGV.replace(old_argv)
    end
  end

  def test_main_append_mode
    with_tempfile_path do |path|
      File.write(path, "existing\n")
      old_stdin = $stdin
      $stdin = StringIO.new("appended\n")
      old_argv = ARGV.dup
      ARGV.replace(["-a", path])
      capture_io { tee_main }
      assert_equal "existing\nappended\n", File.read(path)
    ensure
      $stdin = old_stdin
      ARGV.replace(old_argv)
    end
  end

  def test_main_help_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) do
      capture_io { tee_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) do
      capture_io { tee_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end
end
