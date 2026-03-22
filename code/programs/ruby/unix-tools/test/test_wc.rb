# frozen_string_literal: true

# test_wc.rb -- Tests for the Ruby wc tool
# ==========================================
#
# === What These Tests Verify ===
#
# These tests exercise the wc tool's CLI Builder integration and
# business logic. We test:
# - Line, word, and byte counting
# - Character counting (-m)
# - Maximum line length (-L)
# - Individual flag selection (-l, -w, -c)
# - Default output (lines + words + bytes)
# - Right-justified formatting
# - Total line for multiple files
# - The count_stream, format_counts, and compute_width functions directly

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "stringio"
require "tempfile"
require "coding_adventures_cli_builder"

# Load the wc_tool module so we can test the business logic functions.
require_relative "../wc_tool"

# ---------------------------------------------------------------------------
# Helper module: shared spec path and parse method for wc tests
# ---------------------------------------------------------------------------

module WcTestHelper
  WC_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "wc.json")

  def parse_wc_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(WC_TEST_SPEC, ["wc"] + argv).parse
  end

  # Create a temporary file with given content, yield its path, then clean up.
  def with_tempfile(content)
    f = Tempfile.new("wc_test")
    f.write(content)
    f.close
    yield f.path
  ensure
    f&.unlink
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestWcCliIntegration < Minitest::Test
  include WcTestHelper

  def test_no_flags_returns_parse_result
    result = parse_wc_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_help_returns_help_result
    result = parse_wc_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
    assert_includes result.text, "wc"
  end

  def test_version_returns_version_result
    result = parse_wc_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_lines_flag
    result = parse_wc_argv(["-l"])
    assert result.flags["lines"]
  end

  def test_words_flag
    result = parse_wc_argv(["-w"])
    assert result.flags["words"]
  end

  def test_bytes_flag
    result = parse_wc_argv(["-c"])
    assert result.flags["bytes"]
  end

  def test_chars_flag
    result = parse_wc_argv(["-m"])
    assert result.flags["chars"]
  end

  def test_max_line_length_flag
    result = parse_wc_argv(["-L"])
    assert result.flags["max_line_length"]
  end
end

# ===========================================================================
# Test: count_stream function
# ===========================================================================

class TestWcCountStream < Minitest::Test
  def test_simple_text
    io = StringIO.new("hello world\n")
    counts = count_stream(io, "test.txt")
    assert_equal 1, counts.lines
    assert_equal 2, counts.words
    assert_equal 12, counts.bytes
    assert_equal 12, counts.chars
    assert_equal "test.txt", counts.name
  end

  def test_empty_input
    io = StringIO.new("")
    counts = count_stream(io, nil)
    assert_equal 0, counts.lines
    assert_equal 0, counts.words
    assert_equal 0, counts.bytes
    assert_nil counts.name
  end

  def test_multiple_lines
    io = StringIO.new("line one\nline two\nline three\n")
    counts = count_stream(io, "multi.txt")
    assert_equal 3, counts.lines
    assert_equal 6, counts.words
  end

  def test_no_trailing_newline
    # A file without a trailing newline: "hello" has 0 newline characters.
    io = StringIO.new("hello")
    counts = count_stream(io, "no_nl.txt")
    assert_equal 0, counts.lines
    assert_equal 1, counts.words
    assert_equal 5, counts.bytes
  end

  def test_max_line_length
    io = StringIO.new("short\na longer line\nhi\n")
    counts = count_stream(io, "test.txt")
    assert_equal 13, counts.max_line_length  # "a longer line" = 13 chars
  end

  def test_blank_lines
    io = StringIO.new("\n\n\n")
    counts = count_stream(io, "blanks.txt")
    assert_equal 3, counts.lines
    assert_equal 0, counts.words
  end

  def test_only_whitespace
    io = StringIO.new("   \n  \n")
    counts = count_stream(io, "ws.txt")
    assert_equal 2, counts.lines
    assert_equal 0, counts.words
  end
end

# ===========================================================================
# Test: format_counts function
# ===========================================================================

