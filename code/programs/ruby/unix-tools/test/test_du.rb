# frozen_string_literal: true

# test_du.rb -- Tests for the Ruby du tool
# ==========================================
#
# === What These Tests Verify ===
#
# These tests exercise the du tool's disk usage calculation and
# formatting. We test:
# - format_du_size in default and human-readable modes
# - disk_usage with real temporary directories
# - matches_exclude? glob matching
# - Summarize mode (-s)
# - Max depth limiting (-d)

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "tmpdir"
require "fileutils"
require "coding_adventures_cli_builder"

require_relative "../du_tool"

# ---------------------------------------------------------------------------
# Helper module
# ---------------------------------------------------------------------------

module DuTestHelper
  DU_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "du.json")

  def parse_du_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(DU_TEST_SPEC, ["du"] + argv).parse
  end

  # Create a temporary directory structure for testing.
  # Returns the path to the root directory.
  def create_test_tree
    dir = Dir.mktmpdir("du_test")

    # Create files of known sizes
    File.write(File.join(dir, "file1.txt"), "a" * 1000)
    File.write(File.join(dir, "file2.txt"), "b" * 2000)

    # Create a subdirectory with files
    subdir = File.join(dir, "subdir")
    FileUtils.mkdir_p(subdir)
    File.write(File.join(subdir, "file3.txt"), "c" * 500)

    dir
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestDuCliIntegration < Minitest::Test
  include DuTestHelper

  def test_no_flags_returns_parse_result
    result = parse_du_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_help_returns_help_result
    result = parse_du_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
    assert_includes result.text, "du"
  end

  def test_version_returns_version_result
    result = parse_du_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_summarize_flag
    result = parse_du_argv(["-s"])
    assert result.flags["summarize"]
  end

  def test_all_flag
    result = parse_du_argv(["-a"])
    assert result.flags["all"]
  end

  def test_human_readable_flag
    result = parse_du_argv(["-h"])
    assert result.flags["human_readable"]
  end

  def test_max_depth_flag
    result = parse_du_argv(["-d", "2"])
    assert_equal 2, result.flags["max_depth"]
  end
end

# ===========================================================================
# Test: format_du_size
# ===========================================================================

class TestFormatDuSize < Minitest::Test
  def test_default_1k_blocks
    # 1024 bytes = 1 block
    assert_equal "1", format_du_size(1024, false, false)
  end

  def test_default_rounds_up
    # 1025 bytes = 2 blocks (rounds up)
    assert_equal "2", format_du_size(1025, false, false)
  end

  def test_zero_bytes
    assert_equal "0", format_du_size(0, false, false)
  end

  def test_human_readable_bytes
    result = format_du_size(500, true, false)
    # 500 bytes should show as a small number
    refute_empty result
  end

  def test_human_readable_kilobytes
    result = format_du_size(2048, true, false)
    assert_includes result, "K"
  end

  def test_human_readable_megabytes
    result = format_du_size(2 * 1024 * 1024, true, false)
    assert_includes result, "M"
  end

  def test_human_readable_gigabytes
    result = format_du_size(2 * 1024 * 1024 * 1024, true, false)
    assert_includes result, "G"
  end

  def test_si_mode
    result = format_du_size(2000, false, true)
    assert_includes result, "K"
  end
end

# ===========================================================================
# Test: matches_exclude?
# ===========================================================================

class TestMatchesExclude < Minitest::Test
  def test_no_patterns
    refute matches_exclude?("/tmp/file.txt", nil)
    refute matches_exclude?("/tmp/file.txt", [])
  end

  def test_matching_basename
    assert matches_exclude?("/tmp/file.txt", ["*.txt"])
  end

  def test_non_matching_basename
    refute matches_exclude?("/tmp/file.rb", ["*.txt"])
  end

  def test_multiple_patterns
    assert matches_exclude?("/tmp/file.txt", ["*.rb", "*.txt"])
    refute matches_exclude?("/tmp/file.py", ["*.rb", "*.txt"])
  end

  def test_exact_name_match
    assert matches_exclude?("/tmp/node_modules", ["node_modules"])
  end
end

# ===========================================================================
# Test: disk_usage with real files
# ===========================================================================

class TestDiskUsage < Minitest::Test
  include DuTestHelper

  def test_reports_directory_size
    dir = create_test_tree
    results = disk_usage(dir, {})
    refute_empty results

    # The root directory should appear
    root_entry = results.find { |_, path| path == dir }
    refute_nil root_entry

    # Size should be > 0
    assert root_entry[0] > 0
  ensure
    FileUtils.remove_entry(dir)
  end

  def test_includes_subdirectories
    dir = create_test_tree
    results = disk_usage(dir, {})

    subdir_entry = results.find { |_, path| path.include?("subdir") }
    refute_nil subdir_entry
  ensure
    FileUtils.remove_entry(dir)
  end

  def test_summarize_mode
    dir = create_test_tree
    results = disk_usage(dir, {"summarize" => true})

    # Should only have one entry (the root)
    assert_equal 1, results.length
    assert_equal dir, results[0][1]
  ensure
    FileUtils.remove_entry(dir)
  end

  def test_all_flag_includes_files
    dir = create_test_tree
    results = disk_usage(dir, {"all" => true})

    file_entries = results.select { |_, path| path.include?("file1.txt") }
    refute_empty file_entries
  ensure
    FileUtils.remove_entry(dir)
  end

  def test_max_depth_limits_output
    dir = create_test_tree
    results = disk_usage(dir, {"max_depth" => 0})

    # Only the root should appear
    assert_equal 1, results.length
  ensure
    FileUtils.remove_entry(dir)
  end

  def test_exclude_pattern
    dir = create_test_tree
    results = disk_usage(dir, {"exclude" => ["subdir"]})

    subdir_entry = results.find { |_, path| path.include?("subdir") }
    assert_nil subdir_entry
  ensure
    FileUtils.remove_entry(dir)
  end

  def test_nonexistent_path
    results = disk_usage("/nonexistent/path/xyz", {})
    assert_equal [], results
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestDuMainFunction < Minitest::Test
  include DuTestHelper

  def test_main_runs_on_temp_dir
    dir = create_test_tree
    old_argv = ARGV.dup
    ARGV.replace([dir])
    output = capture_io { du_main }[0]
    refute_empty output
    assert_includes output, dir
  ensure
    ARGV.replace(old_argv)
    FileUtils.remove_entry(dir)
  end

  def test_main_summarize
    dir = create_test_tree
    old_argv = ARGV.dup
    ARGV.replace(["-s", dir])
    output = capture_io { du_main }[0]
    lines = output.strip.split("\n")
    assert_equal 1, lines.length
  ensure
    ARGV.replace(old_argv)
    FileUtils.remove_entry(dir)
  end

  def test_main_total_flag
    dir = create_test_tree
    old_argv = ARGV.dup
    ARGV.replace(["-c", "-s", dir])
    output = capture_io { du_main }[0]
    assert_includes output, "total"
  ensure
    ARGV.replace(old_argv)
    FileUtils.remove_entry(dir)
  end

  def test_main_help_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) { capture_io { du_main } }
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) { capture_io { du_main } }
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end
end
