# frozen_string_literal: true

# test_sort.rb -- Tests for the Ruby sort tool
# ===============================================
#
# === What These Tests Verify ===
#
# These tests exercise the sort tool's CLI Builder integration and
# business logic functions. We test:
# - Lexicographic sorting (default)
# - Numeric sorting (-n)
# - General numeric sorting (-g)
# - Human-readable sorting (-h)
# - Month sorting (-M)
# - Version sorting (-V)
# - Reverse mode (-r)
# - Unique mode (-u)
# - Case-insensitive sorting (-f)
# - Dictionary order (-d)
# - Key-based sorting (-k)
# - Check mode (-c)

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "tempfile"
require "coding_adventures_cli_builder"

# Load the sort_tool module so we can test the business logic functions.
require_relative "../sort_tool"

# ---------------------------------------------------------------------------
# Helper module: shared spec path and parse method for sort tests
# ---------------------------------------------------------------------------

module SortTestHelper
  SORT_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "sort.json")

  def parse_sort_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(SORT_TEST_SPEC, ["sort"] + argv).parse
  end

  def with_tempfile(content)
    f = Tempfile.new("sort_test")
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

class TestSortCliIntegration < Minitest::Test
  include SortTestHelper

  def test_no_flags_returns_parse_result
    result = parse_sort_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_help_returns_help_result
    result = parse_sort_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
    assert_includes result.text, "sort"
  end

  def test_version_returns_version_result
    result = parse_sort_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_reverse_flag
    result = parse_sort_argv(["-r"])
    assert result.flags["reverse"]
  end

  def test_numeric_sort_flag
    result = parse_sort_argv(["-n"])
    assert result.flags["numeric_sort"]
  end

  def test_unique_flag
    result = parse_sort_argv(["-u"])
    assert result.flags["unique"]
  end
end

# ===========================================================================
# Test: sort_lines -- default lexicographic sort
# ===========================================================================

class TestSortLinesDefault < Minitest::Test
  def test_alphabetical_sort
    lines = ["cherry", "apple", "banana"]
    flags = {}
    result = sort_lines(lines, flags)
    assert_equal ["apple", "banana", "cherry"], result
  end

  def test_already_sorted
    lines = ["a", "b", "c"]
    result = sort_lines(lines, {})
    assert_equal ["a", "b", "c"], result
  end

  def test_empty_input
    result = sort_lines([], {})
    assert_equal [], result
  end

  def test_single_line
    result = sort_lines(["hello"], {})
    assert_equal ["hello"], result
  end

  def test_case_sensitive_by_default
    lines = ["Banana", "apple", "Cherry"]
    result = sort_lines(lines, {})
    # Uppercase comes before lowercase in ASCII
    assert_equal ["Banana", "Cherry", "apple"], result
  end

  def test_duplicate_lines
    lines = ["b", "a", "b", "a"]
    result = sort_lines(lines, {})
    assert_equal ["a", "a", "b", "b"], result
  end
end

# ===========================================================================
# Test: sort_lines -- reverse sort
# ===========================================================================

class TestSortLinesReverse < Minitest::Test
  def test_reverse_sort
    lines = ["apple", "cherry", "banana"]
    result = sort_lines(lines, {"reverse" => true})
    assert_equal ["cherry", "banana", "apple"], result
  end

  def test_reverse_numeric
    lines = ["3", "1", "2"]
    result = sort_lines(lines, {"reverse" => true, "numeric_sort" => true})
    assert_equal ["3", "2", "1"], result
  end
end

# ===========================================================================
# Test: sort_lines -- numeric sort
# ===========================================================================

class TestSortLinesNumeric < Minitest::Test
  def test_numeric_sort
    lines = ["10", "2", "1", "20"]
    result = sort_lines(lines, {"numeric_sort" => true})
    assert_equal ["1", "2", "10", "20"], result
  end

  def test_numeric_sort_with_negative
    lines = ["5", "-3", "0", "10"]
    result = sort_lines(lines, {"numeric_sort" => true})
    assert_equal ["-3", "0", "5", "10"], result
  end

  def test_numeric_sort_with_decimals
    lines = ["1.5", "1.2", "1.10"]
    result = sort_lines(lines, {"numeric_sort" => true})
    assert_equal ["1.10", "1.2", "1.5"], result
  end

  def test_numeric_sort_non_numeric_treated_as_zero
    lines = ["abc", "3", "1"]
    result = sort_lines(lines, {"numeric_sort" => true})
    assert_equal ["abc", "1", "3"], result
  end
end

# ===========================================================================
# Test: sort_lines -- general numeric sort
# ===========================================================================

