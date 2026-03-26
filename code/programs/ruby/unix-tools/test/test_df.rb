# frozen_string_literal: true

# test_df.rb -- Tests for the Ruby df tool
# ==========================================
#
# === What These Tests Verify ===
#
# These tests exercise the df tool's filesystem info parsing and
# formatting. Since df shells out to the system `df` command, we
# test:
# - Output parsing from a known format
# - Human-readable size formatting
# - Total calculation
# - CLI Builder integration

require "simplecov"
SimpleCov.start do
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "coding_adventures_cli_builder"

require_relative "../df_tool"

# ---------------------------------------------------------------------------
# Helper module
# ---------------------------------------------------------------------------

module DfTestHelper
  DF_TEST_SPEC = File.join(File.dirname(__FILE__), "..", "df.json")

  def parse_df_argv(argv)
    CodingAdventures::CliBuilder::Parser.new(DF_TEST_SPEC, ["df"] + argv).parse
  end
end

# ===========================================================================
# Test: CLI Builder integration
# ===========================================================================

class TestDfCliIntegration < Minitest::Test
  include DfTestHelper

  def test_no_flags_returns_parse_result
    result = parse_df_argv([])
    assert_kind_of CodingAdventures::CliBuilder::ParseResult, result
  end

  def test_help_returns_help_result
    result = parse_df_argv(["--help"])
    assert_kind_of CodingAdventures::CliBuilder::HelpResult, result
    assert_includes result.text, "df"
  end

  def test_version_returns_version_result
    result = parse_df_argv(["--version"])
    assert_kind_of CodingAdventures::CliBuilder::VersionResult, result
    assert_equal "1.0.0", result.version
  end

  def test_human_readable_flag
    result = parse_df_argv(["-h"])
    assert result.flags["human_readable"]
  end

  def test_total_flag
    result = parse_df_argv(["--total"])
    assert result.flags["total"]
  end
end

# ===========================================================================
# Test: parse_df_output
# ===========================================================================

class TestParseDfOutput < Minitest::Test
  def test_parses_posix_output
    output = <<~DF
      Filesystem     1024-blocks      Used Available Capacity Mounted on
      /dev/disk1       976490576 789012345 187478231      81% /
    DF
    entries = parse_df_output(output, {})
    assert_equal 1, entries.length
    assert_equal "/dev/disk1", entries[0].filesystem
    assert_equal 976490576, entries[0].blocks
    assert_equal 789012345, entries[0].used
    assert_equal 187478231, entries[0].available
    assert_equal "81%", entries[0].use_percent
    assert_equal "/", entries[0].mounted_on
  end

  def test_parses_multiple_entries
    output = <<~DF
      Filesystem     1024-blocks  Used Available Capacity Mounted on
      /dev/disk1       976490576 789012345 187478231      81% /
      tmpfs              1048576   512     1048064       1% /tmp
    DF
    entries = parse_df_output(output, {})
    assert_equal 2, entries.length
    assert_equal "tmpfs", entries[1].filesystem
    assert_equal "/tmp", entries[1].mounted_on
  end

  def test_empty_output
    entries = parse_df_output("", {})
    assert_equal [], entries
  end

  def test_header_only
    entries = parse_df_output("Filesystem  1024-blocks Used Available Capacity Mounted\n", {})
    assert_equal [], entries
  end
end

# ===========================================================================
# Test: format_size
# ===========================================================================

class TestFormatSize < Minitest::Test
  def test_plain_number
    assert_equal "1024", format_size(1024, false, false)
  end

  def test_human_readable_kilobytes
    result = format_size(512, true, false)
    assert_includes result, "K"
  end

  def test_human_readable_megabytes
    result = format_size(1048576, true, false)
    assert_includes result, "G"
  end

  def test_human_readable_small
    result = format_size(5, true, false)
    assert_includes result, "K"
  end

  def test_si_mode
    result = format_size(1000, false, true)
    assert_includes result, "K" if result.include?("K")
  end

  def test_zero
    assert_equal "0", format_size(0, false, false)
  end
end

# ===========================================================================
# Test: format_df_output
# ===========================================================================

class TestFormatDfOutput < Minitest::Test
  def test_includes_header
    entries = []
    lines = format_df_output(entries, {})
    assert_equal 1, lines.length
    assert_includes lines[0], "Filesystem"
  end

  def test_includes_entry
    entry = FilesystemInfo.new(
      filesystem: "/dev/sda1",
      fstype: nil,
      blocks: 1000000,
      used: 500000,
      available: 500000,
      use_percent: "50%",
      mounted_on: "/"
    )
    lines = format_df_output([entry], {})
    assert_equal 2, lines.length
    assert_includes lines[1], "/dev/sda1"
    assert_includes lines[1], "50%"
  end

  def test_total_row
    entry = FilesystemInfo.new(
      filesystem: "/dev/sda1",
      fstype: nil,
      blocks: 1000000,
      used: 500000,
      available: 500000,
      use_percent: "50%",
      mounted_on: "/"
    )
    lines = format_df_output([entry], {"total" => true})
    assert_equal 3, lines.length
    assert_includes lines[2], "total"
  end

  def test_no_total_without_flag
    entry = FilesystemInfo.new(
      filesystem: "/dev/sda1",
      fstype: nil,
      blocks: 1000000,
      used: 500000,
      available: 500000,
      use_percent: "50%",
      mounted_on: "/"
    )
    lines = format_df_output([entry], {})
    assert_equal 2, lines.length
  end
end

# ===========================================================================
# Test: FilesystemInfo struct
# ===========================================================================

class TestFilesystemInfoStruct < Minitest::Test
  def test_create_with_keywords
    info = FilesystemInfo.new(
      filesystem: "/dev/sda1",
      fstype: "ext4",
      blocks: 100,
      used: 50,
      available: 50,
      use_percent: "50%",
      mounted_on: "/"
    )
    assert_equal "/dev/sda1", info.filesystem
    assert_equal "ext4", info.fstype
    assert_equal 100, info.blocks
  end
end

# ===========================================================================
# Test: run_df_command
# ===========================================================================

class TestRunDfCommand < Minitest::Test
  def test_returns_string
    result = run_df_command(["df", "-Pk"])
    assert_kind_of String, result
    refute_empty result
  end

  def test_nonexistent_command
    result = run_df_command(["/nonexistent/command"])
    assert_equal "", result
  end
end

# ===========================================================================
# Test: Main function integration
# ===========================================================================

class TestDfMainFunction < Minitest::Test
  include DfTestHelper

  def test_main_default_output
    old_argv = ARGV.dup
    ARGV.replace([])
    output = capture_io { df_main }[0]
    assert_includes output, "Filesystem"
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_help_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--help"])
    err = assert_raises(SystemExit) { capture_io { df_main } }
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end

  def test_main_version_exits_zero
    old_argv = ARGV.dup
    ARGV.replace(["--version"])
    err = assert_raises(SystemExit) { capture_io { df_main } }
    assert_equal 0, err.status
  ensure
    ARGV.replace(old_argv)
  end
end
