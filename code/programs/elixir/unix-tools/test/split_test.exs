defmodule SplitTest do
  @moduledoc """
  Tests for the split tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Flag parsing (-l, -b, -a, -d, -x, --additional-suffix, --verbose).
  3. Business logic: generate_suffix (alphabetic, numeric, hex).
  4. Business logic: base_convert (the underlying base conversion).
  5. Business logic: split_by_lines.
  6. Business logic: split_by_bytes.
  7. Business logic: parse_size (size string parsing).
  8. Edge cases: empty content, single chunk, exact boundaries.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "split.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the split spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "no arguments returns ParseResult with defaults" do
      assert {:ok, %ParseResult{}} = parse_argv(["split"])
    end

    test "file argument is captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["split", "data.txt"])
      assert arguments["file"] == "data.txt"
    end

    test "file and prefix arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["split", "data.txt", "out"])
      assert arguments["file"] == "data.txt"
      assert arguments["prefix"] == "out"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Flags
  # ---------------------------------------------------------------------------

  describe "flags" do
    test "-l sets lines" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["split", "-l", "100"])
      assert flags["lines"] == 100
    end

    test "-b sets bytes" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["split", "-b", "1024"])
      assert flags["bytes"] == "1024"
    end

    test "-a sets suffix_length" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["split", "-a", "4"])
      assert flags["suffix_length"] == 4
    end

    test "-d sets numeric_suffixes" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["split", "-d"])
      assert flags["numeric_suffixes"] == true
    end

    test "-x sets hex_suffixes" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["split", "-x"])
      assert flags["hex_suffixes"] == true
    end

    test "--verbose sets verbose" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["split", "--verbose"])
      assert flags["verbose"] == true
    end

    test "--additional-suffix sets additional_suffix" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["split", "--additional-suffix", ".txt"])
      assert flags["additional_suffix"] == ".txt"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help and --version
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["split", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["split", "--help"])
      assert text =~ "split"
    end
  end

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["split", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["split", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["split", "--unknown"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: generate_suffix/3 - alphabetic
  # ---------------------------------------------------------------------------

  describe "generate_suffix/3 alphabetic" do
    test "first suffix is aa" do
      assert UnixTools.Split.generate_suffix(0, :alpha, 2) == "aa"
    end

    test "second suffix is ab" do
      assert UnixTools.Split.generate_suffix(1, :alpha, 2) == "ab"
    end

    test "26th suffix is az" do
      assert UnixTools.Split.generate_suffix(25, :alpha, 2) == "az"
    end

    test "27th suffix is ba" do
      assert UnixTools.Split.generate_suffix(26, :alpha, 2) == "ba"
    end

    test "28th suffix is bb" do
      assert UnixTools.Split.generate_suffix(27, :alpha, 2) == "bb"
    end

    test "last 2-char suffix is zz" do
      assert UnixTools.Split.generate_suffix(675, :alpha, 2) == "zz"
    end

    test "3-character suffix" do
      assert UnixTools.Split.generate_suffix(0, :alpha, 3) == "aaa"
      assert UnixTools.Split.generate_suffix(1, :alpha, 3) == "aab"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: generate_suffix/3 - numeric
  # ---------------------------------------------------------------------------

  describe "generate_suffix/3 numeric" do
    test "zero-padded numeric" do
      assert UnixTools.Split.generate_suffix(0, :numeric, 2) == "00"
      assert UnixTools.Split.generate_suffix(5, :numeric, 2) == "05"
      assert UnixTools.Split.generate_suffix(42, :numeric, 2) == "42"
    end

    test "3-digit numeric" do
      assert UnixTools.Split.generate_suffix(5, :numeric, 3) == "005"
      assert UnixTools.Split.generate_suffix(123, :numeric, 3) == "123"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: generate_suffix/3 - hex
  # ---------------------------------------------------------------------------

  describe "generate_suffix/3 hex" do
    test "zero-padded hex" do
      assert UnixTools.Split.generate_suffix(0, :hex, 2) == "00"
      assert UnixTools.Split.generate_suffix(15, :hex, 2) == "0f"
      assert UnixTools.Split.generate_suffix(255, :hex, 2) == "ff"
    end

    test "3-digit hex" do
      assert UnixTools.Split.generate_suffix(16, :hex, 3) == "010"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: base_convert/3
  # ---------------------------------------------------------------------------

  describe "base_convert/3" do
    test "zero converts to all zeros" do
      assert UnixTools.Split.base_convert(0, 26, 2) == [0, 0]
    end

    test "converts to base 26" do
      assert UnixTools.Split.base_convert(1, 26, 2) == [0, 1]
      assert UnixTools.Split.base_convert(26, 26, 2) == [1, 0]
      assert UnixTools.Split.base_convert(27, 26, 2) == [1, 1]
    end

    test "pads with leading zeros" do
      assert UnixTools.Split.base_convert(1, 10, 4) == [0, 0, 0, 1]
    end

    test "converts to base 10" do
      assert UnixTools.Split.base_convert(42, 10, 2) == [4, 2]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: split_by_lines/2
  # ---------------------------------------------------------------------------

  describe "split_by_lines/2" do
    test "splits into chunks of N lines" do
      content = "a\nb\nc\nd\ne\n"
      result = UnixTools.Split.split_by_lines(content, 2)

      assert length(result) == 3
      assert Enum.at(result, 0) =~ "a"
      assert Enum.at(result, 0) =~ "b"
    end

    test "single chunk when fewer lines than limit" do
      content = "one\ntwo\nthree\n"
      result = UnixTools.Split.split_by_lines(content, 5)

      assert length(result) == 1
    end

    test "handles content without trailing newline" do
      content = "a\nb\nc"
      result = UnixTools.Split.split_by_lines(content, 2)

      assert length(result) == 2
    end

    test "empty content" do
      result = UnixTools.Split.split_by_lines("", 10)
      assert result == []
    end

    test "one line per chunk" do
      content = "a\nb\nc\n"
      result = UnixTools.Split.split_by_lines(content, 1)

      # Each chunk should have exactly one line (plus potentially a newline join)
      assert length(result) >= 3
    end
  end

  # ---------------------------------------------------------------------------
  # Test: split_by_bytes/2
  # ---------------------------------------------------------------------------

  describe "split_by_bytes/2" do
    test "splits into fixed-size byte chunks" do
      result = UnixTools.Split.split_by_bytes("abcdefghij", 3)
      assert result == ["abc", "def", "ghi", "j"]
    end

    test "single chunk when smaller than limit" do
      result = UnixTools.Split.split_by_bytes("hello", 10)
      assert result == ["hello"]
    end

    test "exact multiple" do
      result = UnixTools.Split.split_by_bytes("abcdef", 3)
      assert result == ["abc", "def"]
    end

    test "single byte chunks" do
      result = UnixTools.Split.split_by_bytes("abc", 1)
      assert result == ["a", "b", "c"]
    end

    test "empty content" do
      result = UnixTools.Split.split_by_bytes("", 5)
      assert result == []
    end
  end

  # ---------------------------------------------------------------------------
  # Test: parse_size/1
  # ---------------------------------------------------------------------------

  describe "parse_size/1" do
    test "plain number" do
      assert UnixTools.Split.parse_size("100") == 100
    end

    test "kilobytes" do
      assert UnixTools.Split.parse_size("1K") == 1024
      assert UnixTools.Split.parse_size("2k") == 2048
    end

    test "megabytes" do
      assert UnixTools.Split.parse_size("1M") == 1_048_576
      assert UnixTools.Split.parse_size("2M") == 2_097_152
    end

    test "gigabytes" do
      assert UnixTools.Split.parse_size("1G") == 1_073_741_824
    end

    test "integer input passes through" do
      assert UnixTools.Split.parse_size(500) == 500
    end

    test "handles whitespace" do
      assert UnixTools.Split.parse_size("  1K  ") == 1024
    end
  end
end