class TestSortLinesGeneralNumeric < Minitest::Test
  def test_general_numeric_sort
    lines = ["1e2", "50", "1.5e1"]
    result = sort_lines(lines, {"general_numeric_sort" => true})
    assert_equal ["1.5e1", "50", "1e2"], result
  end

  def test_general_numeric_with_non_numeric
    lines = ["abc", "10", "5"]
    result = sort_lines(lines, {"general_numeric_sort" => true})
    assert_equal ["abc", "5", "10"], result
  end
end

# ===========================================================================
# Test: sort_lines -- human-readable sort
# ===========================================================================

class TestSortLinesHumanReadable < Minitest::Test
  def test_human_sort
    lines = ["1G", "2K", "1M"]
    result = sort_lines(lines, {"human_numeric_sort" => true})
    assert_equal ["2K", "1M", "1G"], result
  end

  def test_human_sort_same_suffix
    lines = ["10K", "2K", "5K"]
    result = sort_lines(lines, {"human_numeric_sort" => true})
    assert_equal ["2K", "5K", "10K"], result
  end
end

# ===========================================================================
# Test: sort_lines -- month sort
# ===========================================================================

class TestSortLinesMonth < Minitest::Test
  def test_month_sort
    lines = ["MAR", "JAN", "FEB"]
    result = sort_lines(lines, {"month_sort" => true})
    assert_equal ["JAN", "FEB", "MAR"], result
  end

  def test_month_sort_case_insensitive
    lines = ["dec", "jan", "jun"]
    result = sort_lines(lines, {"month_sort" => true})
    assert_equal ["jan", "jun", "dec"], result
  end

  def test_month_sort_unknown_before_known
    lines = ["FEB", "XXX", "JAN"]
    result = sort_lines(lines, {"month_sort" => true})
    assert_equal ["XXX", "JAN", "FEB"], result
  end
end

# ===========================================================================
# Test: sort_lines -- version sort
# ===========================================================================

class TestSortLinesVersion < Minitest::Test
  def test_version_sort
    lines = ["file10", "file2", "file1"]
    result = sort_lines(lines, {"version_sort" => true})
    assert_equal ["file1", "file2", "file10"], result
  end

  def test_version_sort_dotted
    lines = ["1.10.1", "1.2.3", "1.1.0"]
    result = sort_lines(lines, {"version_sort" => true})
    assert_equal ["1.1.0", "1.2.3", "1.10.1"], result
  end
end

# ===========================================================================
# Test: sort_lines -- unique mode
# ===========================================================================

class TestSortLinesUnique < Minitest::Test
  def test_unique_removes_duplicates
    lines = ["b", "a", "b", "a", "c"]
    result = sort_lines(lines, {"unique" => true})
    assert_equal ["a", "b", "c"], result
  end

  def test_unique_case_insensitive
    lines = ["Apple", "apple", "BANANA", "banana"]
    result = sort_lines(lines, {"unique" => true, "ignore_case" => true})
    assert_equal 2, result.length
  end
end

# ===========================================================================
# Test: sort_lines -- case-insensitive sort
# ===========================================================================

class TestSortLinesCaseInsensitive < Minitest::Test
  def test_case_fold
    lines = ["Banana", "apple", "Cherry"]
    result = sort_lines(lines, {"ignore_case" => true})
    assert_equal ["apple", "Banana", "Cherry"], result
  end
end

# ===========================================================================
# Test: sort_lines -- dictionary order
# ===========================================================================

class TestSortLinesDictionary < Minitest::Test
  def test_dictionary_order
    lines = ["b!x", "a@y", "c#z"]
    result = sort_lines(lines, {"dictionary_order" => true})
    assert_equal ["a@y", "b!x", "c#z"], result
  end
end

# ===========================================================================
# Test: sort_lines -- key-based sort
# ===========================================================================

class TestSortLinesKey < Minitest::Test
  def test_sort_by_second_field
    lines = ["b 2", "a 3", "c 1"]
    result = sort_lines(lines, {"key" => ["2,2"]})
    assert_equal ["c 1", "b 2", "a 3"], result
  end

  def test_sort_by_key_with_separator
    lines = ["b:2", "a:3", "c:1"]
    result = sort_lines(lines, {"key" => ["2,2"], "field_separator" => ":"})
    assert_equal ["c:1", "b:2", "a:3"], result
  end
end

# ===========================================================================
# Test: version_compare helper
# ===========================================================================

