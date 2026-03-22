# frozen_string_literal: true

# test_tail.rb -- Tests for the Ruby tail tool
# ==============================================
#
# === What These Tests Verify ===
#
# These tests exercise the tail tool's CLI Builder integration and
# business logic. We test:
# - Default last-10-lines output
# - Custom line count (-n)
# - From-start mode (-n +NUM)
# - Byte mode (-c)
# - From-start byte mode (-c +NUM)
# - Header display for multiple files
# - The parse_tail_count, tail_lines, and tail_bytes functions directly

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "stringio"
require "tempfile"
require "coding_adventures_cli_builder"

# Load the tail_tool module so we can test the business logic functions.
require_relative "../tail_tool"

# ---------------------------------------------------------------------------
# Helper module: shared spec path and parse method for tail tests
# ---------------------------------------------------------------------------

module TailTestHelper
  TAIL_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "tail.json")

  def parse_tail_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(TAIL_TEST_SPEC, ["tail"] + argv).parse
  end

  def with_tempfile(content)
    f = Tempfile.new("tail_test")
    f.write(content)
    f.close
    yield f.path
  ensure
    f&.unlink
  end

  def generate_lines(count)
    (1..count).map { |i| "line #{i}" }.join("\n") + "\n"
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestTailCliIntegration < Minitest::Test
  include TailTestHelper

  def test_no_flags_returns_parse_result
    result = parse_tail_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_help_returns_help_result
    result = parse_tail_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
    assert_includes result.text, "tail"
  end

  def test_version_returns_version_result
    result = parse_tail_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_lines_flag
    result = parse_tail_argv(["-n", "5"])
    assert_equal "5", result.flags["lines"]
  end

  def test_quiet_flag
    result = parse_tail_argv(["-q"])
    assert result.flags["quiet"]
  end

  def test_verbose_flag
    result = parse_tail_argv(["-v"])
    assert result.flags["verbose"]
  end
end

# ===========================================================================
# Test: parse_tail_count function
# ===========================================================================

class TestParseTailCount < Minitest::Test
  def test_plain_number
    count, from_start = parse_tail_count("10")
    assert_equal 10, count
    refute from_start
  end

  def test_plus_prefix
    count, from_start = parse_tail_count("+3")
    assert_equal 3, count
    assert from_start
  end

  def test_minus_prefix
    count, from_start = parse_tail_count("-5")
    assert_equal 5, count
    refute from_start
  end

  def test_plus_one
    count, from_start = parse_tail_count("+1")
    assert_equal 1, count
    assert from_start
  end

  def test_large_number
    count, from_start = parse_tail_count("1000")
    assert_equal 1000, count
    refute from_start
  end

  def test_zero
    count, from_start = parse_tail_count("0")
    assert_equal 0, count
    refute from_start
  end
end

# ===========================================================================
# Test: tail_lines function
# ===========================================================================

class TestTailLines < Minitest::Test
  include TailTestHelper

  def test_last_10_lines
    input = generate_lines(15)
    io = StringIO.new(input)
    output = capture_io { tail_lines(io, 10, false, "\n") }[0]
    lines = output.split("\n")
    assert_equal 10, lines.length
    assert_equal "line 6", lines[0]
    assert_equal "line 15", lines[9]
  end

  def test_fewer_lines_than_requested
    input = generate_lines(3)
    io = StringIO.new(input)
    output = capture_io { tail_lines(io, 10, false, "\n") }[0]
    lines = output.split("\n")
    assert_equal 3, lines.length
  end

  def test_from_start
    input = generate_lines(5)
    io = StringIO.new(input)
    output = capture_io { tail_lines(io, 3, true, "\n") }[0]
    lines = output.split("\n")
    assert_equal 3, lines.length
    assert_equal "line 3", lines[0]
    assert_equal "line 5", lines[2]
  end

  def test_from_start_plus_one_is_all
    input = generate_lines(5)
    io = StringIO.new(input)
    output = capture_io { tail_lines(io, 1, true, "\n") }[0]
    lines = output.split("\n")
    assert_equal 5, lines.length
  end

  def test_empty_input
    io = StringIO.new("")
    output = capture_io { tail_lines(io, 10, false, "\n") }[0]
    assert_equal "", output
  end

  def test_custom_count
    input = generate_lines(10)
    io = StringIO.new(input)
    output = capture_io { tail_lines(io, 3, false, "\n") }[0]
    lines = output.split("\n")
    assert_equal 3, lines.length
    assert_equal "line 8", lines[0]
  end
end

# ===========================================================================
# Test: tail_bytes function
# ===========================================================================

class TestTailBytes < Minitest::Test
  def test_last_n_bytes
    io = StringIO.new("hello world")
    output = capture_io { tail_bytes(io, 5, false) }[0]
    assert_equal "world", output
  end

  def test_from_start_bytes
    io = StringIO.new("hello world")
    output = capture_io { tail_bytes(io, 7, true) }[0]
    assert_equal "world", output
  end

  def test_more_bytes_than_content
    io = StringIO.new("hi")
    output = capture_io { tail_bytes(io, 100, false) }[0]
    assert_equal "hi", output
  end

  def test_empty_input
    io = StringIO.new("")
    output = capture_io { tail_bytes(io, 10, false) }[0]
    assert_equal "", output
  end

  def test_from_start_plus_one_is_all
    io = StringIO.new("abcdef")
    output = capture_io { tail_bytes(io, 1, true) }[0]
    assert_equal "abcdef", output
  end

  def test_exact_byte_count
    io = StringIO.new("abcdef")
    output = capture_io { tail_bytes(io, 6, false) }[0]
    assert_equal "abcdef", output
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestTailMainFunction < Minitest::Test
  include TailTestHelper

  def test_main_reads_file
    with_tempfile(generate_lines(5)) do |path|
      old_argv = ARGV.dup
      ARGV.replace([path])
      output = capture_io { tail_main }[0]
      assert_includes output, "line 1"
      assert_includes output, "line 5"
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_last_3_lines
    with_tempfile(generate_lines(10)) do |path|
      old_argv = ARGV.dup
      ARGV.replace(["-n", "3", path])
      output = capture_io { tail_main }[0]
      lines = output.split("\n")
      assert_equal 3, lines.length
      assert_equal "line 8", lines[0]
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_nonexistent_file_prints_error
    old_argv = ARGV.dup
    ARGV.replace(["/nonexistent/file/xyz"])
    _stdout, stderr = capture_io { tail_main }
    assert_includes stderr, "No such file or directory"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_help_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) do
      capture_io { tail_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) do
      capture_io { tail_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_multiple_files_show_headers
    with_tempfile("aaa\n") do |path1|
      with_tempfile("bbb\n") do |path2|
        old_argv = ARGV.dup
        ARGV.replace([path1, path2])
        output = capture_io { tail_main }[0]
        assert_includes output, "==> #{path1} <=="
        assert_includes output, "==> #{path2} <=="
      ensure
        ARGV.replace(old_argv)
      end
    end
  end
end
