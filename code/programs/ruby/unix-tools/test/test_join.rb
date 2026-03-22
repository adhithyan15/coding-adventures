# frozen_string_literal: true

# test_join.rb -- Tests for the Ruby join tool
# ===============================================
#
# === What These Tests Verify ===
#
# These tests exercise the join tool's CLI Builder integration and
# business logic functions. We test:
# - Parsing lines with different separators
# - Formatting joined output
# - Basic merge join on field 1
# - Joining on different fields (-1, -2)
# - Custom field separator (-t)
# - Case-insensitive joining (-i)
# - Unpaired lines from file 1 (-a 1)
# - Unpaired lines from file 2 (-a 2)
# - Only unpaired mode (-v)
# - Custom empty field replacement (-e)
# - Output format specification (-o)
# - Header line handling (--header)
# - Duplicate key handling (cross-product)

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "tmpdir"
require "fileutils"
require "coding_adventures_cli_builder"

require_relative "../join_tool"

# ---------------------------------------------------------------------------
# Helper module: shared spec path and parse method for join tests
# ---------------------------------------------------------------------------

module JoinTestHelper
  JOIN_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "join.json")

  def parse_join_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(JOIN_TEST_SPEC, ["join"] + argv).parse
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestJoinCliIntegration < Minitest::Test
  include JoinTestHelper

  def test_help_returns_help_result
    result = parse_join_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_help_text_contains_program_name
    result = parse_join_argv(["--help"])
    assert_includes result.text, "join"
  end

  def test_version_returns_version_result
    result = parse_join_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_basic_parse_with_two_files
    result = parse_join_argv(["file1.txt", "file2.txt"])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_ignore_case_flag
    result = parse_join_argv(["-i", "f1", "f2"])
    assert result.flags["ignore_case"]
  end

  def test_separator_flag
    result = parse_join_argv(["-t", ",", "f1", "f2"])
    assert_equal ",", result.flags["separator"]
  end

  def test_field1_flag
    result = parse_join_argv(["-1", "2", "f1", "f2"])
    assert_equal 2, result.flags["field1"]
  end

  def test_field2_flag
    result = parse_join_argv(["-2", "3", "f1", "f2"])
    assert_equal 3, result.flags["field2"]
  end

  def test_header_flag
    result = parse_join_argv(["--header", "f1", "f2"])
    assert result.flags["header"]
  end
end

# ===========================================================================
# Test: join_parse_line
# ===========================================================================

class TestJoinParseLine < Minitest::Test
  def test_split_on_whitespace_by_default
    # Default behavior: split on whitespace runs.
    fields = join_parse_line("alice 100 math")
    assert_equal ["alice", "100", "math"], fields
  end

  def test_split_on_custom_separator
    fields = join_parse_line("alice,100,math", separator: ",")
    assert_equal ["alice", "100", "math"], fields
  end

  def test_split_preserves_empty_fields_with_separator
    # With an explicit separator, consecutive separators create empty fields.
    fields = join_parse_line("a,,b", separator: ",")
    assert_equal ["a", "", "b"], fields
  end

  def test_split_on_tab
    fields = join_parse_line("first\tsecond\tthird", separator: "\t")
    assert_equal ["first", "second", "third"], fields
  end

  def test_single_field_line
    fields = join_parse_line("onlyfield")
    assert_equal ["onlyfield"], fields
  end

  def test_empty_line
    fields = join_parse_line("")
    assert_equal [], fields
  end
end

# ===========================================================================
# Test: join_format_output
# ===========================================================================

