defmodule CommTest do
  @moduledoc """
  Tests for the comm tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version).
  2. Three-column comparison of sorted inputs.
  3. Column suppression (-1, -2, -3).
  4. Output formatting with correct tab prefixes.
  5. Edge cases (empty inputs, identical inputs, disjoint inputs).
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "comm.json"]) |> Path.expand()

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
    test "two file arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["comm", "f1", "f2"])
      assert arguments["file1"] == "f1"
      assert arguments["file2"] == "f2"
    end

    test "-1 sets suppress_col1" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["comm", "-1", "f1", "f2"])
      assert flags["suppress_col1"] == true
    end

    test "-2 sets suppress_col2" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["comm", "-2", "f1", "f2"])
      assert flags["suppress_col2"] == true
    end

    test "-3 sets suppress_col3" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["comm", "-3", "f1", "f2"])
      assert flags["suppress_col3"] == true
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help flag
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["comm", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["comm", "--help"])
      assert text =~ "comm"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --version flag
  # ---------------------------------------------------------------------------

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["comm", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["comm", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["comm", "--unknown", "f1", "f2"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: compare_sorted/3
  # ---------------------------------------------------------------------------

  describe "compare_sorted/3" do
    test "basic three-column comparison" do
      result = UnixTools.Comm.compare_sorted(
        ["apple", "banana", "cherry"],
        ["banana", "cherry", "date"]
      )

      assert result == [
        {1, "apple"},
        {3, "banana"},
        {3, "cherry"},
        {2, "date"}
      ]
    end

    test "identical inputs produce all column-3 entries" do
      result = UnixTools.Comm.compare_sorted(["a", "b", "c"], ["a", "b", "c"])
      assert result == [{3, "a"}, {3, "b"}, {3, "c"}]
    end

    test "disjoint inputs produce columns 1 and 2 only" do
      result = UnixTools.Comm.compare_sorted(["a", "c"], ["b", "d"])
      assert result == [{1, "a"}, {2, "b"}, {1, "c"}, {2, "d"}]
    end

    test "empty first input" do
      result = UnixTools.Comm.compare_sorted([], ["a", "b"])
      assert result == [{2, "a"}, {2, "b"}]
    end

    test "empty second input" do
      result = UnixTools.Comm.compare_sorted(["a", "b"], [])
      assert result == [{1, "a"}, {1, "b"}]
    end

    test "both empty" do
      result = UnixTools.Comm.compare_sorted([], [])
      assert result == []
    end

    test "suppress column 1" do
      result = UnixTools.Comm.compare_sorted(
        ["a", "b"],
        ["b", "c"],
        %{suppress: MapSet.new([1])}
      )

      assert result == [{3, "b"}, {2, "c"}]
    end

    test "suppress columns 1 and 2 (show only common)" do
      result = UnixTools.Comm.compare_sorted(
        ["a", "b", "c"],
        ["b", "d"],
        %{suppress: MapSet.new([1, 2])}
      )

      assert result == [{3, "b"}]
    end

    test "suppress column 3 (show only unique)" do
      result = UnixTools.Comm.compare_sorted(
        ["a", "b"],
        ["b", "c"],
        %{suppress: MapSet.new([3])}
      )

      assert result == [{1, "a"}, {2, "c"}]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: format_output/2
  # ---------------------------------------------------------------------------

  describe "format_output/2" do
    test "column 1 has no tabs" do
      result = UnixTools.Comm.format_output([{1, "hello"}], MapSet.new())
      assert result == ["hello"]
    end

    test "column 2 has one tab" do
      result = UnixTools.Comm.format_output([{2, "hello"}], MapSet.new())
      assert result == ["\thello"]
    end

    test "column 3 has two tabs" do
      result = UnixTools.Comm.format_output([{3, "hello"}], MapSet.new())
      assert result == ["\t\thello"]
    end

    test "suppressed column adjusts tabs" do
      # When column 1 is suppressed, column 2 gets 0 tabs, column 3 gets 1 tab.
      result = UnixTools.Comm.format_output(
        [{2, "world"}, {3, "common"}],
        MapSet.new([1])
      )

      assert result == ["world", "\tcommon"]
    end

    test "multiple suppressions adjust tabs" do
      # When columns 1 and 2 are suppressed, column 3 gets 0 tabs.
      result = UnixTools.Comm.format_output(
        [{3, "common"}],
        MapSet.new([1, 2])
      )

      assert result == ["common"]
    end

    test "mixed columns format correctly" do
      result = UnixTools.Comm.format_output(
        [{1, "a"}, {2, "b"}, {3, "c"}],
        MapSet.new()
      )

      assert result == ["a", "\tb", "\t\tc"]
    end
  end
end
