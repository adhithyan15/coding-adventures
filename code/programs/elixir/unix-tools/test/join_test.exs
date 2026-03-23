defmodule JoinTest do
  @moduledoc """
  Tests for the join tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Flag parsing (-1, -2, -j, -t, -a, -v, -e, -i).
  3. Business logic: parse_line, get_join_key, compare_keys.
  4. Business logic: format_output, format_unpaired.
  5. Business logic: merge_join (the core algorithm).
  6. Business logic: collect_same_key (duplicate key handling).
  7. Edge cases: empty inputs, unpairable lines, case-insensitive joins.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "join.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the join spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "two file arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["join", "f1.txt", "f2.txt"])
      assert arguments["file1"] == "f1.txt"
      assert arguments["file2"] == "f2.txt"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Flags
  # ---------------------------------------------------------------------------

  describe "flags" do
    test "-1 sets field1" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["join", "-1", "3", "f1", "f2"])
      assert flags["field1"] == 3
    end

    test "-2 sets field2" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["join", "-2", "2", "f1", "f2"])
      assert flags["field2"] == 2
    end

    test "-j sets join_field" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["join", "-j", "2", "f1", "f2"])
      assert flags["join_field"] == 2
    end

    test "-t sets separator" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["join", "-t", ",", "f1", "f2"])
      assert flags["separator"] == ","
    end

    test "-i sets ignore_case" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["join", "-i", "f1", "f2"])
      assert flags["ignore_case"] == true
    end

    test "-e sets empty replacement" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["join", "-e", "N/A", "f1", "f2"])
      assert flags["empty"] == "N/A"
    end

    test "-a sets unpaired" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["join", "-a", "1", "f1", "f2"])
      assert flags["unpaired"] == ["1"]
    end

    test "-v sets only_unpaired" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["join", "-v", "2", "f1", "f2"])
      assert flags["only_unpaired"] == "2"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help and --version
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["join", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["join", "--help"])
      assert text =~ "join"
    end
  end

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["join", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["join", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["join", "--unknown", "f1", "f2"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: parse_line/2
  # ---------------------------------------------------------------------------

  describe "parse_line/2" do
    test "splits on whitespace by default" do
      assert UnixTools.Join.parse_line("alice 30 engineer", nil) ==
               ["alice", "30", "engineer"]
    end

    test "splits on custom separator" do
      assert UnixTools.Join.parse_line("alice,30,engineer", ",") ==
               ["alice", "30", "engineer"]
    end

    test "handles multiple spaces as whitespace split" do
      result = UnixTools.Join.parse_line("  spaced   out  ", nil)
      assert result == ["spaced", "out"]
    end

    test "handles tab separator" do
      assert UnixTools.Join.parse_line("a\tb\tc", "\t") == ["a", "b", "c"]
    end

    test "single field line" do
      assert UnixTools.Join.parse_line("solo", nil) == ["solo"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: get_join_key/2
  # ---------------------------------------------------------------------------

  describe "get_join_key/2" do
    test "extracts field at given index" do
      assert UnixTools.Join.get_join_key(["alice", "30", "engineer"], 0) == "alice"
      assert UnixTools.Join.get_join_key(["alice", "30", "engineer"], 1) == "30"
      assert UnixTools.Join.get_join_key(["alice", "30", "engineer"], 2) == "engineer"
    end

    test "returns empty string for out-of-bounds index" do
      assert UnixTools.Join.get_join_key(["alice", "30"], 5) == ""
    end

    test "works with single-field list" do
      assert UnixTools.Join.get_join_key(["only"], 0) == "only"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: compare_keys/3
  # ---------------------------------------------------------------------------

  describe "compare_keys/3" do
    test "returns :lt when first key is smaller" do
      assert UnixTools.Join.compare_keys("alice", "bob", false) == :lt
    end

    test "returns :gt when first key is larger" do
      assert UnixTools.Join.compare_keys("bob", "alice", false) == :gt
    end

    test "returns :eq when keys are equal" do
      assert UnixTools.Join.compare_keys("same", "same", false) == :eq
    end

    test "case-insensitive comparison" do
      assert UnixTools.Join.compare_keys("Alice", "alice", true) == :eq
      assert UnixTools.Join.compare_keys("Alice", "BOB", true) == :lt
    end

    test "case-sensitive comparison treats uppercase differently" do
      assert UnixTools.Join.compare_keys("Alice", "alice", false) == :lt
    end
  end

  # ---------------------------------------------------------------------------
  # Test: format_output/6
  # ---------------------------------------------------------------------------

  describe "format_output/6" do
    test "joins fields with space separator" do
      result = UnixTools.Join.format_output(
        "alice",
        ["alice", "30"],
        ["alice", "engineer"],
        0, 0, nil
      )

      assert result == "alice 30 engineer"
    end

    test "joins fields with custom separator" do
      result = UnixTools.Join.format_output(
        "alice",
        ["alice", "30"],
        ["alice", "engineer"],
        0, 0, ","
      )

      assert result == "alice,30,engineer"
    end

    test "handles different join field indices" do
      result = UnixTools.Join.format_output(
        "key",
        ["data1", "key"],
        ["key", "data2"],
        1, 0, nil
      )

      assert result == "key data1 data2"
    end

    test "handles single-field lines" do
      result = UnixTools.Join.format_output(
        "only",
        ["only"],
        ["only"],
        0, 0, nil
      )

      assert result == "only"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: format_unpaired/4
  # ---------------------------------------------------------------------------

  describe "format_unpaired/4" do
    test "formats an unpairable line" do
      result = UnixTools.Join.format_unpaired("dave", ["dave", "40"], 0, nil)
      assert result == "dave 40"
    end

    test "formats with custom separator" do
      result = UnixTools.Join.format_unpaired("dave", ["dave", "40"], 0, ",")
      assert result == "dave,40"
    end

    test "handles single-field unpairable" do
      result = UnixTools.Join.format_unpaired("solo", ["solo"], 0, nil)
      assert result == "solo"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: collect_same_key/4
  # ---------------------------------------------------------------------------

  describe "collect_same_key/4" do
    test "collects consecutive lines with the same key" do
      lines = [["alice", "1"], ["alice", "2"], ["bob", "3"]]
      {same, remaining} = UnixTools.Join.collect_same_key(lines, 0, "alice", false)

      assert same == [["alice", "1"], ["alice", "2"]]
      assert remaining == [["bob", "3"]]
    end

    test "single matching line" do
      lines = [["alice", "1"], ["bob", "2"]]
      {same, remaining} = UnixTools.Join.collect_same_key(lines, 0, "alice", false)

      assert same == [["alice", "1"]]
      assert remaining == [["bob", "2"]]
    end

    test "all lines match" do
      lines = [["a", "1"], ["a", "2"], ["a", "3"]]
      {same, remaining} = UnixTools.Join.collect_same_key(lines, 0, "a", false)

      assert same == [["a", "1"], ["a", "2"], ["a", "3"]]
      assert remaining == []
    end

    test "no lines match" do
      lines = [["bob", "1"]]
      {same, remaining} = UnixTools.Join.collect_same_key(lines, 0, "alice", false)

      assert same == []
      assert remaining == [["bob", "1"]]
    end

    test "case-insensitive key matching" do
      lines = [["Alice", "1"], ["alice", "2"], ["Bob", "3"]]
      {same, remaining} = UnixTools.Join.collect_same_key(lines, 0, "alice", true)

      assert same == [["Alice", "1"], ["alice", "2"]]
      assert remaining == [["Bob", "3"]]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: merge_join/3
  # ---------------------------------------------------------------------------

  describe "merge_join/3" do
    @default_opts %{
      field1: 0,
      field2: 0,
      separator: nil,
      empty: "",
      ignore_case: false,
      unpaired: [],
      only_unpaired: nil
    }

    test "basic join on matching keys" do
      lines1 = ["alice engineer", "bob designer", "carol manager"]
      lines2 = ["alice 30", "bob 25", "carol 35"]

      result = UnixTools.Join.merge_join(lines1, lines2, @default_opts)

      assert result == [
               "alice engineer 30",
               "bob designer 25",
               "carol manager 35"
             ]
    end

    test "unmatched lines are suppressed by default" do
      lines1 = ["alice engineer", "bob designer"]
      lines2 = ["alice 30", "carol 35"]

      result = UnixTools.Join.merge_join(lines1, lines2, @default_opts)

      assert result == ["alice engineer 30"]
    end

    test "unpaired lines from file 1 with -a 1" do
      opts = %{@default_opts | unpaired: [1]}
      lines1 = ["alice engineer", "bob designer"]
      lines2 = ["alice 30"]

      result = UnixTools.Join.merge_join(lines1, lines2, opts)

      assert result == [
               "alice engineer 30",
               "bob designer"
             ]
    end

    test "unpaired lines from file 2 with -a 2" do
      opts = %{@default_opts | unpaired: [2]}
      lines1 = ["alice engineer"]
      lines2 = ["alice 30", "carol 35"]

      result = UnixTools.Join.merge_join(lines1, lines2, opts)

      assert result == [
               "alice engineer 30",
               "carol 35"
             ]
    end

    test "only unpaired from file 1 with -v 1" do
      opts = %{@default_opts | only_unpaired: 1}
      lines1 = ["alice engineer", "bob designer"]
      lines2 = ["alice 30"]

      result = UnixTools.Join.merge_join(lines1, lines2, opts)

      assert result == ["bob designer"]
    end

    test "only unpaired from file 2 with -v 2" do
      opts = %{@default_opts | only_unpaired: 2}
      lines1 = ["alice engineer"]
      lines2 = ["alice 30", "carol 35"]

      result = UnixTools.Join.merge_join(lines1, lines2, opts)

      assert result == ["carol 35"]
    end

    test "empty file 1" do
      result = UnixTools.Join.merge_join([], ["alice 30"], @default_opts)
      assert result == []
    end

    test "empty file 2" do
      result = UnixTools.Join.merge_join(["alice engineer"], [], @default_opts)
      assert result == []
    end

    test "both files empty" do
      result = UnixTools.Join.merge_join([], [], @default_opts)
      assert result == []
    end

    test "empty file 1 with unpaired from file 2" do
      opts = %{@default_opts | unpaired: [2]}
      result = UnixTools.Join.merge_join([], ["alice 30", "bob 25"], opts)
      assert result == ["alice 30", "bob 25"]
    end

    test "join on second field" do
      opts = %{@default_opts | field1: 1, field2: 1}
      lines1 = ["engineer alice", "designer bob"]
      lines2 = ["30 alice", "25 bob"]

      result = UnixTools.Join.merge_join(lines1, lines2, opts)

      assert result == [
               "alice engineer 30",
               "bob designer 25"
             ]
    end

    test "case-insensitive join" do
      opts = %{@default_opts | ignore_case: true}
      lines1 = ["Alice engineer"]
      lines2 = ["alice 30"]

      result = UnixTools.Join.merge_join(lines1, lines2, opts)

      assert result == ["Alice engineer 30"]
    end

    test "duplicate keys produce cross-product" do
      lines1 = ["key a", "key b"]
      lines2 = ["key x", "key y"]

      result = UnixTools.Join.merge_join(lines1, lines2, @default_opts)

      assert result == [
               "key a x",
               "key a y",
               "key b x",
               "key b y"
             ]
    end

    test "custom separator" do
      opts = %{@default_opts | separator: ","}
      lines1 = ["alice,engineer"]
      lines2 = ["alice,30"]

      result = UnixTools.Join.merge_join(lines1, lines2, opts)

      assert result == ["alice,engineer,30"]
    end
  end
end