class TestVersionCompare < Minitest::Test
  def test_equal_strings
    assert_equal 0, version_compare("abc", "abc")
  end

  def test_numeric_comparison
    assert_equal(-1, version_compare("file2", "file10"))
  end

  def test_pure_numbers
    assert_equal(-1, version_compare("1", "2"))
  end

  def test_mixed
    assert_equal 1, version_compare("z10", "a20")
  end
end

# ===========================================================================
# Test: parse_human_size helper
# ===========================================================================

class TestParseHumanSize < Minitest::Test
  def test_plain_number
    assert_in_delta 42.0, parse_human_size("42"), 0.01
  end

  def test_kilobytes
    assert_in_delta 2048.0, parse_human_size("2K"), 0.01
  end

  def test_megabytes
    assert_in_delta 1048576.0, parse_human_size("1M"), 0.01
  end

  def test_gigabytes
    assert_in_delta 1073741824.0, parse_human_size("1G"), 0.01
  end

  def test_empty_string
    assert_in_delta 0.0, parse_human_size(""), 0.01
  end

  def test_invalid_string
    assert_in_delta 0.0, parse_human_size("abc"), 0.01
  end
end

# ===========================================================================
# Test: check_sorted function
# ===========================================================================

class TestCheckSorted < Minitest::Test
  def test_sorted_returns_true
    lines = ["a", "b", "c"]
    assert check_sorted(lines, {})
  end

  def test_unsorted_returns_false
    lines = ["b", "a", "c"]
    _out, err = capture_io { refute check_sorted(lines, {}) }
    assert_includes err, "disorder"
  end

  def test_empty_is_sorted
    assert check_sorted([], {})
  end

  def test_single_element_is_sorted
    assert check_sorted(["a"], {})
  end

  def test_numeric_check
    lines = ["1", "2", "10"]
    assert check_sorted(lines, {"numeric_sort" => true})
  end

  def test_reverse_check
    lines = ["c", "b", "a"]
    assert check_sorted(lines, {"reverse" => true})
  end
end

# ===========================================================================
# Test: extract_sort_key function
# ===========================================================================

class TestExtractSortKey < Minitest::Test
  def test_no_key_returns_whole_line
    assert_equal "hello world", extract_sort_key("hello world", nil, nil)
  end

  def test_single_field
    assert_equal "world", extract_sort_key("hello world foo", "2,2", nil)
  end

  def test_field_range
    result = extract_sort_key("a b c d", "2,3", nil)
    assert_equal "b c", result
  end

  def test_with_separator
    assert_equal "b", extract_sort_key("a:b:c", "2,2", ":")
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestSortMainFunction < Minitest::Test
  include SortTestHelper

  def test_main_sorts_file
    with_tempfile("cherry\napple\nbanana\n") do |path|
      old_argv = ARGV.dup
      ARGV.replace([path])
      output = capture_io { sort_main }[0]
      assert_equal "apple\nbanana\ncherry\n", output
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_reverse_sort
    with_tempfile("a\nb\nc\n") do |path|
      old_argv = ARGV.dup
      ARGV.replace(["-r", path])
      output = capture_io { sort_main }[0]
      assert_equal "c\nb\na\n", output
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_numeric_sort
    with_tempfile("10\n2\n1\n") do |path|
      old_argv = ARGV.dup
      ARGV.replace(["-n", path])
      output = capture_io { sort_main }[0]
      assert_equal "1\n2\n10\n", output
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_unique
    with_tempfile("b\na\nb\na\n") do |path|
      old_argv = ARGV.dup
      ARGV.replace(["-u", path])
      output = capture_io { sort_main }[0]
      assert_equal "a\nb\n", output
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_help_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) { capture_io { sort_main } }
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) { capture_io { sort_main } }
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_check_sorted
    with_tempfile("a\nb\nc\n") do |path|
      old_argv = ARGV.dup
      ARGV.replace(["-c", path])
      err = assert_raises(SystemExit) { capture_io { sort_main } }
      assert_equal 0, err.status
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_check_unsorted
    with_tempfile("b\na\nc\n") do |path|
      old_argv = ARGV.dup
      ARGV.replace(["-c", path])
      err = assert_raises(SystemExit) { capture_io { sort_main } }
      assert_equal 1, err.status
    ensure
      ARGV.replace(old_argv)
    end
  end

  def test_main_output_to_file
    with_tempfile("c\na\nb\n") do |input_path|
      output_file = Tempfile.new("sort_output")
      output_path = output_file.path
      output_file.close

      old_argv = ARGV.dup
      ARGV.replace(["-o", output_path, input_path])
      capture_io { sort_main }
      assert_equal "a\nb\nc\n", File.read(output_path)
    ensure
      ARGV.replace(old_argv)
      output_file&.unlink
    end
  end
end
