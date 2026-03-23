# frozen_string_literal: true

# test_cut.rb -- Tests for the Ruby cut tool
# =============================================
#
# === What These Tests Verify ===
#
# These tests exercise the cut tool's range parsing, byte/character
# selection, field selection, and CLI Builder integration. We test:
# - Range notation parsing (single, range, open-ended)
# - Byte mode (-b)
# - Character mode (-c)
# - Field mode (-f) with custom delimiters
# - Complement mode
# - Only-delimited mode (-s)
# - Output delimiter

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "tempfile"
require "coding_adventures_cli_builder"

# Load the cut_tool module so we can test the business logic functions.
require_relative "../cut_tool"

# ---------------------------------------------------------------------------
# Helper module
# ---------------------------------------------------------------------------

module CutTestHelper
  CUT_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "cut.json")

  def parse_cut_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(CUT_TEST_SPEC, ["cut"] + argv).parse
  end

  def with_tempfile(content)
    f = Tempfile.new("cut_test")
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

class TestCutCliIntegration < Minitest::Test
  include CutTestHelper

  def test_help_returns_help_result
    result = parse_cut_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
    assert_includes result.text, "cut"
  end

  def test_version_returns_version_result
    result = parse_cut_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_bytes_flag
    result = parse_cut_argv(["-b", "1-3"])
    assert_equal "1-3", result.flags["bytes"]
  end

  def test_fields_flag
    result = parse_cut_argv(["-f", "1,3"])
    assert_equal "1,3", result.flags["fields"]
  end

  def test_delimiter_flag
    result = parse_cut_argv(["-f", "1", "-d", ":"])
    assert_equal ":", result.flags["delimiter"]
  end
end

# ===========================================================================
# Test: parse_range_list
# ===========================================================================

class TestParseRangeList < Minitest::Test
  def test_single_position
    assert_equal [3], parse_range_list("3", 10)
  end

  def test_range
    assert_equal [2, 3, 4], parse_range_list("2-4", 10)
  end

  def test_open_start
    assert_equal [1, 2, 3], parse_range_list("-3", 10)
  end

  def test_open_end
    assert_equal [5, 6, 7, 8, 9, 10], parse_range_list("5-", 10)
  end

  def test_comma_separated
    assert_equal [1, 3, 5], parse_range_list("1,3,5", 10)
  end

  def test_mixed
    assert_equal [1, 2, 3, 5, 8, 9, 10], parse_range_list("1-3,5,8-", 10)
  end

  def test_out_of_range
    assert_equal [3], parse_range_list("3", 3)
    assert_equal [], parse_range_list("5", 3)
  end

  def test_duplicates_removed
    result = parse_range_list("1-3,2-4", 10)
    assert_equal [1, 2, 3, 4], result
  end
end

# ===========================================================================
# Test: cut_line_by_positions -- byte mode
# ===========================================================================

class TestCutLineByBytes < Minitest::Test
  def test_select_bytes
    result = cut_line_by_positions("hello", "1-3", :bytes, false, nil)
    assert_equal "hel", result
  end

  def test_select_single_byte
    result = cut_line_by_positions("hello", "2", :bytes, false, nil)
    assert_equal "e", result
  end

  def test_select_open_end
    result = cut_line_by_positions("hello", "3-", :bytes, false, nil)
    assert_equal "llo", result
  end

  def test_complement_bytes
    result = cut_line_by_positions("hello", "2-4", :bytes, true, nil)
    assert_equal "ho", result
  end
end

# ===========================================================================
# Test: cut_line_by_positions -- character mode
# ===========================================================================

class TestCutLineByChars < Minitest::Test
  def test_select_chars
    result = cut_line_by_positions("abcde", "1,3,5", :chars, false, nil)
    assert_equal "ace", result
  end

  def test_select_char_range
    result = cut_line_by_positions("abcdef", "2-4", :chars, false, nil)
    assert_equal "bcd", result
  end

  def test_complement_chars
    result = cut_line_by_positions("abcde", "2,4", :chars, true, nil)
    assert_equal "ace", result
  end
end

# ===========================================================================
# Test: cut_line_by_fields
# ===========================================================================

class TestCutLineByFields < Minitest::Test
  def test_select_field_tab_delimited
    result = cut_line_by_fields("a\tb\tc", "2", "\t", false, false, nil)
    assert_equal "b", result
  end

  def test_select_multiple_fields
    result = cut_line_by_fields("a\tb\tc\td", "1,3", "\t", false, false, nil)
    assert_equal "a\tc", result
  end

  def test_custom_delimiter
    result = cut_line_by_fields("a:b:c", "2", ":", false, false, nil)
    assert_equal "b", result
  end

  def test_only_delimited_suppresses
    result = cut_line_by_fields("no delimiter here", "1", "\t", false, true, nil)
    assert_nil result
  end

  def test_only_delimited_passes_with_delimiter
    result = cut_line_by_fields("a\tb", "1", "\t", false, true, nil)
    assert_equal "a", result
  end

  def test_no_delimiter_without_s_flag
    result = cut_line_by_fields("no delimiter", "1", "\t", false, false, nil)
    assert_equal "no delimiter", result
  end

  def test_output_delimiter
    result = cut_line_by_fields("a\tb\tc", "1,3", "\t", false, false, ",")
    assert_equal "a,c", result
  end

  def test_complement_fields
    result = cut_line_by_fields("a\tb\tc", "2", "\t", true, false, nil)
    assert_equal "a\tc", result
  end

  def test_field_range
    result = cut_line_by_fields("a:b:c:d:e", "2-4", ":", false, false, nil)
    assert_equal "b:c:d", result
  end
end

# ===========================================================================
# Test: cut_line dispatch function
# ===========================================================================

class TestCutLine < Minitest::Test
  def test_dispatch_bytes
    flags = {"bytes" => "1-3"}
    assert_equal "hel", cut_line("hello", flags)
  end

  def test_dispatch_chars
    flags = {"characters" => "1,5"}
    assert_equal "ho", cut_line("hello", flags)
  end

  def test_dispatch_fields
    flags = {"fields" => "2", "delimiter" => ":"}
    assert_equal "world", cut_line("hello:world", flags)
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestCutMainFunction < Minitest::Test
  include CutTestHelper

  def test_main_cut_bytes
    with_tempfile("hello\nworld\n") do |path|
      old_argv = ARGV.dup
      ARGV.replace(["-b", "1-3", path])
      output = capture_io { cut_main }[0]
      assert_equal "hel\nwor\n", output
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_cut_fields
    with_tempfile("a:b:c\n1:2:3\n") do |path|
      old_argv = ARGV.dup
      ARGV.replace(["-f", "2", "-d", ":", path])
      output = capture_io { cut_main }[0]
      assert_equal "b\n2\n", output
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_help_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) { capture_io { cut_main } }
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) { capture_io { cut_main } }
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end
end
