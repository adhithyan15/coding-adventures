# frozen_string_literal: true

# test_grep.rb -- Tests for the Ruby grep tool
# ===============================================
#
# === What These Tests Verify ===
#
# These tests exercise the grep tool's CLI Builder integration and
# business logic functions. We test:
# - Pattern building from strings and flags
# - Fixed-string matching (-F)
# - Case-insensitive matching (-i)
# - Whole-word matching (-w)
# - Whole-line matching (-x)
# - Inverted matching (-v)
# - Match counting (-c)
# - Files-with-matches mode (-l)
# - Files-without-match mode (-L)
# - Only-matching output (-o)
# - Line number output (-n)
# - Filename prefix (-H, -h)
# - Max count limiting (-m)
# - Multi-file searching

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "tmpdir"
require "fileutils"
require "coding_adventures_cli_builder"

require_relative "../grep_tool"

# ---------------------------------------------------------------------------
# Helper module: shared spec path and parse method for grep tests
# ---------------------------------------------------------------------------

module GrepTestHelper
  GREP_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "grep.json")

  def parse_grep_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(GREP_TEST_SPEC, ["grep"] + argv).parse
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestGrepCliIntegration < Minitest::Test
  include GrepTestHelper

  def test_help_returns_help_result
    result = parse_grep_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_help_text_contains_program_name
    result = parse_grep_argv(["--help"])
    assert_includes result.text, "grep"
  end

  def test_version_returns_version_result
    result = parse_grep_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_basic_parse_with_pattern
    result = parse_grep_argv(["hello", "file.txt"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_ignore_case_flag
    result = parse_grep_argv(["-i", "hello", "file.txt"])
    assert result.flags["ignore_case"]
  end

  def test_invert_match_flag
    result = parse_grep_argv(["-v", "hello", "file.txt"])
    assert result.flags["invert_match"]
  end

  def test_count_flag
    result = parse_grep_argv(["-c", "hello", "file.txt"])
    assert result.flags["count"]
  end

  def test_line_number_flag
    result = parse_grep_argv(["-n", "hello", "file.txt"])
    assert result.flags["line_number"]
  end

  def test_fixed_strings_flag
    result = parse_grep_argv(["-F", "hello", "file.txt"])
    assert result.flags["fixed_strings"]
  end

  def test_word_regexp_flag
    result = parse_grep_argv(["-w", "hello", "file.txt"])
    assert result.flags["word_regexp"]
  end

  def test_line_regexp_flag
    result = parse_grep_argv(["-x", "hello", "file.txt"])
    assert result.flags["line_regexp"]
  end

  def test_files_with_matches_flag
    result = parse_grep_argv(["-l", "hello", "file.txt"])
    assert result.flags["files_with_matches"]
  end

  def test_files_without_match_flag
    result = parse_grep_argv(["-L", "hello", "file.txt"])
    assert result.flags["files_without_match"]
  end

  def test_only_matching_flag
    result = parse_grep_argv(["-o", "hello", "file.txt"])
    assert result.flags["only_matching"]
  end

  def test_max_count_flag
    result = parse_grep_argv(["-m", "5", "hello", "file.txt"])
    assert_equal 5, result.flags["max_count"]
  end
end

# ===========================================================================
# Test: grep_build_pattern
# ===========================================================================

class TestGrepBuildPattern < Minitest::Test
  def test_simple_pattern
    # A basic regex pattern should compile without error.
    pattern = grep_build_pattern("hello")
    assert_instance_of Regexp, pattern
    assert pattern.match?("hello world")
    refute pattern.match?("goodbye")
  end

  def test_fixed_strings_escape_metacharacters
    # With -F, regex metacharacters like . and * should be literal.
    pattern = grep_build_pattern("a.b", fixed_strings: true)
    assert pattern.match?("a.b")
    refute pattern.match?("axb"), ". should be literal, not wildcard"
  end

  def test_case_insensitive
    pattern = grep_build_pattern("hello", ignore_case: true)
    assert pattern.match?("HELLO")
    assert pattern.match?("Hello")
    assert pattern.match?("hello")
  end

  def test_word_boundary_matching
    # -w wraps the pattern in \b...\b so it matches whole words only.
    pattern = grep_build_pattern("cat", word_regexp: true)
    assert pattern.match?("the cat sat")
    refute pattern.match?("concatenate"), "cat inside another word should not match"
  end

  def test_line_matching
    # -x wraps the pattern in ^...\z so it must match the entire line.
    pattern = grep_build_pattern("hello", line_regexp: true)
    assert pattern.match?("hello")
    refute pattern.match?("hello world"), "partial line should not match"
  end

  def test_regex_pattern
    # A real regex pattern with character class and quantifier.
    pattern = grep_build_pattern("[0-9]+")
    assert pattern.match?("abc123def")
    refute pattern.match?("no digits here")
  end

  def test_combined_flags
    # Case-insensitive + word boundary.
    pattern = grep_build_pattern("Cat", ignore_case: true, word_regexp: true)
    assert pattern.match?("the CAT sat")
    refute pattern.match?("concatenate")
  end
end

# ===========================================================================
# Test: grep_match
# ===========================================================================

class TestGrepMatch < Minitest::Test
  def test_matching_line_returns_true
    pattern = grep_build_pattern("hello")
    assert grep_match("hello world", pattern)
  end

  def test_non_matching_line_returns_false
    pattern = grep_build_pattern("hello")
    refute grep_match("goodbye world", pattern)
  end

  def test_inverted_match_returns_opposite
    pattern = grep_build_pattern("hello")
    # Matching line with invert => false.
    refute grep_match("hello world", pattern, invert_match: true)
    # Non-matching line with invert => true.
    assert grep_match("goodbye world", pattern, invert_match: true)
  end
end

# ===========================================================================
# Test: grep_file
# ===========================================================================

class TestGrepFile < Minitest::Test
  def test_search_lines_array
    lines = ["apple pie", "banana split", "apple sauce", "cherry tart"]
    pattern = grep_build_pattern("apple")
    results = grep_file(lines, pattern)
    assert_equal 2, results.length
    assert_includes results, "apple pie"
    assert_includes results, "apple sauce"
  end

  def test_search_with_invert
    lines = ["apple", "banana", "cherry"]
    pattern = grep_build_pattern("banana")
    results = grep_file(lines, pattern, invert_match: true)
    assert_equal 2, results.length
    assert_includes results, "apple"
    assert_includes results, "cherry"
  end

  def test_count_mode
    lines = ["apple", "banana", "apple pie", "cherry"]
    pattern = grep_build_pattern("apple")
    results = grep_file(lines, pattern, count: true)
    assert_equal ["2"], results
  end

  def test_count_mode_with_filename
    lines = ["apple", "banana", "apple pie"]
    pattern = grep_build_pattern("apple")
    results = grep_file(lines, pattern, count: true, with_filename: true, filename: "test.txt")
    assert_equal ["test.txt:2"], results
  end

  def test_line_number_output
    lines = ["first", "second", "third"]
    pattern = grep_build_pattern("second")
    results = grep_file(lines, pattern, line_number: true)
    assert_equal ["2:second"], results
  end

  def test_with_filename_prefix
    lines = ["hello world"]
    pattern = grep_build_pattern("hello")
    results = grep_file(lines, pattern, with_filename: true, filename: "data.txt")
    assert_equal ["data.txt:hello world"], results
  end

  def test_line_number_and_filename
    lines = ["match here"]
    pattern = grep_build_pattern("match")
    results = grep_file(lines, pattern, line_number: true, with_filename: true, filename: "f.txt")
    assert_equal ["f.txt:1:match here"], results
  end

  def test_max_count_limits_results
    lines = ["a1", "a2", "a3", "a4", "a5"]
    pattern = grep_build_pattern("a")
    results = grep_file(lines, pattern, max_count: 3)
    assert_equal 3, results.length
  end

  def test_only_matching_output
    lines = ["hello world", "goodbye world"]
    pattern = grep_build_pattern("hello")
    results = grep_file(lines, pattern, only_matching: true)
    assert_equal ["hello"], results
  end

  def test_files_with_matches_returns_filename
    lines = ["hello world"]
    pattern = grep_build_pattern("hello")
    results = grep_file(lines, pattern, files_with_matches: true, filename: "test.txt")
    assert_equal ["test.txt"], results
  end

  def test_files_with_matches_no_match_returns_empty
    lines = ["goodbye world"]
    pattern = grep_build_pattern("hello")
    results = grep_file(lines, pattern, files_with_matches: true, filename: "test.txt")
    assert_equal [], results
  end

  def test_files_without_match_returns_filename_when_no_match
    lines = ["goodbye world"]
    pattern = grep_build_pattern("hello")
    results = grep_file(lines, pattern, files_without_match: true, filename: "test.txt")
    assert_equal ["test.txt"], results
  end

  def test_files_without_match_returns_empty_when_match_found
    lines = ["hello world"]
    pattern = grep_build_pattern("hello")
    results = grep_file(lines, pattern, files_without_match: true, filename: "test.txt")
    assert_equal [], results
  end

  def test_search_file_on_disk
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "data.txt")
      File.write(path, "line one\nline two\nline three\n")
      pattern = grep_build_pattern("two")
      results = grep_file(path, pattern)
      assert_equal 1, results.length
      assert_includes results[0], "two"
    end
  end

  def test_search_nonexistent_file_returns_nil
    pattern = grep_build_pattern("hello")
    results = capture_io do
      @result = grep_file("/nonexistent/path/file.txt", pattern)
    end
    assert_nil @result
  end

  def test_empty_input_returns_empty
    lines = []
    pattern = grep_build_pattern("hello")
    results = grep_file(lines, pattern)
    assert_equal [], results
  end

  def test_only_matching_with_regex_groups
    lines = ["abc123def456"]
    pattern = grep_build_pattern("[0-9]+")
    results = grep_file(lines, pattern, only_matching: true)
    # Should find both "123" and "456".
    assert_equal 2, results.length
    assert_includes results, "123"
    assert_includes results, "456"
  end

  def test_count_with_invert
    lines = ["apple", "banana", "cherry"]
    pattern = grep_build_pattern("banana")
    results = grep_file(lines, pattern, count: true, invert_match: true)
    assert_equal ["2"], results
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestGrepMainFunction < Minitest::Test
  def test_main_help_exits_with_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) do
      capture_io { grep_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_with_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) do
      capture_io { grep_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_searches_file
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "data.txt")
      File.write(path, "hello world\ngoodbye world\nhello again\n")

      old_argv = ARGV.dup
      ARGV.replace(["hello", path])
      out, = capture_io do
        begin
          grep_main
        rescue SystemExit
          # grep exits 0 (found) or 1 (not found)
        end
      end
      assert_includes out, "hello world"
      assert_includes out, "hello again"
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_exits_1_when_no_match
    Dir.mktmpdir do |tmp|
      path = File.join(tmp, "data.txt")
      File.write(path, "nothing matches\n")

      old_argv = ARGV.dup
      ARGV.replace(["zzzzz", path])
      err = assert_raises(SystemExit) do
        capture_io { grep_main }
      end
      assert_equal 1, err.status
    ensure
      ARGV.replace(old_argv)
    end
  end
end
