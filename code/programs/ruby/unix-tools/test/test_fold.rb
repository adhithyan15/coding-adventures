# frozen_string_literal: true

# test_fold.rb -- Tests for the Ruby fold tool
# ==============================================

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
  minimum_coverage 80
end

require "minitest/autorun"
require "coding_adventures_cli_builder"

require_relative "../fold_tool"

module FoldTestHelper
  FOLD_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "fold.json")

  def parse_fold_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(FOLD_TEST_SPEC, ["fold"] + argv).parse
  end
end

class TestFoldCliIntegration < Minitest::Test
  include FoldTestHelper

  def test_basic_parse
    result = parse_fold_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_help
    result = parse_fold_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_version
    result = parse_fold_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_width_flag
    result = parse_fold_argv(["-w", "40"])
    assert_equal 40, result.flags["width"]
  end

  def test_spaces_flag
    result = parse_fold_argv(["-s"])
    assert result.flags["spaces"]
  end

  def test_bytes_flag
    result = parse_fold_argv(["-b"])
    assert result.flags["bytes"]
  end
end

class TestFoldFoldLine < Minitest::Test
  def test_short_line_unchanged
    assert_equal "hello", fold_fold_line("hello", 80, break_at_spaces: false, count_bytes: false)
  end

  def test_exact_width
    assert_equal "a" * 10, fold_fold_line("a" * 10, 10, break_at_spaces: false, count_bytes: false)
  end

  def test_long_line_wrapped
    result = fold_fold_line("a" * 20, 10, break_at_spaces: false, count_bytes: false)
    assert_includes result, "\n"
    lines = result.split("\n")
    assert_equal 10, lines[0].length
  end

  def test_empty_string
    assert_equal "", fold_fold_line("", 80, break_at_spaces: false, count_bytes: false)
  end

  def test_width_1
    assert_equal "a\nb\nc", fold_fold_line("abc", 1, break_at_spaces: false, count_bytes: false)
  end

  def test_no_break_needed
    assert_equal "short", fold_fold_line("short", 100, break_at_spaces: false, count_bytes: false)
  end

  def test_break_at_spaces
    result = fold_fold_line("hello world this is long", 12, break_at_spaces: true, count_bytes: false)
    lines = result.split("\n")
    assert lines.length > 1
  end

  def test_break_at_spaces_no_space_available
    result = fold_fold_line("abcdefghij", 5, break_at_spaces: true, count_bytes: false)
    assert_includes result, "\n"
  end

  def test_count_bytes_mode
    result = fold_fold_line("abcde", 3, break_at_spaces: false, count_bytes: true)
    assert_includes result, "\n"
  end

  def test_zero_width_returns_unchanged
    assert_equal "hello", fold_fold_line("hello", 0, break_at_spaces: false, count_bytes: false)
  end

  def test_tab_handling
    result = fold_fold_line("\thello", 10, break_at_spaces: false, count_bytes: false)
    # Tab takes up to 8 columns, so \thello is 13 columns total, should wrap
    assert_includes result, "\n"
  end

  def test_backspace_handling
    result = fold_fold_line("abc\bd", 5, break_at_spaces: false, count_bytes: false)
    # \b reduces column by 1, so "abc\bd" is 3 columns (abc) - 1 (\b) + 1 (d) = 3
    refute_includes result, "\n"
  end

  def test_newline_resets_column
    result = fold_fold_line("abc\ndefghijklmnop", 5, break_at_spaces: false, count_bytes: false)
    lines = result.split("\n")
    assert_equal "abc", lines[0]
  end

  def test_break_at_spaces_with_count_bytes
    result = fold_fold_line("hello world test", 8, break_at_spaces: true, count_bytes: true)
    assert_includes result, "\n"
  end

  def test_break_at_spaces_backspace_in_after_break
    # Construct a line where after breaking at a space, the remaining portion
    # contains a backspace character
    result = fold_fold_line("abcde fg\bhi", 6, break_at_spaces: true, count_bytes: false)
    refute_nil result
  end

  def test_break_at_spaces_tab_in_after_break
    result = fold_fold_line("abcde f\thi", 6, break_at_spaces: true, count_bytes: false)
    refute_nil result
  end
end

class TestFoldMainIntegration < Minitest::Test
  def test_main_with_file
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "input.txt")
      File.write(path, "a" * 100 + "\n")
      old_argv = ARGV.dup
      ARGV.replace([path])
      out, _err = capture_io { fold_main }
      lines = out.split("\n")
      assert lines.length > 1
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_with_width
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "input.txt")
      File.write(path, "a" * 20 + "\n")
      old_argv = ARGV.dup
      ARGV.replace(["-w", "10", path])
      out, _err = capture_io { fold_main }
      lines = out.split("\n")
      assert lines.length > 1
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_nonexistent_file
    old_argv = ARGV.dup
    ARGV.replace(["/nonexistent/file.txt"])
    _out, err = capture_io { fold_main }
    assert_includes err, "No such file or directory"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_help
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    out, _err = capture_io do
      e = assert_raises(SystemExit) { fold_main }
      assert_equal 0, e.status
    end
    assert_includes out, "fold"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    out, _err = capture_io do
      e = assert_raises(SystemExit) { fold_main }
      assert_equal 0, e.status
    end
    assert_includes out, "1.0.0"
  ensure
    ARGV.replace(old_argv)
  end
end
