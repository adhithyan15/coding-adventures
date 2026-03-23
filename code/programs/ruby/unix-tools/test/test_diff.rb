# frozen_string_literal: true

# test_diff.rb -- Tests for the Ruby diff tool
# ==============================================
#
# === What These Tests Verify ===
#
# These tests exercise the diff tool's LCS-based comparison engine
# and all output formats. We test:
# - LCS table computation and edit backtracking
# - Normal, unified, and context output formats
# - Ignore case (-i), ignore whitespace (-b, -w), ignore blank lines (-B)
# - Brief mode (-q)
# - Identical files
# - Recursive directory comparison (-r)
# - Hunk grouping with context lines

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "tmpdir"
require "fileutils"
require "coding_adventures_cli_builder"

require_relative "../diff_tool"

# ---------------------------------------------------------------------------
# Helper module
# ---------------------------------------------------------------------------

module DiffTestHelper
  DIFF_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "diff.json")

  def parse_diff_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(DIFF_TEST_SPEC, ["diff"] + argv).parse
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestDiffCliIntegration < Minitest::Test
  include DiffTestHelper

  def test_help_returns_help_result
    result = parse_diff_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
  end

  def test_help_text_contains_program_name
    result = parse_diff_argv(["--help"])
    assert_includes result.text, "diff"
  end

  def test_version_returns_version_result
    result = parse_diff_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end
end

# ===========================================================================
# Test: Line normalization
# ===========================================================================

class TestDiffNormalizeLine < Minitest::Test
  def test_no_normalization
    assert_equal "Hello World", diff_normalize_line("Hello World")
  end

  def test_ignore_case
    assert_equal "hello world", diff_normalize_line("Hello World", ignore_case: true)
  end

  def test_ignore_all_space
    assert_equal "HelloWorld", diff_normalize_line("Hello  World", ignore_all_space: true)
  end

  def test_ignore_space_change
    assert_equal "Hello World", diff_normalize_line("Hello   World", ignore_space_change: true)
  end

  def test_ignore_case_and_space
    result = diff_normalize_line("Hello   World", ignore_case: true, ignore_space_change: true)
    assert_equal "hello world", result
  end

  def test_empty_line
    assert_equal "", diff_normalize_line("")
  end
end

# ===========================================================================
# Test: LCS computation
# ===========================================================================

class TestDiffLCS < Minitest::Test
  def test_identical_files
    lines = %w[a b c]
    edits = compute_edits(lines, lines)
    assert edits.all? { |e| e[:type] == :equal }
  end

  def test_completely_different_files
    edits = compute_edits(%w[a b], %w[c d])
    deletes = edits.select { |e| e[:type] == :delete }
    inserts = edits.select { |e| e[:type] == :insert }
    assert_equal 2, deletes.length
    assert_equal 2, inserts.length
  end

  def test_addition_only
    edits = compute_edits(%w[a b], %w[a b c])
    inserts = edits.select { |e| e[:type] == :insert }
    assert_equal 1, inserts.length
    assert_equal "c", inserts.first[:line]
  end

  def test_deletion_only
    edits = compute_edits(%w[a b c], %w[a c])
    deletes = edits.select { |e| e[:type] == :delete }
    assert_equal 1, deletes.length
    assert_equal "b", deletes.first[:line]
  end

  def test_empty_files
    edits = compute_edits([], [])
    assert_empty edits
  end

  def test_first_file_empty
    edits = compute_edits([], %w[a b])
    assert_equal 2, edits.select { |e| e[:type] == :insert }.length
  end

  def test_second_file_empty
    edits = compute_edits(%w[a b], [])
    assert_equal 2, edits.select { |e| e[:type] == :delete }.length
  end

  def test_lcs_table_dimensions
    lines_a = %w[a b c]
    lines_b = %w[x y]
    table = compute_lcs_table(lines_a, lines_b)
    assert_equal 4, table.length      # m+1 rows
    assert_equal 3, table[0].length   # n+1 columns
  end

  def test_lcs_with_ignore_case
    edits = compute_edits(["Hello"], ["hello"], ignore_case: true)
    assert edits.all? { |e| e[:type] == :equal }
  end
