defmodule UniqTest do
  @moduledoc """
  Tests for the uniq tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Flag parsing (-c, -d, -u, -i, -f, -s, -w).
  3. Business logic (get_comparison_key, group_lines).
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "uniq.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the uniq spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "no arguments returns ParseResult" do
      assert {:ok, %ParseResult{}} = parse_argv(["uniq"])
    end

    test "input file is captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["uniq", "input.txt"])
      assert arguments["input"] == "input.txt"
    end

    test "input and output files are captured" do
      {:ok, %ParseResult{arguments: arguments}} =
        parse_argv(["uniq", "input.txt", "output.txt"])

      assert arguments["input"] == "input.txt"
      assert arguments["output"] == "output.txt"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Flags
  # ---------------------------------------------------------------------------

  describe "flags" do
    test "-c sets count to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["uniq", "-c"])
      assert flags["count"] == true
    end

    test "-d sets repeated to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["uniq", "-d"])
      assert flags["repeated"] == true
    end

    test "-u sets unique to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["uniq", "-u"])
      assert flags["unique"] == true
    end

    test "-i sets ignore_case to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["uniq", "-i"])
      assert flags["ignore_case"] == true
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help and --version
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["uniq", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["uniq", "--help"])
      assert text =~ "uniq"
    end
  end

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["uniq", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["uniq", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["uniq", "--unknown"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - get_comparison_key
  # ---------------------------------------------------------------------------

  describe "get_comparison_key/5" do
    test "returns full line with no options" do
      assert UnixTools.Uniq.get_comparison_key("hello", 0, 0, nil, false) == "hello"
    end

    test "case insensitive comparison" do
      assert UnixTools.Uniq.get_comparison_key("Hello", 0, 0, nil, true) == "hello"
    end

    test "skip characters" do
      assert UnixTools.Uniq.get_comparison_key("abcdef", 0, 3, nil, false) == "def"
    end

    test "check limited characters" do
      assert UnixTools.Uniq.get_comparison_key("abcdef", 0, 0, 3, false) == "abc"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - group_lines
  # ---------------------------------------------------------------------------

  describe "group_lines/5" do
    test "groups adjacent identical lines" do
      lines = ["apple", "apple", "banana", "cherry", "cherry"]
      groups = UnixTools.Uniq.group_lines(lines, 0, 0, nil, false)

      assert groups == [
               {"apple", 2},
               {"banana", 1},
               {"cherry", 2}
             ]
    end

    test "single lines are groups of 1" do
      lines = ["a", "b", "c"]
      groups = UnixTools.Uniq.group_lines(lines, 0, 0, nil, false)
      assert groups == [{"a", 1}, {"b", 1}, {"c", 1}]
    end

    test "all same lines form one group" do
      lines = ["x", "x", "x"]
      groups = UnixTools.Uniq.group_lines(lines, 0, 0, nil, false)
      assert groups == [{"x", 3}]
    end

    test "case-insensitive grouping" do
      lines = ["Hello", "hello", "HELLO"]
      groups = UnixTools.Uniq.group_lines(lines, 0, 0, nil, true)
      assert groups == [{"Hello", 3}]
    end
  end
end