class TestJoinFormatOutput < Minitest::Test
  def test_default_format
    # Default output: key, then non-key fields from file1, then file2.
    result = join_format_output("1", ["1", "Alice"], ["1", "A"],
                                 field1: 0, field2: 0)
    assert_equal "1 Alice A", result
  end

  def test_custom_separator_in_output
    result = join_format_output("1", ["1", "Alice"], ["1", "A"],
                                 field1: 0, field2: 0, separator: ",")
    assert_equal "1,Alice,A", result
  end

  def test_format_string_with_field_selectors
    # Format "1.2,2.2" means: field 2 of file 1, then field 2 of file 2.
    result = join_format_output("1", ["1", "Alice"], ["1", "A"],
                                 field1: 0, field2: 0, format: "1.2,2.2")
    assert_equal "Alice A", result
  end

  def test_format_string_with_join_field_zero
    # Format "0,1.2" means: join field, then field 2 of file 1.
    result = join_format_output("k", ["k", "val1"], ["k", "val2"],
                                 field1: 0, field2: 0, format: "0,1.2")
    assert_equal "k val1", result
  end

  def test_missing_field_uses_empty_replacement
    result = join_format_output("1", ["1", "Alice"], nil,
                                 field1: 0, field2: 0, format: "0,1.2,2.2",
                                 empty: "NONE")
    assert_equal "1 Alice NONE", result
  end

  def test_nil_fields2_in_default_format
    # When file2 is nil (unpaired line from file1), only file1 fields appear.
    result = join_format_output("1", ["1", "Alice"], nil,
                                 field1: 0, field2: 0)
    assert_equal "1 Alice", result
  end

  def test_nil_fields1_in_default_format
    result = join_format_output("1", nil, ["1", "B"],
                                 field1: 0, field2: 0)
    assert_equal "1 B", result
  end
end

# ===========================================================================
# Test: join_files
# ===========================================================================

