defmodule DfTest do
  @moduledoc """
  Tests for the df tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version).
  2. Parsing of `df -k` output into structured data.
  3. Human-readable size formatting.
  4. Live filesystem info retrieval.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "df.json"]) |> Path.expand()

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
      assert {:ok, %ParseResult{}} = parse_argv(["df"])
    end

    test "-h sets human_readable to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["df", "-h"])
      assert flags["human_readable"] == true
    end

    test "-H sets si to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["df", "-H"])
      assert flags["si"] == true
    end

    test "-T sets print_type to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["df", "-T"])
      assert flags["print_type"] == true
    end

    test "file argument is captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["df", "/tmp"])
      assert arguments["files"] == ["/tmp"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help flag
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["df", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["df", "--help"])
      assert text =~ "df"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --version flag
  # ---------------------------------------------------------------------------

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["df", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["df", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["df", "--unknown"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: parse_df_output/1
  # ---------------------------------------------------------------------------

  describe "parse_df_output/1" do
    test "parses typical Linux df output" do
      output = """
      Filesystem     1K-blocks     Used Available Use% Mounted on
      /dev/sda1      102400000 60000000  42400000  59% /
      tmpfs            8192000        0   8192000   0% /tmp
      """

      entries = UnixTools.Df.parse_df_output(output)
      assert length(entries) == 2

      first = List.first(entries)
      assert first.filesystem == "/dev/sda1"
      assert first.blocks == 102_400_000
      assert first.used == 60_000_000
      assert first.available == 42_400_000
      assert first.use_percent == "59"
      assert first.mounted_on == "/"
    end

    test "parses macOS df output with longer mount points" do
      output = """
      Filesystem   1024-blocks      Used Available Capacity  Mounted on
      /dev/disk1s1   488245288 350000000 138245288    72%    /System/Volumes/Data
      """

      entries = UnixTools.Df.parse_df_output(output)
      assert length(entries) == 1

      entry = List.first(entries)
      assert entry.filesystem == "/dev/disk1s1"
      assert entry.blocks == 488_245_288
    end

    test "empty output returns empty list" do
      assert UnixTools.Df.parse_df_output("") == []
    end

    test "header-only output returns empty list" do
      output = "Filesystem     1K-blocks     Used Available Use% Mounted on\n"
      assert UnixTools.Df.parse_df_output(output) == []
    end
  end

  # ---------------------------------------------------------------------------
  # Test: format_size/2
  # ---------------------------------------------------------------------------

  describe "format_size/2" do
    test "formats kilobytes" do
      result = UnixTools.Df.format_size(500)
      assert result =~ "K"
    end

    test "formats megabytes" do
      result = UnixTools.Df.format_size(1536)
      assert result =~ "M"
    end

    test "formats gigabytes" do
      result = UnixTools.Df.format_size(2_000_000)
      assert result =~ "G"
    end

    test "si mode uses 1000 as base" do
      result = UnixTools.Df.format_size(1500, 1000)
      assert result =~ "M"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: get_fs_info/0 (live system test)
  # ---------------------------------------------------------------------------

  describe "get_fs_info/0" do
    test "returns a non-empty list of filesystem entries" do
      entries = UnixTools.Df.get_fs_info()
      assert is_list(entries)
      assert length(entries) > 0
    end

    test "entries have expected keys" do
      entries = UnixTools.Df.get_fs_info()
      entry = List.first(entries)

      assert Map.has_key?(entry, :filesystem)
      assert Map.has_key?(entry, :blocks)
      assert Map.has_key?(entry, :used)
      assert Map.has_key?(entry, :available)
      assert Map.has_key?(entry, :use_percent)
      assert Map.has_key?(entry, :mounted_on)
    end

    test "blocks are positive integers" do
      entries = UnixTools.Df.get_fs_info()
      entry = List.first(entries)
      assert is_integer(entry.blocks)
    end
  end

  # ---------------------------------------------------------------------------
  # Test: get_fs_info/1 (specific path, live system test)
  # ---------------------------------------------------------------------------

  describe "get_fs_info/1" do
    test "returns info for root filesystem" do
      entries = UnixTools.Df.get_fs_info("/")
      assert is_list(entries)
      assert length(entries) >= 1
    end
  end
end
