defmodule DiffTest do
  @moduledoc """
  Tests for the diff tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version).
  2. LCS (Longest Common Subsequence) algorithm correctness.
  3. Edit script generation from LCS.
  4. Normal, unified, and context output formats.
  5. Normalization flags (-i, -b, -w, -B).
  6. Edge cases (identical files, empty files, completely different files).
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "diff.json"]) |> Path.expand()

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: CLI parsing
  # ---------------------------------------------------------------------------

  describe "CLI parsing" do
    test "two file arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["diff", "f1", "f2"])
      assert arguments["file1"] == "f1"
      assert arguments["file2"] == "f2"
    end

    test "-u sets unified format" do
      # -u takes an optional integer value, default 3. Pass it explicitly.
      {:ok, %ParseResult{flags: flags}} = parse_argv(["diff", "-u", "3", "f1", "f2"])
      assert flags["unified"] != nil
    end

    test "-i sets ignore case" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["diff", "-i", "f1", "f2"])
      assert flags["ignore_case"] == true
    end

    test "-q sets brief mode" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["diff", "-q", "f1", "f2"])
      assert flags["brief"] == true
    end

    test "-b sets ignore-space-change" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["diff", "-b", "f1", "f2"])
      assert flags["ignore_space_change"] == true
    end

    test "-w sets ignore-all-space" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["diff", "-w", "f1", "f2"])
      assert flags["ignore_all_space"] == true
    end

    test "-B sets ignore-blank-lines" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["diff", "-B", "f1", "f2"])
      assert flags["ignore_blank_lines"] == true
    end

    test "-r sets recursive" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["diff", "-r", "f1", "f2"])
      assert flags["recursive"] == true
    end

    test "--help returns help text" do
      {:ok, %HelpResult{text: text}} = parse_argv(["diff", "--help"])
      assert text =~ "diff"
    end

    test "--version returns version" do
      {:ok, %VersionResult{version: version}} = parse_argv(["diff", "--version"])
      assert version =~ "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: normalize_line
  # ---------------------------------------------------------------------------

  describe "normalize_line" do
    test "no options returns line unchanged" do
      assert UnixTools.Diff.normalize_line("Hello World") == "Hello World"
    end

    test "ignore_case downcases" do
      assert UnixTools.Diff.normalize_line("Hello World", %{ignore_case: true}) == "hello world"
    end

    test "ignore_space_change collapses whitespace" do
      assert UnixTools.Diff.normalize_line("a  b  c", %{ignore_space_change: true}) == "a b c"
    end

    test "ignore_all_space removes all whitespace" do
      assert UnixTools.Diff.normalize_line("a b c", %{ignore_all_space: true}) == "abc"
    end

    test "combined ignore_case and ignore_space_change" do
      result = UnixTools.Diff.normalize_line("Hello  World", %{ignore_case: true, ignore_space_change: true})
      assert result == "hello world"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: compute_lcs
  # ---------------------------------------------------------------------------

  describe "compute_lcs" do
    test "identical lists have full LCS" do
      lcs = UnixTools.Diff.compute_lcs(["a", "b", "c"], ["a", "b", "c"], %{})
      assert lcs == [{0, 0}, {1, 1}, {2, 2}]
    end

    test "completely different lists have empty LCS" do
      lcs = UnixTools.Diff.compute_lcs(["a", "b", "c"], ["x", "y", "z"], %{})
      assert lcs == []
    end

    test "partial overlap" do
      lcs = UnixTools.Diff.compute_lcs(["a", "b", "c", "d"], ["a", "c", "d"], %{})
      assert lcs == [{0, 0}, {2, 1}, {3, 2}]
    end

    test "single common element" do
      lcs = UnixTools.Diff.compute_lcs(["x", "a", "y"], ["p", "a", "q"], %{})
      assert lcs == [{1, 1}]
    end

    test "empty first list" do
      lcs = UnixTools.Diff.compute_lcs([], ["a", "b"], %{})
      assert lcs == []
    end

    test "empty second list" do
      lcs = UnixTools.Diff.compute_lcs(["a", "b"], [], %{})
      assert lcs == []
    end

    test "both empty" do
      lcs = UnixTools.Diff.compute_lcs([], [], %{})
      assert lcs == []
    end

    test "ignore case in LCS" do
      lcs = UnixTools.Diff.compute_lcs(["Hello", "World"], ["hello", "world"], %{ignore_case: true})
      assert lcs == [{0, 0}, {1, 1}]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: compute_edit_script
  # ---------------------------------------------------------------------------

  describe "compute_edit_script" do
    test "identical files produce only equal ops" do
      lcs = [{0, 0}, {1, 1}, {2, 2}]
      script = UnixTools.Diff.compute_edit_script(3, 3, lcs)
      assert Enum.all?(script, fn {op, _, _} -> op == :equal end)
    end

    test "deletion produces delete ops" do
      # file1: ["a", "b", "c"], file2: ["a", "c"]
      lcs = [{0, 0}, {2, 1}]
      script = UnixTools.Diff.compute_edit_script(3, 2, lcs)

      assert {:equal, 0, 0} in script
      assert {:delete, 1} in script
      assert {:equal, 2, 1} in script
    end

    test "insertion produces insert ops" do
      # file1: ["a", "c"], file2: ["a", "b", "c"]
      lcs = [{0, 0}, {1, 2}]
      script = UnixTools.Diff.compute_edit_script(2, 3, lcs)

      assert {:equal, 0, 0} in script
      assert {:insert, 1} in script
      assert {:equal, 1, 2} in script
    end

    test "completely different files" do
      lcs = []
      script = UnixTools.Diff.compute_edit_script(2, 2, lcs)

      deletes = Enum.filter(script, fn {op, _} -> op == :delete end)
      inserts = Enum.filter(script, fn {op, _} -> op == :insert end)
      assert length(deletes) == 2
      assert length(inserts) == 2
    end

    test "empty files produce empty script" do
      script = UnixTools.Diff.compute_edit_script(0, 0, [])
      assert script == []
    end
  end

  # ---------------------------------------------------------------------------
  # Test: format_normal
  # ---------------------------------------------------------------------------

  describe "format_normal" do
    test "single deletion" do
      lines1 = ["a", "b", "c"]
      lines2 = ["a", "c"]
      lcs = UnixTools.Diff.compute_lcs(lines1, lines2, %{})
      script = UnixTools.Diff.compute_edit_script(3, 2, lcs)
      output = UnixTools.Diff.format_normal(script, lines1, lines2)

      assert output =~ "< b"
      assert output =~ "d"
    end

    test "single insertion" do
      lines1 = ["a", "c"]
      lines2 = ["a", "b", "c"]
      lcs = UnixTools.Diff.compute_lcs(lines1, lines2, %{})
      script = UnixTools.Diff.compute_edit_script(2, 3, lcs)
      output = UnixTools.Diff.format_normal(script, lines1, lines2)

      assert output =~ "> b"
      assert output =~ "a"
    end

    test "change (delete + insert at same position)" do
      lines1 = ["a", "old", "c"]
      lines2 = ["a", "new", "c"]
      lcs = UnixTools.Diff.compute_lcs(lines1, lines2, %{})
      script = UnixTools.Diff.compute_edit_script(3, 3, lcs)
      output = UnixTools.Diff.format_normal(script, lines1, lines2)

      assert output =~ "< old"
      assert output =~ "> new"
      assert output =~ "---"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: format_unified
  # ---------------------------------------------------------------------------

  describe "format_unified" do
    test "produces @@ header" do
      lines1 = ["a", "b", "c"]
      lines2 = ["a", "x", "c"]
      lcs = UnixTools.Diff.compute_lcs(lines1, lines2, %{})
      script = UnixTools.Diff.compute_edit_script(3, 3, lcs)
      output = UnixTools.Diff.format_unified(script, lines1, lines2, 3)

      assert output =~ "@@"
    end

    test "shows deleted lines with minus prefix" do
      lines1 = ["a", "b", "c"]
      lines2 = ["a", "c"]
      lcs = UnixTools.Diff.compute_lcs(lines1, lines2, %{})
      script = UnixTools.Diff.compute_edit_script(3, 2, lcs)
      output = UnixTools.Diff.format_unified(script, lines1, lines2, 3)

      assert output =~ "-b"
    end

    test "shows added lines with plus prefix" do
      lines1 = ["a", "c"]
      lines2 = ["a", "b", "c"]
      lcs = UnixTools.Diff.compute_lcs(lines1, lines2, %{})
      script = UnixTools.Diff.compute_edit_script(2, 3, lcs)
      output = UnixTools.Diff.format_unified(script, lines1, lines2, 3)

      assert output =~ "+b"
    end

    test "shows context lines with space prefix" do
      lines1 = ["a", "b", "c"]
      lines2 = ["a", "x", "c"]
      lcs = UnixTools.Diff.compute_lcs(lines1, lines2, %{})
      script = UnixTools.Diff.compute_edit_script(3, 3, lcs)
      output = UnixTools.Diff.format_unified(script, lines1, lines2, 3)

      assert output =~ " a"
      assert output =~ " c"
    end

    test "identical files produce empty output" do
      lines = ["a", "b", "c"]
      lcs = UnixTools.Diff.compute_lcs(lines, lines, %{})
      script = UnixTools.Diff.compute_edit_script(3, 3, lcs)
      output = UnixTools.Diff.format_unified(script, lines, lines, 3)

      assert output == ""
    end
  end

  # ---------------------------------------------------------------------------
  # Test: format_context
  # ---------------------------------------------------------------------------

  describe "format_context" do
    test "produces separator and file headers" do
      lines1 = ["a", "b", "c"]
      lines2 = ["a", "x", "c"]
      lcs = UnixTools.Diff.compute_lcs(lines1, lines2, %{})
      script = UnixTools.Diff.compute_edit_script(3, 3, lcs)
      output = UnixTools.Diff.format_context(script, lines1, lines2, 3)

      assert output =~ "***"
      assert output =~ "---"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: diff_lines (high-level)
  # ---------------------------------------------------------------------------

  describe "diff_lines" do
    test "identical lines return :identical" do
      lines = ["a", "b", "c"]
      assert {:identical, ""} = UnixTools.Diff.diff_lines(lines, lines)
    end

    test "different lines return :different with output" do
      lines1 = ["a", "b"]
      lines2 = ["a", "c"]
      assert {:different, output} = UnixTools.Diff.diff_lines(lines1, lines2)
      assert output != ""
    end

    test "brief mode returns :brief atom" do
      lines1 = ["a"]
      lines2 = ["b"]
      assert {:different, :brief} = UnixTools.Diff.diff_lines(lines1, lines2, %{brief: true})
    end

    test "unified mode produces unified output" do
      lines1 = ["a", "b"]
      lines2 = ["a", "c"]
      {:different, output} = UnixTools.Diff.diff_lines(lines1, lines2, %{unified: 3})
      assert output =~ "@@"
    end

    test "context mode produces context output" do
      lines1 = ["a", "b"]
      lines2 = ["a", "c"]
      {:different, output} = UnixTools.Diff.diff_lines(lines1, lines2, %{context_format: 3})
      assert output =~ "***"
    end

    test "ignore_case treats case-different lines as equal" do
      lines1 = ["Hello", "World"]
      lines2 = ["hello", "world"]
      assert {:identical, ""} = UnixTools.Diff.diff_lines(lines1, lines2, %{ignore_case: true})
    end

    test "ignore_all_space treats whitespace-different lines as equal" do
      lines1 = ["a b c"]
      lines2 = ["abc"]
      assert {:identical, ""} = UnixTools.Diff.diff_lines(lines1, lines2, %{ignore_all_space: true})
    end
  end

  # ---------------------------------------------------------------------------
  # Test: filter_lines
  # ---------------------------------------------------------------------------

  describe "filter_lines" do
    test "without ignore_blank_lines preserves all lines" do
      lines = ["a", "", "b", "", "c"]
      result = UnixTools.Diff.filter_lines(lines)
      assert length(result) == 5
    end

    test "with ignore_blank_lines removes blank lines" do
      lines = ["a", "", "b", "  ", "c"]
      result = UnixTools.Diff.filter_lines(lines, %{ignore_blank_lines: true})
      assert length(result) == 3
    end
  end

  # ---------------------------------------------------------------------------
  # Test: File-based diff
  # ---------------------------------------------------------------------------

  describe "file-based diff" do
    @tag :tmp_dir
    test "diff identical files", %{tmp_dir: tmp} do
      path1 = Path.join(tmp, "file1.txt")
      path2 = Path.join(tmp, "file2.txt")
      content = "line1\nline2\nline3\n"
      File.write!(path1, content)
      File.write!(path2, content)

      lines1 = String.split(content, "\n", trim: true)
      lines2 = String.split(content, "\n", trim: true)
      assert {:identical, ""} = UnixTools.Diff.diff_lines(lines1, lines2)
    end

    @tag :tmp_dir
    test "diff files with one changed line", %{tmp_dir: tmp} do
      path1 = Path.join(tmp, "file1.txt")
      path2 = Path.join(tmp, "file2.txt")
      File.write!(path1, "line1\nline2\nline3\n")
      File.write!(path2, "line1\nchanged\nline3\n")

      lines1 = File.read!(path1) |> String.split("\n", trim: true)
      lines2 = File.read!(path2) |> String.split("\n", trim: true)
      assert {:different, output} = UnixTools.Diff.diff_lines(lines1, lines2)
      assert output =~ "line2"
      assert output =~ "changed"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: group_into_hunks
  # ---------------------------------------------------------------------------

  describe "group_into_hunks" do
    test "no changes produce no hunks" do
      script = [{:equal, 0, 0}, {:equal, 1, 1}]
      assert UnixTools.Diff.group_into_hunks(script) == []
    end

    test "single change produces one hunk" do
      script = [{:equal, 0, 0}, {:delete, 1}, {:equal, 2, 1}]
      hunks = UnixTools.Diff.group_into_hunks(script, 1)
      assert length(hunks) == 1
    end
  end
end