class TestJoinFiles < Minitest::Test
  def test_basic_join_on_field1
    # Classic example: two sorted files joined on field 1.
    lines1 = ["1 Alice", "2 Bob", "3 Charlie"]
    lines2 = ["1 A", "2 B", "3 A"]

    result = join_files(lines1, lines2)

    assert_equal 3, result.length
    assert_equal "1 Alice A", result[0]
    assert_equal "2 Bob B", result[1]
    assert_equal "3 Charlie A", result[2]
  end

  def test_join_with_unmatched_keys
    # When keys don't match, those lines are silently dropped (default).
    lines1 = ["1 Alice", "2 Bob", "4 Diana"]
    lines2 = ["1 A", "3 C", "4 D"]

    result = join_files(lines1, lines2)

    assert_equal 2, result.length
    assert_equal "1 Alice A", result[0]
    assert_equal "4 Diana D", result[1]
  end

  def test_unpaired_from_file1
    # -a 1: also print unpairable lines from file 1.
    lines1 = ["1 Alice", "2 Bob"]
    lines2 = ["1 A"]

    result = join_files(lines1, lines2, unpaired: [1])

    assert_equal 2, result.length
    assert_equal "1 Alice A", result[0]
    assert_equal "2 Bob", result[1]
  end

  def test_unpaired_from_file2
    lines1 = ["1 Alice"]
    lines2 = ["1 A", "2 B"]

    result = join_files(lines1, lines2, unpaired: [2])

    assert_equal 2, result.length
    assert_equal "1 Alice A", result[0]
    assert_equal "2 B", result[1]
  end

  def test_unpaired_from_both_files
    lines1 = ["1 Alice", "3 Charlie"]
    lines2 = ["2 B", "3 C"]

    result = join_files(lines1, lines2, unpaired: [1, 2])

    assert_equal 3, result.length
    assert_equal "1 Alice", result[0]
    assert_equal "2 B", result[1]
    assert_equal "3 Charlie C", result[2]
  end

  def test_only_unpaired_from_file1
    # -v 1: only unpaired lines from file 1, suppress joined output.
    lines1 = ["1 Alice", "2 Bob", "3 Charlie"]
    lines2 = ["1 A", "3 C"]

    result = join_files(lines1, lines2, only_unpaired: 1)

    assert_equal 1, result.length
    assert_equal "2 Bob", result[0]
  end

  def test_only_unpaired_from_file2
    lines1 = ["1 Alice", "3 Charlie"]
    lines2 = ["1 A", "2 B", "3 C"]

    result = join_files(lines1, lines2, only_unpaired: 2)

    assert_equal 1, result.length
    assert_equal "2 B", result[0]
  end

  def test_join_on_different_fields
    # Join on field 2 of file 1 and field 1 of file 2.
    lines1 = ["Alice 1", "Bob 2", "Charlie 3"]
    lines2 = ["1 A", "2 B", "3 C"]

    result = join_files(lines1, lines2, field1: 2, field2: 1)

    assert_equal 3, result.length
    assert_equal "1 Alice A", result[0]
    assert_equal "2 Bob B", result[1]
    assert_equal "3 Charlie C", result[2]
  end

  def test_custom_separator
    lines1 = ["1,Alice", "2,Bob"]
    lines2 = ["1,A", "2,B"]

    result = join_files(lines1, lines2, separator: ",")

    assert_equal 2, result.length
    assert_equal "1,Alice,A", result[0]
    assert_equal "2,Bob,B", result[1]
  end

  def test_case_insensitive_join
    lines1 = ["a Alice", "B Bob"]
    lines2 = ["A grade1", "b grade2"]

    result = join_files(lines1, lines2, ignore_case: true)

    assert_equal 2, result.length
  end

  def test_header_handling
    lines1 = ["ID Name", "1 Alice", "2 Bob"]
    lines2 = ["ID Grade", "1 A", "2 B"]

    result = join_files(lines1, lines2, header: true)

    # The header line should be joined regardless of key matching.
    assert_equal 3, result.length
    assert_includes result[0], "Name"
    assert_includes result[0], "Grade"
  end

  def test_duplicate_keys_produce_cross_product
    # When both files have multiple lines with the same key, the output
    # should be the cross-product of those lines.
    lines1 = ["1 A", "1 B"]
    lines2 = ["1 X", "1 Y"]

    result = join_files(lines1, lines2)

    assert_equal 4, result.length
    assert_includes result, "1 A X"
    assert_includes result, "1 A Y"
    assert_includes result, "1 B X"
    assert_includes result, "1 B Y"
  end

  def test_empty_files
    result = join_files([], [])
    assert_equal [], result
  end

  def test_one_empty_file
    lines1 = ["1 Alice", "2 Bob"]
    result = join_files(lines1, [])
    assert_equal [], result
  end

  def test_one_empty_file_with_unpaired
    lines1 = ["1 Alice", "2 Bob"]
    result = join_files(lines1, [], unpaired: [1])
    assert_equal 2, result.length
  end

  def test_format_output_specification
    lines1 = ["1 Alice Smith"]
    lines2 = ["1 A"]

    result = join_files(lines1, lines2, format: "0,1.2,1.3,2.2")

    assert_equal 1, result.length
    assert_equal "1 Alice Smith A", result[0]
  end

  def test_empty_replacement
    lines1 = ["1 Alice"]
    lines2 = []

    result = join_files(lines1, lines2, unpaired: [1], format: "0,1.2,2.2", empty: "N/A")

    assert_equal 1, result.length
    assert_includes result[0], "N/A"
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestJoinMainFunction < Minitest::Test
  include JoinTestHelper

  def test_main_help_exits_with_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) do
      capture_io { join_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_with_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) do
      capture_io { join_main }
    end
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_joins_two_files
    Dir.mktmpdir do |tmp|
      f1 = File.join(tmp, "file1.txt")
      f2 = File.join(tmp, "file2.txt")
      File.write(f1, "1 Alice\n2 Bob\n")
      File.write(f2, "1 A\n2 B\n")

      old_argv = ARGV.dup
      ARGV.replace([f1, f2])
      out, = capture_io do
        begin
          join_main
        rescue SystemExit
          # join_main may not exit on success
        end
      end
      assert_includes out, "1 Alice A"
      assert_includes out, "2 Bob B"
    ensure
      ARGV.replace(old_argv)
    end
  end
end