class TestWcFormatCounts < Minitest::Test
  def test_default_format
    counts = FileCounts.new(10, 42, 280, 280, 15, "file.txt")
    # Default mode: lines, words, bytes
    output = format_counts(counts, {}, 3)
    assert_includes output, " 10"
    assert_includes output, " 42"
    assert_includes output, "280"
    assert_includes output, "file.txt"
  end

  def test_lines_only
    counts = FileCounts.new(10, 42, 280, 280, 15, "file.txt")
    output = format_counts(counts, { "lines" => true }, 3)
    assert_includes output, " 10"
    refute_includes output, " 42"
    refute_includes output, "280"
    assert_includes output, "file.txt"
  end

  def test_words_only
    counts = FileCounts.new(10, 42, 280, 280, 15, "file.txt")
    output = format_counts(counts, { "words" => true }, 3)
    refute_includes output, " 10"
    assert_includes output, " 42"
    assert_includes output, "file.txt"
  end

  def test_bytes_only
    counts = FileCounts.new(10, 42, 280, 280, 15, "file.txt")
    output = format_counts(counts, { "bytes" => true }, 3)
    assert_includes output, "280"
    assert_includes output, "file.txt"
  end

  def test_chars_only
    counts = FileCounts.new(10, 42, 280, 250, 15, "file.txt")
    output = format_counts(counts, { "chars" => true }, 3)
    assert_includes output, "250"
    assert_includes output, "file.txt"
  end

  def test_max_line_length_only
    counts = FileCounts.new(10, 42, 280, 280, 15, "file.txt")
    output = format_counts(counts, { "max_line_length" => true }, 3)
    assert_includes output, " 15"
    assert_includes output, "file.txt"
  end

  def test_no_name
    counts = FileCounts.new(10, 42, 280, 280, 15, nil)
    output = format_counts(counts, {}, 3)
    refute_includes output, "nil"
  end
end

# ===========================================================================
# Test: compute_width function
# ===========================================================================

class TestWcComputeWidth < Minitest::Test
  def test_single_digit_counts
    counts = [FileCounts.new(5, 8, 9, 9, 3, "a.txt")]
    width = compute_width(counts, {})
    assert_equal 1, width
  end

  def test_multi_digit_counts
    counts = [FileCounts.new(100, 5000, 30000, 30000, 80, "big.txt")]
    width = compute_width(counts, {})
    assert_equal 5, width  # 30000 has 5 digits
  end

  def test_lines_only_width
    counts = [FileCounts.new(100, 5000, 30000, 30000, 80, "big.txt")]
    width = compute_width(counts, { "lines" => true })
    assert_equal 3, width  # 100 has 3 digits
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestWcMainFunction < Minitest::Test
  include WcTestHelper

  def test_main_reads_file
    with_tempfile("hello world\n") do |path|
      old_argv = ARGV.dup
      ARGV.replace([path])
      output = capture_io { wc_main }[0]
      # Should contain line count (1), word count (2), byte count (12)
      assert_match(/1/, output)
      assert_match(/2/, output)
      assert_match(/12/, output)
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_multiple_files_shows_total
    with_tempfile("aaa\n") do |path1|
      with_tempfile("bbb\n") do |path2|
        old_argv = ARGV.dup
        ARGV.replace([path1, path2])
        output = capture_io { wc_main }[0]
        assert_includes output, "total"
      ensure
        ARGV.replace(old_argv)
      end
    end
  end

  def test_main_nonexistent_file_prints_error
    old_argv = ARGV.dup
    ARGV.replace(["/nonexistent/file/xyz"])
    _stdout, stderr = capture_io { wc_main }
    assert_includes stderr, "No such file or directory"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_help_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) do
      capture_io { wc_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) do
      capture_io { wc_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_lines_only
    with_tempfile("alpha\nbeta\ngamma\n") do |path|
      old_argv = ARGV.dup
      ARGV.replace(["-l", path])
      output = capture_io { wc_main }[0]
      assert_match(/3/, output)
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_words_only
    with_tempfile("one two three\n") do |path|
      old_argv = ARGV.dup
      ARGV.replace(["-w", path])
      output = capture_io { wc_main }[0]
      assert_match(/3/, output)
    ensure
      ARGV.replace(old_argv)
    end
  end
end
