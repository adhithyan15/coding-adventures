# frozen_string_literal: true

# test_comm.rb -- Tests for the Ruby comm tool
# ===============================================
#
# === What These Tests Verify ===
#
# These tests exercise the comm tool's sorted-file comparison and
# three-column output. We test:
# - Default three-column output
# - Column suppression (-1, -2, -3)
# - Custom output delimiter
# - Edge cases (empty files, identical files, no overlap)

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "tempfile"
require "coding_adventures_cli_builder"

require_relative "../comm_tool"

# ---------------------------------------------------------------------------
# Helper module
# ---------------------------------------------------------------------------

module CommTestHelper
  COMM_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "comm.json")

  def parse_comm_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(COMM_TEST_SPEC, ["comm"] + argv).parse
  end

  def with_tempfile(content)
    f = Tempfile.new("comm_test")
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

class TestCommCliIntegration < Minitest::Test
  include CommTestHelper

  def test_help_returns_help_result
    result = parse_comm_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
    assert_includes result.text, "comm"
  end

  def test_version_returns_version_result
    result = parse_comm_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end
end

# ===========================================================================
# Test: compare_sorted -- default output
# ===========================================================================

class TestCompareSortedDefault < Minitest::Test
  def test_basic_comparison
    lines1 = ["a", "c", "e"]
    lines2 = ["b", "c", "d"]
    suppress = [false, false, false]
    result = compare_sorted(lines1, lines2, suppress, "\t")

    assert_equal 5, result.length
    assert_equal "a", result[0]           # unique to file1
    assert_equal "\tb", result[1]         # unique to file2
    assert_equal "\t\tc", result[2]       # common
    assert_equal "\td", result[3]         # unique to file2
    assert_equal "e", result[4]           # unique to file1
  end

  def test_identical_files
    lines1 = ["a", "b", "c"]
    lines2 = ["a", "b", "c"]
    suppress = [false, false, false]
    result = compare_sorted(lines1, lines2, suppress, "\t")

    assert_equal 3, result.length
    result.each { |line| assert line.start_with?("\t\t") }
  end

  def test_no_overlap
    lines1 = ["a", "c"]
    lines2 = ["b", "d"]
    suppress = [false, false, false]
    result = compare_sorted(lines1, lines2, suppress, "\t")

    assert_equal 4, result.length
    assert_equal "a", result[0]
    assert_equal "\tb", result[1]
    assert_equal "c", result[2]
    assert_equal "\td", result[3]
  end

  def test_empty_file1
    lines1 = []
    lines2 = ["a", "b"]
    suppress = [false, false, false]
    result = compare_sorted(lines1, lines2, suppress, "\t")

    assert_equal 2, result.length
    assert_equal "\ta", result[0]
    assert_equal "\tb", result[1]
  end

  def test_empty_file2
    lines1 = ["a", "b"]
    lines2 = []
    suppress = [false, false, false]
    result = compare_sorted(lines1, lines2, suppress, "\t")

    assert_equal 2, result.length
    assert_equal "a", result[0]
    assert_equal "b", result[1]
  end

  def test_both_empty
    result = compare_sorted([], [], [false, false, false], "\t")
    assert_equal [], result
  end
end

# ===========================================================================
# Test: compare_sorted -- column suppression
# ===========================================================================

class TestCompareSortedSuppression < Minitest::Test
  def test_suppress_col1
    lines1 = ["a", "c"]
    lines2 = ["b", "c"]
    suppress = [true, false, false]
    result = compare_sorted(lines1, lines2, suppress, "\t")

    # "a" is suppressed (col1), "b" shows without prefix (col1 suppressed),
    # "c" shows with one tab (col2 is the only prior non-suppressed column)
    refute result.any? { |l| l == "a" }
    assert result.any? { |l| l.include?("b") }
    assert result.any? { |l| l.include?("c") }
  end

  def test_suppress_col2
    lines1 = ["a", "c"]
    lines2 = ["b", "c"]
    suppress = [false, true, false]
    result = compare_sorted(lines1, lines2, suppress, "\t")

    refute result.any? { |l| l.include?("b") }
    assert result.any? { |l| l == "a" }
  end

  def test_suppress_col3
    lines1 = ["a", "c"]
    lines2 = ["b", "c"]
    suppress = [false, false, true]
    result = compare_sorted(lines1, lines2, suppress, "\t")

    # "c" (common) should not appear
    refute result.any? { |l| l.end_with?("c") && l.include?("\t\t") }
  end

  def test_suppress_all
    lines1 = ["a", "c"]
    lines2 = ["b", "c"]
    suppress = [true, true, true]
    result = compare_sorted(lines1, lines2, suppress, "\t")

    assert_equal [], result
  end

  def test_suppress_12_shows_only_common
    lines1 = ["a", "c", "e"]
    lines2 = ["b", "c", "d"]
    suppress = [true, true, false]
    result = compare_sorted(lines1, lines2, suppress, "\t")

    assert_equal 1, result.length
    assert_equal "c", result[0]
  end
end

# ===========================================================================
# Test: build_comm_prefix
# ===========================================================================

class TestBuildCommPrefix < Minitest::Test
  def test_col1_no_prefix
    prefix = build_comm_prefix(1, [false, false, false], "\t")
    assert_equal "", prefix
  end

  def test_col2_one_tab
    prefix = build_comm_prefix(2, [false, false, false], "\t")
    assert_equal "\t", prefix
  end

  def test_col3_two_tabs
    prefix = build_comm_prefix(3, [false, false, false], "\t")
    assert_equal "\t\t", prefix
  end

  def test_col2_with_col1_suppressed
    prefix = build_comm_prefix(2, [true, false, false], "\t")
    assert_equal "", prefix
  end

  def test_col3_with_col1_suppressed
    prefix = build_comm_prefix(3, [true, false, false], "\t")
    assert_equal "\t", prefix
  end

  def test_col3_with_both_suppressed
    prefix = build_comm_prefix(3, [true, true, false], "\t")
    assert_equal "", prefix
  end

  def test_custom_separator
    prefix = build_comm_prefix(3, [false, false, false], "||")
    assert_equal "||||", prefix
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestCommMainFunction < Minitest::Test
  include CommTestHelper

  def test_main_basic
    with_tempfile("a\nc\n") do |path1|
      with_tempfile("b\nc\n") do |path2|
        old_argv = ARGV.dup
        ARGV.replace([path1, path2])
        output = capture_io { comm_main }[0]
        assert_includes output, "a"
        assert_includes output, "b"
        assert_includes output, "c"
      ensure
        ARGV.replace(old_argv)
      end
    end
  end

  def test_main_suppress_col1
    with_tempfile("a\nc\n") do |path1|
      with_tempfile("b\nc\n") do |path2|
        old_argv = ARGV.dup
        ARGV.replace(["-1", path1, path2])
        output = capture_io { comm_main }[0]
        lines = output.split("\n")
        refute lines.any? { |l| l.strip == "a" && !l.include?("\t") }
      ensure
        ARGV.replace(old_argv)
      end
    end
  end

  def test_main_help_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) { capture_io { comm_main } }
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) { capture_io { comm_main } }
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end
end