end

# ===========================================================================
# Test: Hunk grouping
# ===========================================================================

class TestDiffHunkGrouping < Minitest::Test
  def test_no_changes_produces_no_hunks
    edits = [{ type: :equal, line: "a", pos_a: 1, pos_b: 1 }]
    hunks = group_edits_into_hunks(edits)
    assert_empty hunks
  end

  def test_single_change_produces_one_hunk
    edits = [
      { type: :equal, line: "a", pos_a: 1, pos_b: 1 },
      { type: :delete, line: "b", pos_a: 2 },
      { type: :equal, line: "c", pos_a: 3, pos_b: 2 },
    ]
    hunks = group_edits_into_hunks(edits, 1)
    assert_equal 1, hunks.length
  end

  def test_distant_changes_produce_separate_hunks
    # Build edits with two changes far apart
    edits = []
    edits << { type: :delete, line: "x", pos_a: 1 }
    10.times do |i|
      edits << { type: :equal, line: "eq#{i}", pos_a: i + 2, pos_b: i + 1 }
    end
    edits << { type: :insert, line: "y", pos_b: 11 }

    hunks = group_edits_into_hunks(edits, 2)
    assert_equal 2, hunks.length
  end

  def test_empty_edits
    hunks = group_edits_into_hunks([])
    assert_empty hunks
  end
end

# ===========================================================================
# Test: Normal format output
# ===========================================================================

class TestDiffNormalFormat < Minitest::Test
  def test_change_format
    edits = compute_edits(%w[a b c], %w[a x c])
    output = format_normal(edits)
    assert_includes output, "< b"
    assert_includes output, "> x"
    assert_includes output, "---"
  end

  def test_addition_format
    edits = compute_edits(%w[a c], %w[a b c])
    output = format_normal(edits)
    assert_includes output, "> b"
    assert_includes output, "a"
  end

  def test_deletion_format
    edits = compute_edits(%w[a b c], %w[a c])
    output = format_normal(edits)
    assert_includes output, "< b"
  end

  def test_identical_produces_empty
    edits = compute_edits(%w[a b], %w[a b])
    output = format_normal(edits)
    assert_equal "", output
  end
end

# ===========================================================================
# Test: Unified format output
# ===========================================================================

class TestDiffUnifiedFormat < Minitest::Test
  def test_unified_header
    edits = compute_edits(%w[a b c], %w[a x c])
    output = format_unified(edits, "file1.txt", "file2.txt", 1)
    assert_includes output, "--- file1.txt"
    assert_includes output, "+++ file2.txt"
  end

  def test_unified_hunk_header
    edits = compute_edits(%w[a b c], %w[a x c])
    output = format_unified(edits, "file1.txt", "file2.txt", 1)
    assert_match(/@@ .* @@/, output)
  end

  def test_unified_change_markers
    edits = compute_edits(%w[a b c], %w[a x c])
    output = format_unified(edits, "file1.txt", "file2.txt", 1)
    assert_includes output, "-b"
    assert_includes output, "+x"
  end

  def test_unified_context_lines
    edits = compute_edits(%w[a b c], %w[a x c])
    output = format_unified(edits, "file1.txt", "file2.txt", 1)
    assert_includes output, " a"
    assert_includes output, " c"
  end

  def test_identical_produces_empty
    edits = compute_edits(%w[a b], %w[a b])
    output = format_unified(edits, "f1", "f2")
    assert_equal "", output
  end
end

# ===========================================================================
# Test: Context format output
# ===========================================================================

