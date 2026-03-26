# frozen_string_literal: true

# test_head.rb -- Tests for the Ruby head tool
# ==============================================
#
# === What These Tests Verify ===
#
# These tests exercise the head tool's CLI Builder integration and
# business logic. We test:
# - Default line output (first 10 lines)
# - Custom line count (-n)
# - Byte mode (-c)
# - Header display for multiple files
# - Quiet mode (-q)
# - Verbose mode (-v)
# - The head_lines and head_bytes functions directly

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "stringio"
require "tempfile"
require "coding_adventures_cli_builder"

# Load the head_tool module so we can test the business logic functions.
require_relative "../head_tool"

# ---------------------------------------------------------------------------
# Helper module: shared spec path and parse method for head tests
# ---------------------------------------------------------------------------

module HeadTestHelper
  HEAD_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "head.json")

  def parse_head_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(HEAD_TEST_SPEC, ["head"] + argv).parse
  end

  # Create a temporary file with given content, yield its path, then clean up.
  def with_tempfile(content)
    f = Tempfile.new("head_test")
    f.write(content)
    f.close
    yield f.path
  ensure
    f&.unlink
  end

  # Generate a string with N lines: "line 1\nline 2\n...\nline N\n"
  def generate_lines(count)
    (1..count).map { |i| "line #{i}" }.join("\n") + "\n"
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestHeadCliIntegration < Minitest::Test
  include HeadTestHelper

  def test_no_flags_returns_parse_result
    result = parse_head_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_help_returns_help_result
    result = parse_head_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
    assert_includes result.text, "head"
  end

  def test_version_returns_version_result
    result = parse_head_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_lines_flag
    result = parse_head_argv(["-n", "5"])
    assert_equal 5, result.flags["lines"]
  end

  def test_bytes_flag
    result = parse_head_argv(["-c", "100"])
    assert_equal 100, result.flags["bytes"]
  end

  def test_quiet_flag
    result = parse_head_argv(["-q"])
    assert result.flags["quiet"]
  end

  def test_verbose_flag
    result = parse_head_argv(["-v"])
    assert result.flags["verbose"]
  end
end

# ===========================================================================
# Test: head_lines function
# ===========================================================================

class TestHeadLines < Minitest::Test
  include HeadTestHelper

  def test_default_10_lines
    input = generate_lines(15)
    io = StringIO.new(input)
    output = capture_io { head_lines(io, 10, "\n") }[0]
    lines = output.split("\n")
    assert_equal 10, lines.length
    assert_equal "line 1", lines[0]
    assert_equal "line 10", lines[9]
  end

  def test_fewer_lines_than_requested
    input = generate_lines(3)
    io = StringIO.new(input)
    output = capture_io { head_lines(io, 10, "\n") }[0]
    lines = output.split("\n")
    assert_equal 3, lines.length
  end

  def test_custom_line_count
    input = generate_lines(20)
    io = StringIO.new(input)
    output = capture_io { head_lines(io, 5, "\n") }[0]
    lines = output.split("\n")
    assert_equal 5, lines.length
    assert_equal "line 5", lines[4]
  end

  def test_single_line
    io = StringIO.new("only one\n")
    output = capture_io { head_lines(io, 10, "\n") }[0]
    assert_equal "only one\n", output
  end

  def test_empty_input
    io = StringIO.new("")
    output = capture_io { head_lines(io, 10, "\n") }[0]
    assert_equal "", output
  end

  def test_zero_lines
    io = StringIO.new("hello\nworld\n")
    output = capture_io { head_lines(io, 0, "\n") }[0]
    assert_equal "", output
  end
end

# ===========================================================================
# Test: head_bytes function
# ===========================================================================

class TestHeadBytes < Minitest::Test
  def test_first_n_bytes
    io = StringIO.new("hello world")
    output = capture_io { head_bytes(io, 5) }[0]
    assert_equal "hello", output
  end

  def test_fewer_bytes_than_requested
    io = StringIO.new("hi")
    output = capture_io { head_bytes(io, 100) }[0]
    assert_equal "hi", output
  end

  def test_exact_byte_count
    io = StringIO.new("abcdef")
    output = capture_io { head_bytes(io, 6) }[0]
    assert_equal "abcdef", output
  end

  def test_zero_bytes
    io = StringIO.new("hello")
    output = capture_io { head_bytes(io, 0) }[0]
    assert_equal "", output
  end

  def test_empty_input
    io = StringIO.new("")
    output = capture_io { head_bytes(io, 10) }[0]
    assert_equal "", output
  end

  def test_bytes_cuts_mid_line
    io = StringIO.new("hello\nworld\n")
    output = capture_io { head_bytes(io, 8) }[0]
    assert_equal "hello\nwo", output
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestHeadMainFunction < Minitest::Test
  include HeadTestHelper

  def test_main_reads_file
    with_tempfile(generate_lines(3)) do |path|
      old_argv = ARGV.dup
      ARGV.replace([path])
      output = capture_io { head_main }[0]
      assert_includes output, "line 1"
      assert_includes output, "line 3"
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_default_10_lines
    with_tempfile(generate_lines(15)) do |path|
      old_argv = ARGV.dup
      ARGV.replace([path])
      output = capture_io { head_main }[0]
      lines = output.split("\n")
      assert_equal 10, lines.length
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_custom_line_count
    with_tempfile(generate_lines(20)) do |path|
      old_argv = ARGV.dup
      ARGV.replace(["-n", "3", path])
      output = capture_io { head_main }[0]
      lines = output.split("\n")
      assert_equal 3, lines.length
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_multiple_files_show_headers
    with_tempfile("aaa\n") do |path1|
      with_tempfile("bbb\n") do |path2|
        old_argv = ARGV.dup
        ARGV.replace([path1, path2])
        output = capture_io { head_main }[0]
        assert_includes output, "==> #{path1} <=="
        assert_includes output, "==> #{path2} <=="
      ensure
        ARGV.replace(old_argv)
      end
    end
  end

  def test_main_nonexistent_file_prints_error
    old_argv = ARGV.dup
    ARGV.replace(["/nonexistent/file/xyz"])
    _stdout, stderr = capture_io { head_main }
    assert_includes stderr, "No such file or directory"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_help_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) do
      capture_io { head_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) do
      capture_io { head_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end
end
