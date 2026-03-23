defmodule DuTest do
  @moduledoc """
  Tests for the du tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version).
  2. Bytes-to-blocks conversion.
  3. Human-readable formatting.
  4. Disk usage measurement on actual files/directories.
  5. Max depth limiting.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "du.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: CLI parsing
  # ---------------------------------------------------------------------------

  describe "CLI parsing" do
    test "no arguments returns ParseResult" do
      assert {:ok, %ParseResult{}} = parse_argv(["du"])
    end

    test "-a sets all to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["du", "-a"])
      assert flags["all"] == true
    end

    test "-h sets human_readable to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["du", "-h"])
      assert flags["human_readable"] == true
    end

    test "-s sets summarize to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["du", "-s"])
      assert flags["summarize"] == true
    end

    test "-c sets total to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["du", "-c"])
      assert flags["total"] == true
    end

    test "file arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["du", "/tmp"])
      assert arguments["files"] == ["/tmp"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help flag
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["du", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["du", "--help"])
      assert text =~ "du"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --version flag
  # ---------------------------------------------------------------------------

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["du", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["du", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["du", "--unknown"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: bytes_to_blocks/1
  # ---------------------------------------------------------------------------

  describe "bytes_to_blocks/1" do
    test "0 bytes is 0 blocks" do
      assert UnixTools.Du.bytes_to_blocks(0) == 0
    end

    test "exactly 1024 bytes is 1 block" do
      assert UnixTools.Du.bytes_to_blocks(1024) == 1
    end

    test "1025 bytes is 2 blocks (ceiling)" do
      assert UnixTools.Du.bytes_to_blocks(1025) == 2
    end

    test "1 byte is 1 block" do
      assert UnixTools.Du.bytes_to_blocks(1) == 1
    end

    test "2048 bytes is 2 blocks" do
      assert UnixTools.Du.bytes_to_blocks(2048) == 2
    end

    test "negative bytes is 0 blocks" do
      assert UnixTools.Du.bytes_to_blocks(-100) == 0
    end
  end

  # ---------------------------------------------------------------------------
  # Test: format_human/2
  # ---------------------------------------------------------------------------

  describe "format_human/2" do
    test "small value stays in K" do
      result = UnixTools.Du.format_human(4)
      assert result =~ "K"
    end

    test "1024K becomes 1.0M" do
      result = UnixTools.Du.format_human(1024)
      assert result =~ "M"
    end

    test "large value becomes G" do
      result = UnixTools.Du.format_human(2_000_000)
      assert result =~ "G"
    end

    test "si mode uses 1000 as base" do
      result = UnixTools.Du.format_human(1500, 1000)
      assert result =~ "M"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: disk_usage/2 with temp files
  # ---------------------------------------------------------------------------

  describe "disk_usage/2" do
    setup do
      # Create a temporary directory structure for testing.
      tmp_dir = Path.join(System.tmp_dir!(), "du_test_#{:rand.uniform(100_000)}")
      File.mkdir_p!(tmp_dir)
      sub_dir = Path.join(tmp_dir, "subdir")
      File.mkdir_p!(sub_dir)

      # Create some test files with known content.
      File.write!(Path.join(tmp_dir, "file1.txt"), String.duplicate("a", 1000))
      File.write!(Path.join(sub_dir, "file2.txt"), String.duplicate("b", 2000))

      on_exit(fn ->
        File.rm_rf!(tmp_dir)
      end)

      %{tmp_dir: tmp_dir, sub_dir: sub_dir}
    end

    test "measures a single file", %{tmp_dir: tmp_dir} do
      file_path = Path.join(tmp_dir, "file1.txt")
      results = UnixTools.Du.disk_usage(file_path)
      assert length(results) == 1
      [{^file_path, size}] = results
      assert size == 1000
    end

    test "measures a directory recursively", %{tmp_dir: tmp_dir} do
      results = UnixTools.Du.disk_usage(tmp_dir)
      # Should have entries for subdir and tmp_dir
      assert length(results) >= 2

      # The last entry should be the root dir with total size
      {root_path, total_size} = List.last(results)
      assert root_path == tmp_dir
      assert total_size == 3000
    end

    test "show_all includes individual files", %{tmp_dir: tmp_dir} do
      results = UnixTools.Du.disk_usage(tmp_dir, %{show_all: true})
      # With show_all, we should see individual files
      paths = Enum.map(results, fn {path, _size} -> path end)
      assert Enum.any?(paths, &String.ends_with?(&1, "file1.txt"))
      assert Enum.any?(paths, &String.ends_with?(&1, "file2.txt"))
    end

    test "max_depth limits output", %{tmp_dir: tmp_dir} do
      results = UnixTools.Du.disk_usage(tmp_dir, %{max_depth: 0})
      # With max_depth 0, should only show the root directory
      assert length(results) == 1
      [{^tmp_dir, _size}] = results
    end

    test "handles nonexistent path" do
      # Use Path.join with System.tmp_dir! so the path format is correct on
      # both Unix and Windows (avoids hard-coding a Unix-style absolute path).
      nonexistent = Path.join(System.tmp_dir!(), "nonexistent_du_test_#{:rand.uniform(999_999)}")
      results = UnixTools.Du.disk_usage(nonexistent)
      assert results == []
    end
  end
end