class TestDiffContextFormat < Minitest::Test
  def test_context_header
    edits = compute_edits(%w[a b c], %w[a x c])
    output = format_context(edits, "file1.txt", "file2.txt", 1)
    assert_includes output, "*** file1.txt"
    assert_includes output, "--- file2.txt"
  end

  def test_context_separator
    edits = compute_edits(%w[a b c], %w[a x c])
    output = format_context(edits, "f1", "f2", 1)
    assert_includes output, "***************"
  end

  def test_context_old_section
    edits = compute_edits(%w[a b c], %w[a x c])
    output = format_context(edits, "f1", "f2", 1)
    assert_includes output, "- b"
  end

  def test_context_new_section
    edits = compute_edits(%w[a b c], %w[a x c])
    output = format_context(edits, "f1", "f2", 1)
    assert_includes output, "+ x"
  end

  def test_identical_produces_empty
    edits = compute_edits(%w[a b], %w[a b])
    output = format_context(edits, "f1", "f2")
    assert_equal "", output
  end
end

# ===========================================================================
# Test: diff_files integration
# ===========================================================================

class TestDiffFiles < Minitest::Test
  def setup
    @tmpdir = Dir.mktmpdir("diff_test")
  end

  def teardown
    FileUtils.rm_rf(@tmpdir)
  end

  def test_identical_files
    f1 = File.join(@tmpdir, "a.txt")
    f2 = File.join(@tmpdir, "b.txt")
    File.write(f1, "hello\nworld\n")
    File.write(f2, "hello\nworld\n")

    output, code = diff_files(f1, f2)
    assert_equal 0, code
    assert_equal "", output
  end

  def test_different_files
    f1 = File.join(@tmpdir, "a.txt")
    f2 = File.join(@tmpdir, "b.txt")
    File.write(f1, "hello\nworld\n")
    File.write(f2, "hello\nearth\n")

    output, code = diff_files(f1, f2)
    assert_equal 1, code
    refute_empty output
  end

  def test_brief_mode
    f1 = File.join(@tmpdir, "a.txt")
    f2 = File.join(@tmpdir, "b.txt")
    File.write(f1, "hello\n")
    File.write(f2, "world\n")

    output, code = diff_files(f1, f2, brief: true)
    assert_equal 1, code
    assert_includes output, "differ"
  end

  def test_brief_identical
    f1 = File.join(@tmpdir, "a.txt")
    f2 = File.join(@tmpdir, "b.txt")
    File.write(f1, "same\n")
    File.write(f2, "same\n")

    output, code = diff_files(f1, f2, brief: true)
    assert_equal 0, code
    assert_equal "", output
  end

  def test_unified_format
    f1 = File.join(@tmpdir, "a.txt")
    f2 = File.join(@tmpdir, "b.txt")
    File.write(f1, "a\nb\nc\n")
    File.write(f2, "a\nx\nc\n")

    output, code = diff_files(f1, f2, format: :unified, context_size: 1)
    assert_equal 1, code
    assert_includes output, "---"
    assert_includes output, "+++"
  end

  def test_context_format
    f1 = File.join(@tmpdir, "a.txt")
    f2 = File.join(@tmpdir, "b.txt")
    File.write(f1, "a\nb\nc\n")
    File.write(f2, "a\nx\nc\n")

    output, code = diff_files(f1, f2, format: :context, context_size: 1)
    assert_equal 1, code
    assert_includes output, "***************"
  end

  def test_ignore_case
    f1 = File.join(@tmpdir, "a.txt")
    f2 = File.join(@tmpdir, "b.txt")
    File.write(f1, "Hello\n")
    File.write(f2, "hello\n")

    output, code = diff_files(f1, f2, ignore_case: true)
    assert_equal 0, code
  end

  def test_ignore_all_space
    f1 = File.join(@tmpdir, "a.txt")
    f2 = File.join(@tmpdir, "b.txt")
    File.write(f1, "hello world\n")
    File.write(f2, "helloworld\n")

    output, code = diff_files(f1, f2, ignore_all_space: true)
    assert_equal 0, code
  end

  def test_ignore_space_change
    f1 = File.join(@tmpdir, "a.txt")
    f2 = File.join(@tmpdir, "b.txt")
    File.write(f1, "hello   world\n")
    File.write(f2, "hello world\n")

    output, code = diff_files(f1, f2, ignore_space_change: true)
    assert_equal 0, code
  end

  def test_ignore_blank_lines
    f1 = File.join(@tmpdir, "a.txt")
    f2 = File.join(@tmpdir, "b.txt")
    File.write(f1, "hello\n\nworld\n")
    File.write(f2, "hello\nworld\n")

    output, code = diff_files(f1, f2, ignore_blank_lines: true)
    assert_equal 0, code
  end

  def test_nonexistent_file
    f1 = File.join(@tmpdir, "nonexistent.txt")
    f2 = File.join(@tmpdir, "b.txt")
    File.write(f2, "hello\n")

    output, code = diff_files(f1, f2)
    assert_equal 2, code
    assert_includes output, "No such file"
  end
