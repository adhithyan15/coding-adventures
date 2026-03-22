# frozen_string_literal: true

# test_split.rb -- Tests for the Ruby split tool
# =================================================
#
# === What These Tests Verify ===
#
# These tests exercise the split tool's CLI Builder integration and
# business logic functions. We test:
# - Suffix generation (alphabetic, numeric, hex)
# - Size parsing (bytes, K, M, G)
# - Splitting by lines (-l)
# - Splitting by bytes (-b)
# - Splitting by number of chunks (-n)
# - Custom prefix
# - Custom suffix length (-a)
# - Additional suffix (--additional-suffix)
# - Edge cases: empty input, single line, exact boundaries

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "tmpdir"
require "fileutils"
require "coding_adventures_cli_builder"

require_relative "../split_tool"

# ---------------------------------------------------------------------------
# Helper module: shared spec path and parse method for split tests
# ---------------------------------------------------------------------------

module SplitTestHelper
  SPLIT_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "split.json")

  def parse_split_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(SPLIT_TEST_SPEC, ["split"] + argv).parse
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestSplitCliIntegration < Minitest::Test
  include SplitTestHelper

  def test_help_returns_help_result
    result = parse_split_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_help_text_contains_program_name
    result = parse_split_argv(["--help"])
    assert_includes result.text, "split"
  end

  def test_version_returns_version_result
    result = parse_split_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_basic_parse
    result = parse_split_argv(["file.txt"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_lines_flag
    result = parse_split_argv(["-l", "100", "file.txt"])
    assert_equal 100, result.flags["lines"]
  end

  def test_bytes_flag
    result = parse_split_argv(["-b", "1024", "file.txt"])
    assert_equal "1024", result.flags["bytes"]
  end

  def test_numeric_suffixes_flag
    result = parse_split_argv(["-d", "file.txt"])
    assert result.flags["numeric_suffixes"]
  end

  def test_hex_suffixes_flag
    result = parse_split_argv(["-x", "file.txt"])
    assert result.flags["hex_suffixes"]
  end

  def test_suffix_length_flag
    result = parse_split_argv(["-a", "3", "file.txt"])
    assert_equal 3, result.flags["suffix_length"]
  end

  def test_verbose_flag
    result = parse_split_argv(["--verbose", "file.txt"])
    assert result.flags["verbose"]
  end
end

# ===========================================================================
# Test: split_generate_suffix
# ===========================================================================

class TestSplitGenerateSuffix < Minitest::Test
  def test_alphabetic_suffix_first
    # The first suffix should be "aa".
    assert_equal "aa", split_generate_suffix(0, suffix_length: 2)
  end

  def test_alphabetic_suffix_second
    assert_equal "ab", split_generate_suffix(1, suffix_length: 2)
  end

  def test_alphabetic_suffix_wraps_to_next_letter
    # After 'az' (index 25) comes 'ba' (index 26).
    assert_equal "az", split_generate_suffix(25, suffix_length: 2)
    assert_equal "ba", split_generate_suffix(26, suffix_length: 2)
  end

  def test_alphabetic_suffix_with_length_3
    assert_equal "aaa", split_generate_suffix(0, suffix_length: 3)
    assert_equal "aab", split_generate_suffix(1, suffix_length: 3)
  end

  def test_numeric_suffix_first
    assert_equal "00", split_generate_suffix(0, suffix_length: 2, numeric: true)
  end

  def test_numeric_suffix_sequence
    assert_equal "01", split_generate_suffix(1, suffix_length: 2, numeric: true)
    assert_equal "09", split_generate_suffix(9, suffix_length: 2, numeric: true)
    assert_equal "10", split_generate_suffix(10, suffix_length: 2, numeric: true)
    assert_equal "99", split_generate_suffix(99, suffix_length: 2, numeric: true)
  end

  def test_hex_suffix_first
    assert_equal "00", split_generate_suffix(0, suffix_length: 2, hex: true)
  end

  def test_hex_suffix_sequence
    assert_equal "0a", split_generate_suffix(10, suffix_length: 2, hex: true)
    assert_equal "0f", split_generate_suffix(15, suffix_length: 2, hex: true)
    assert_equal "10", split_generate_suffix(16, suffix_length: 2, hex: true)
    assert_equal "ff", split_generate_suffix(255, suffix_length: 2, hex: true)
  end

  def test_alphabetic_overflow_raises
    # With suffix_length 2, max is 26^2 = 676.
    assert_raises(RuntimeError) do
      split_generate_suffix(676, suffix_length: 2)
    end
  end

  def test_numeric_overflow_raises
    # With suffix_length 2, max is 10^2 = 100.
    assert_raises(RuntimeError) do
      split_generate_suffix(100, suffix_length: 2, numeric: true)
    end
  end

  def test_hex_overflow_raises
    # With suffix_length 2, max is 16^2 = 256.
    assert_raises(RuntimeError) do
      split_generate_suffix(256, suffix_length: 2, hex: true)
    end
  end
end

# ===========================================================================
# Test: split_parse_size
# ===========================================================================

class TestSplitParseSize < Minitest::Test
  def test_plain_bytes
    assert_equal 1024, split_parse_size("1024")
  end

  def test_kilobytes
    assert_equal 1024, split_parse_size("1K")
    assert_equal 1024, split_parse_size("1k")
    assert_equal 1024, split_parse_size("1KB")
  end

  def test_megabytes
    assert_equal 1_048_576, split_parse_size("1M")
    assert_equal 1_048_576, split_parse_size("1MB")
  end

  def test_gigabytes
    assert_equal 1_073_741_824, split_parse_size("1G")
    assert_equal 1_073_741_824, split_parse_size("1GB")
  end

  def test_larger_numbers
    assert_equal 5120, split_parse_size("5K")
    assert_equal 10_485_760, split_parse_size("10M")
  end

  def test_invalid_size_raises
    assert_raises(RuntimeError) do
      split_parse_size("abc")
    end
  end
end

# ===========================================================================
# Test: split_by_lines
# ===========================================================================

class TestSplitByLines < Minitest::Test
  def test_basic_split
    content = "line1\nline2\nline3\nline4\nline5\n"
    chunks = split_by_lines(content, 2, "x")

    assert_equal 3, chunks.length
    assert_equal "xaa", chunks[0][0]
    assert_equal "line1\nline2\n", chunks[0][1]
    assert_equal "xab", chunks[1][0]
    assert_equal "line3\nline4\n", chunks[1][1]
    assert_equal "xac", chunks[2][0]
    assert_equal "line5\n", chunks[2][1]
  end

  def test_custom_prefix
    content = "a\nb\nc\n"
    chunks = split_by_lines(content, 1, "chunk_")
    assert_equal "chunk_aa", chunks[0][0]
    assert_equal "chunk_ab", chunks[1][0]
    assert_equal "chunk_ac", chunks[2][0]
  end

  def test_numeric_suffixes
    content = "a\nb\nc\n"
    chunks = split_by_lines(content, 1, "x", numeric: true)
    assert_equal "x00", chunks[0][0]
    assert_equal "x01", chunks[1][0]
    assert_equal "x02", chunks[2][0]
  end

  def test_hex_suffixes
    content = "a\nb\n"
    chunks = split_by_lines(content, 1, "x", hex: true)
    assert_equal "x00", chunks[0][0]
    assert_equal "x01", chunks[1][0]
  end

  def test_suffix_length_3
    content = "a\nb\n"
    chunks = split_by_lines(content, 1, "x", suffix_length: 3)
    assert_equal "xaaa", chunks[0][0]
    assert_equal "xaab", chunks[1][0]
  end

  def test_additional_suffix
    content = "a\nb\n"
    chunks = split_by_lines(content, 1, "x", additional_suffix: ".txt")
    assert_equal "xaa.txt", chunks[0][0]
    assert_equal "xab.txt", chunks[1][0]
  end

  def test_all_lines_in_one_chunk
    content = "a\nb\nc\n"
    chunks = split_by_lines(content, 10, "x")
    assert_equal 1, chunks.length
    assert_equal content, chunks[0][1]
  end

  def test_empty_content
    chunks = split_by_lines("", 10, "x")
    assert_equal 0, chunks.length
  end

  def test_single_line
    content = "only line\n"
    chunks = split_by_lines(content, 1, "x")
    assert_equal 1, chunks.length
    assert_equal "only line\n", chunks[0][1]
  end

  def test_exact_boundary
    # 4 lines split by 2 should give exactly 2 chunks.
    content = "1\n2\n3\n4\n"
    chunks = split_by_lines(content, 2, "x")
    assert_equal 2, chunks.length
  end
end

# ===========================================================================
# Test: split_by_bytes
# ===========================================================================

class TestSplitByBytes < Minitest::Test
  def test_basic_byte_split
    content = "abcdefghij"  # 10 bytes
    chunks = split_by_bytes(content, 3, "x")

    assert_equal 4, chunks.length
    assert_equal "xaa", chunks[0][0]
    assert_equal "abc", chunks[0][1]
    assert_equal "xab", chunks[1][0]
    assert_equal "def", chunks[1][1]
    assert_equal "xac", chunks[2][0]
    assert_equal "ghi", chunks[2][1]
    assert_equal "xad", chunks[3][0]
    assert_equal "j", chunks[3][1]
  end

  def test_exact_byte_boundary
    content = "abcdef"  # 6 bytes, split by 3
    chunks = split_by_bytes(content, 3, "x")
    assert_equal 2, chunks.length
    assert_equal "abc", chunks[0][1]
    assert_equal "def", chunks[1][1]
  end

  def test_single_byte_chunks
    content = "abc"
    chunks = split_by_bytes(content, 1, "x")
    assert_equal 3, chunks.length
  end

  def test_chunk_larger_than_content
    content = "hello"
    chunks = split_by_bytes(content, 100, "x")
    assert_equal 1, chunks.length
    assert_equal "hello", chunks[0][1]
  end

  def test_empty_content
    chunks = split_by_bytes("", 10, "x")
    assert_equal 0, chunks.length
  end

  def test_numeric_suffixes
    content = "abcdef"
    chunks = split_by_bytes(content, 2, "x", numeric: true)
    assert_equal "x00", chunks[0][0]
    assert_equal "x01", chunks[1][0]
    assert_equal "x02", chunks[2][0]
  end
end

# ===========================================================================
# Test: split_by_number
# ===========================================================================

class TestSplitByNumber < Minitest::Test
  def test_split_into_equal_chunks
    content = "123456789"  # 9 bytes, split into 3
    chunks = split_by_number(content, 3, "x")

    assert_equal 3, chunks.length
    # Each chunk should be about 3 bytes.
    assert_equal 3, chunks[0][1].length
    assert_equal 3, chunks[1][1].length
    assert_equal 3, chunks[2][1].length
  end

  def test_uneven_split
    content = "1234567890"  # 10 bytes, split into 3
    chunks = split_by_number(content, 3, "x")

    # Verify all content is preserved.
    reassembled = chunks.map { |_, c| c }.join
    assert_equal content.b, reassembled
  end

  def test_single_chunk
    content = "hello world"
    chunks = split_by_number(content, 1, "x")
    assert_equal 1, chunks.length
    assert_equal content.b, chunks[0][1]
  end

  def test_more_chunks_than_bytes
    content = "abc"  # 3 bytes, split into 5
    chunks = split_by_number(content, 5, "x")
    # Can only produce as many chunks as there are bytes.
    assert chunks.length <= 5
    reassembled = chunks.map { |_, c| c }.join
    assert_equal content.b, reassembled
  end

  def test_empty_content
    chunks = split_by_number("", 3, "x")
    assert_equal 0, chunks.length
  end

  def test_suffix_naming
    content = "abcdef"
    chunks = split_by_number(content, 2, "part_")
    assert_equal "part_aa", chunks[0][0]
    assert_equal "part_ab", chunks[1][0]
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestSplitMainFunction < Minitest::Test
  include SplitTestHelper

  def test_main_help_exits_with_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) do
      capture_io { split_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_with_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) do
      capture_io { split_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_splits_file
    Dir.mktmpdir do |tmp|
      input = File.join(tmp, "input.txt")
      File.write(input, "line1\nline2\nline3\nline4\n")

      # Change to tmp dir so output files land there.
      old_argv = ARGV.dup
      ARGV.replace(["-l", "2", input, File.join(tmp, "chunk_")])
      out, = capture_io do
        begin
          split_main
        rescue SystemExit
          # split_main may not exit on success
        end
      end

      # Verify output files were created.
      assert File.exist?(File.join(tmp, "chunk_aa"))
      assert File.exist?(File.join(tmp, "chunk_ab"))
      assert_equal "line1\nline2\n", File.read(File.join(tmp, "chunk_aa"))
      assert_equal "line3\nline4\n", File.read(File.join(tmp, "chunk_ab"))
    ensure
      ARGV.replace(old_argv)
    end
  end
end
