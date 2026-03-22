defmodule FoldTest do
  @moduledoc """
  Tests for the fold tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Flag parsing (-b for bytes, -s for spaces, -w for width).
  3. Business logic (fold_line).
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "fold.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the fold spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "no arguments returns ParseResult" do
      assert {:ok, %ParseResult{}} = parse_argv(["fold"])
    end

    test "file arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["fold", "file.txt"])
      assert arguments["files"] == ["file.txt"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Flags
  # ---------------------------------------------------------------------------

  describe "flags" do
    test "-b sets bytes to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["fold", "-b", "file.txt"])
      assert flags["bytes"] == true
    end

    test "-s sets spaces to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["fold", "-s", "file.txt"])
      assert flags["spaces"] == true
    end

    test "-w sets width" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["fold", "-w", "40", "file.txt"])
      assert flags["width"] == 40
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help and --version
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["fold", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["fold", "--help"])
      assert text =~ "fold"
    end
  end

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["fold", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["fold", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["fold", "--unknown"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - fold_line
  # ---------------------------------------------------------------------------

  describe "fold_line/4" do
    test "short line is unchanged" do
      result = UnixTools.Fold.fold_line("hello", 80, false, false)
      assert result == "hello"
    end

    test "long line is folded at width" do
      input = String.duplicate("a", 20)
      result = UnixTools.Fold.fold_line(input, 10, false, false)
      assert result == "aaaaaaaaaa\naaaaaaaaaa"
    end

    test "empty line is unchanged" do
      result = UnixTools.Fold.fold_line("", 80, false, false)
      assert result == ""
    end
  end
end