end

# ===========================================================================
# Test: diff_directories
# ===========================================================================

class TestDiffDirectories < Minitest::Test
  def setup
    @tmpdir = Dir.mktmpdir("diff_dir_test")
    @dir_a = File.join(@tmpdir, "a")
    @dir_b = File.join(@tmpdir, "b")
    FileUtils.mkdir_p(@dir_a)
    FileUtils.mkdir_p(@dir_b)
  end

  def teardown
    FileUtils.rm_rf(@tmpdir)
  end

  def test_identical_directories
    File.write(File.join(@dir_a, "file.txt"), "hello\n")
    File.write(File.join(@dir_b, "file.txt"), "hello\n")

    output, code = diff_directories(@dir_a, @dir_b)
    assert_equal 0, code
    assert_equal "", output
  end

  def test_file_only_in_first_dir
    File.write(File.join(@dir_a, "only_a.txt"), "hello\n")

    output, code = diff_directories(@dir_a, @dir_b)
    assert_equal 1, code
    assert_includes output, "Only in #{@dir_a}: only_a.txt"
  end

  def test_file_only_in_second_dir
    File.write(File.join(@dir_b, "only_b.txt"), "hello\n")

    output, code = diff_directories(@dir_a, @dir_b)
    assert_equal 1, code
    assert_includes output, "Only in #{@dir_b}: only_b.txt"
  end

  def test_different_file_content
    File.write(File.join(@dir_a, "file.txt"), "hello\n")
    File.write(File.join(@dir_b, "file.txt"), "world\n")

    output, code = diff_directories(@dir_a, @dir_b)
    assert_equal 1, code
    refute_empty output
  end

  def test_recursive_subdirectories
    FileUtils.mkdir_p(File.join(@dir_a, "sub"))
    FileUtils.mkdir_p(File.join(@dir_b, "sub"))
    File.write(File.join(@dir_a, "sub", "file.txt"), "hello\n")
    File.write(File.join(@dir_b, "sub", "file.txt"), "world\n")

    output, code = diff_directories(@dir_a, @dir_b)
    assert_equal 1, code
  end

  def test_exclude_pattern
    File.write(File.join(@dir_a, "file.txt"), "hello\n")
    File.write(File.join(@dir_a, "file.bak"), "backup\n")
    File.write(File.join(@dir_b, "file.txt"), "hello\n")

    output, code = diff_directories(@dir_a, @dir_b, exclude: ["*.bak"])
    assert_equal 0, code
  end
end

# ===========================================================================
# Test: Helper methods
# ===========================================================================

class TestDiffHelpers < Minitest::Test
  def test_format_line_range_single
    assert_equal "5", format_line_range([5])
  end

  def test_format_line_range_multiple
    assert_equal "3,7", format_line_range([3, 4, 5, 6, 7])
  end
end
