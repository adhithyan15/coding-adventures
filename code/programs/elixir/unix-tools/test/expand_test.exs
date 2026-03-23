defmodule ExpandTest do
  @moduledoc """
  Tests for the expand tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Flag parsing (-i for initial, -t for tab stops).
  3. Business logic (expand_line, parse_tab_stops, spaces_to_next_tab).
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "expand.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the expand spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "no arguments returns ParseResult" do
      assert {:ok, %ParseResult{}} = parse_argv(["expand"])
    end

    test "file arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["expand", "file.txt"])
      assert arguments["files"] == ["file.txt"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Flags
  # ---------------------------------------------------------------------------

  describe "flags" do
    test "-i sets initial to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["expand", "-i", "file.txt"])
      assert flags["initial"] == true
    end

    test "-t sets tab stops" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["expand", "-t", "4", "file.txt"])
      assert flags["tabs"] == "4"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help and --version
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["expand", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["expand", "--help"])
      assert text =~ "expand"
    end
  end

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["expand", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["expand", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["expand", "--unknown"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - parse_tab_stops
  # ---------------------------------------------------------------------------

  describe "parse_tab_stops/1" do
    test "parses single number" do
      assert UnixTools.ExpandTool.parse_tab_stops("4") == 4
    end

    test "parses comma-separated list" do
      assert UnixTools.ExpandTool.parse_tab_stops("2,6,10") == [2, 6, 10]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - spaces_to_next_tab
  # ---------------------------------------------------------------------------

  describe "spaces_to_next_tab/2" do
    test "uniform tab stops: column 0" do
      assert UnixTools.ExpandTool.spaces_to_next_tab(0, 8) == 8
    end

    test "uniform tab stops: column 3" do
      assert UnixTools.ExpandTool.spaces_to_next_tab(3, 8) == 5
    end

    test "uniform tab stops: column 7" do
      assert UnixTools.ExpandTool.spaces_to_next_tab(7, 8) == 1
    end

    test "variable tab stops" do
      assert UnixTools.ExpandTool.spaces_to_next_tab(0, [4, 8, 12]) == 4
      assert UnixTools.ExpandTool.spaces_to_next_tab(5, [4, 8, 12]) == 3
    end

    test "past all variable stops" do
      assert UnixTools.ExpandTool.spaces_to_next_tab(15, [4, 8, 12]) == 1
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - expand_line
  # ---------------------------------------------------------------------------

  describe "expand_line/3" do
    test "expands tab at start of line" do
      result = UnixTools.ExpandTool.expand_line("\thello", 8, false)
      assert result == "        hello"
    end

    test "expands tab with custom tab size" do
      result = UnixTools.ExpandTool.expand_line("\thello", 4, false)
      assert result == "    hello"
    end

    test "handles line with no tabs" do
      result = UnixTools.ExpandTool.expand_line("hello", 8, false)
      assert result == "hello"
    end

    test "initial-only mode preserves tabs after text" do
      result = UnixTools.ExpandTool.expand_line("\thello\tworld", 8, true)
      assert result == "        hello\tworld"
    end
  end
end
