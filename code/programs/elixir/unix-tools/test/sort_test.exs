defmodule SortTest do
  @moduledoc """
  Tests for the sort tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Lexicographic sorting (default behavior).
  3. Numeric sorting (-n).
  4. Human-numeric sorting (-h).
  5. Month sorting (-M).
  6. General numeric sorting (-g).
  7. Version sorting (-V).
  8. Modifier flags (-r, -u, -f, -d, -b).
  9. Sorted-check mode (-c).
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "sort.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the sort spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "no arguments returns ParseResult" do
      assert {:ok, %ParseResult{}} = parse_argv(["sort"])
    end

    test "file arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["sort", "file.txt"])
      assert arguments["files"] == ["file.txt"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Flags
  # ---------------------------------------------------------------------------

  describe "flags" do
    test "-r sets reverse to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["sort", "-r"])
      assert flags["reverse"] == true
    end

    test "-n sets numeric_sort to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["sort", "-n"])
      assert flags["numeric_sort"] == true
    end

    test "-u sets unique to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["sort", "-u"])
      assert flags["unique"] == true
    end

    test "-f sets ignore_case to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["sort", "-f"])
      assert flags["ignore_case"] == true
    end

    test "--numeric-sort long flag works" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["sort", "--numeric-sort"])
      assert flags["numeric_sort"] == true
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help flag
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["sort", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["sort", "--help"])
      assert text =~ "sort"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --version flag
  # ---------------------------------------------------------------------------

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["sort", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["sort", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags produce errors
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["sort", "--unknown"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Lexicographic sorting
  # ---------------------------------------------------------------------------

  describe "sort_lines/2 lexicographic" do
    test "sorts lines alphabetically" do
      assert UnixTools.Sort.sort_lines(["banana", "apple", "cherry"]) ==
               ["apple", "banana", "cherry"]
    end

    test "handles empty list" do
      assert UnixTools.Sort.sort_lines([]) == []
    end

    test "handles single element" do
      assert UnixTools.Sort.sort_lines(["hello"]) == ["hello"]
    end

    test "handles already sorted input" do
      assert UnixTools.Sort.sort_lines(["a", "b", "c"]) == ["a", "b", "c"]
    end

    test "case-sensitive by default (uppercase before lowercase)" do
      result = UnixTools.Sort.sort_lines(["banana", "Apple", "cherry"])
      assert List.first(result) == "Apple"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Reverse sorting
  # ---------------------------------------------------------------------------

  describe "sort_lines/2 reverse" do
    test "reverses the sort order" do
      assert UnixTools.Sort.sort_lines(["a", "b", "c"], %{reverse: true}) ==
               ["c", "b", "a"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unique filtering
  # ---------------------------------------------------------------------------

  describe "sort_lines/2 unique" do
    test "removes duplicate lines" do
      assert UnixTools.Sort.sort_lines(["a", "b", "a", "c", "b"], %{unique: true}) ==
               ["a", "b", "c"]
    end

    test "unique with reverse" do
      result = UnixTools.Sort.sort_lines(["a", "b", "a", "c"], %{unique: true, reverse: true})
      assert result == ["c", "b", "a"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Numeric sorting
  # ---------------------------------------------------------------------------

  describe "sort_lines/2 numeric" do
    test "sorts by numeric value" do
      assert UnixTools.Sort.sort_lines(["10", "2", "1", "20"], %{numeric: true}) ==
               ["1", "2", "10", "20"]
    end

    test "non-numeric lines sort as 0" do
      result = UnixTools.Sort.sort_lines(["5", "abc", "3"], %{numeric: true})
      # abc parses as 0, so it comes first
      assert List.first(result) == "abc"
    end

    test "handles negative numbers" do
      assert UnixTools.Sort.sort_lines(["5", "-3", "0"], %{numeric: true}) ==
               ["-3", "0", "5"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Human-numeric sorting
  # ---------------------------------------------------------------------------

  describe "sort_lines/2 human_numeric" do
    test "sorts human-readable sizes" do
      result = UnixTools.Sort.sort_lines(["1M", "2K", "1G"], %{human_numeric: true})
      assert result == ["2K", "1M", "1G"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Month sorting
  # ---------------------------------------------------------------------------

  describe "sort_lines/2 month" do
    test "sorts by month abbreviation" do
      result = UnixTools.Sort.sort_lines(["MAR", "JAN", "FEB"], %{month: true})
      assert result == ["JAN", "FEB", "MAR"]
    end

    test "unknown months sort before JAN" do
      result = UnixTools.Sort.sort_lines(["FEB", "unknown", "JAN"], %{month: true})
      assert List.first(result) == "unknown"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Version sorting
  # ---------------------------------------------------------------------------

  describe "sort_lines/2 version" do
    test "sorts version numbers naturally" do
      result = UnixTools.Sort.sort_lines(["file10", "file2", "file1"], %{version: true})
      assert result == ["file1", "file2", "file10"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Case-insensitive sorting
  # ---------------------------------------------------------------------------

  describe "sort_lines/2 ignore_case" do
    test "folds case for comparison" do
      result = UnixTools.Sort.sort_lines(["Banana", "apple", "Cherry"], %{ignore_case: true})
      assert result == ["apple", "Banana", "Cherry"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Ignore leading blanks
  # ---------------------------------------------------------------------------

  describe "sort_lines/2 ignore_leading_blanks" do
    test "strips leading spaces before comparing" do
      result = UnixTools.Sort.sort_lines(["  b", "a", " c"], %{ignore_leading_blanks: true})
      assert result == ["a", "  b", " c"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Numeric parsers
  # ---------------------------------------------------------------------------

  describe "parse_numeric/1" do
    test "parses integer" do
      assert UnixTools.Sort.parse_numeric("42") == 42.0
    end

    test "parses float" do
      assert UnixTools.Sort.parse_numeric("3.14") == 3.14
    end

    test "parses negative" do
      assert UnixTools.Sort.parse_numeric("-5") == -5.0
    end

    test "non-numeric returns 0" do
      assert UnixTools.Sort.parse_numeric("abc") == 0.0
    end

    test "handles leading whitespace" do
      assert UnixTools.Sort.parse_numeric("  42") == 42.0
    end
  end

  describe "parse_human_numeric/1" do
    test "parses kilobytes" do
      assert UnixTools.Sort.parse_human_numeric("2K") == 2048.0
    end

    test "parses megabytes" do
      assert UnixTools.Sort.parse_human_numeric("1M") == 1_048_576.0
    end

    test "parses plain number" do
      assert UnixTools.Sort.parse_human_numeric("100") == 100.0
    end
  end

  describe "parse_month/1" do
    test "parses JAN" do
      assert UnixTools.Sort.parse_month("JAN") == 1
    end

    test "parses DEC" do
      assert UnixTools.Sort.parse_month("DEC") == 12
    end

    test "case insensitive" do
      assert UnixTools.Sort.parse_month("jan") == 1
    end

    test "unknown returns 0" do
      assert UnixTools.Sort.parse_month("xyz") == 0
    end
  end

  describe "parse_version/1" do
    test "splits into segments" do
      assert UnixTools.Sort.parse_version("file10") == ["file", 10]
    end

    test "handles dotted version" do
      assert UnixTools.Sort.parse_version("1.2.3") == [1, ".", 2, ".", 3]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: check_sorted/2
  # ---------------------------------------------------------------------------

  describe "check_sorted/2" do
    test "returns :ok for sorted input" do
      assert UnixTools.Sort.check_sorted(["a", "b", "c"]) == :ok
    end

    test "returns error for unsorted input" do
      assert {:error, 2, "a"} = UnixTools.Sort.check_sorted(["b", "a", "c"])
    end

    test "returns :ok for empty input" do
      assert UnixTools.Sort.check_sorted([]) == :ok
    end

    test "returns :ok for single element" do
      assert UnixTools.Sort.check_sorted(["hello"]) == :ok
    end
  end
end
