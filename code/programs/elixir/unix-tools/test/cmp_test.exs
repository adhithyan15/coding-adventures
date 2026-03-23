defmodule CmpTest do
  @moduledoc """
  Tests for the cmp tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version).
  2. Byte-by-byte comparison of binaries.
  3. Output formatting (default, verbose, silent).
  4. Skip (-i) and max-bytes (-n) functionality.
  5. Edge cases (identical files, empty files, EOF detection).
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "cmp.json"]) |> Path.expand()

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
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["cmp", "f1", "f2"])
      assert arguments["file1"] == "f1"
      assert arguments["file2"] == "f2"
    end

    test "-l sets list (verbose) mode" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["cmp", "-l", "f1", "f2"])
      assert flags["list"] == true
    end

    test "-s sets silent mode" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["cmp", "-s", "f1", "f2"])
      assert flags["silent"] == true
    end

    test "-b sets print-bytes mode" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["cmp", "-b", "f1", "f2"])
      assert flags["print_bytes"] == true
    end

    test "-n sets max bytes" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["cmp", "-n", "100", "f1", "f2"])
      assert flags["max_bytes"] == 100
    end

    test "-i sets ignore-initial" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["cmp", "-i", "10", "f1", "f2"])
      assert flags["ignore_initial"] == "10"
    end

    test "--help returns help text" do
      {:ok, %HelpResult{text: text}} = parse_argv(["cmp", "--help"])
      assert text =~ "cmp"
    end

    test "--version returns version" do
      {:ok, %VersionResult{version: version}} = parse_argv(["cmp", "--version"])
      assert version =~ "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: compare_bytes — identical inputs
  # ---------------------------------------------------------------------------

  describe "compare_bytes with identical inputs" do
    test "identical binaries return :equal" do
      assert UnixTools.Cmp.compare_bytes("hello", "hello") == :equal
    end

    test "empty binaries return :equal" do
      assert UnixTools.Cmp.compare_bytes("", "") == :equal
    end

    test "single byte identical returns :equal" do
      assert UnixTools.Cmp.compare_bytes("a", "a") == :equal
    end
  end

  # ---------------------------------------------------------------------------
  # Test: compare_bytes — different inputs
  # ---------------------------------------------------------------------------

  describe "compare_bytes with differences" do
    test "single byte difference at position 1" do
      result = UnixTools.Cmp.compare_bytes("a", "b")
      assert {:differ, [{1, ?a, ?b}]} = result
    end

    test "difference in the middle" do
      result = UnixTools.Cmp.compare_bytes("hello", "hxllo")
      assert {:differ, [{2, ?e, ?x}]} = result
    end

    test "multiple differences" do
      result = UnixTools.Cmp.compare_bytes("abc", "xyz")
      assert {:differ, diffs} = result
      assert length(diffs) == 3
      assert {1, ?a, ?x} in diffs
      assert {2, ?b, ?y} in diffs
      assert {3, ?c, ?z} in diffs
    end
  end

  # ---------------------------------------------------------------------------
  # Test: compare_bytes — EOF detection
  # ---------------------------------------------------------------------------

  describe "compare_bytes with different lengths" do
    test "file1 shorter than file2 with matching prefix" do
      result = UnixTools.Cmp.compare_bytes("abc", "abcdef")
      assert {:eof, :file1, 3} = result
    end

    test "file2 shorter than file1 with matching prefix" do
      result = UnixTools.Cmp.compare_bytes("abcdef", "abc")
      assert {:eof, :file2, 3} = result
    end

    test "file1 shorter with differences" do
      result = UnixTools.Cmp.compare_bytes("ax", "abcdef")
      assert {:differ, [{2, ?x, ?b}]} = result
    end
  end

  # ---------------------------------------------------------------------------
  # Test: compare_bytes — skip option
  # ---------------------------------------------------------------------------

  describe "compare_bytes with skip" do
    test "skip equal bytes in both files" do
      # "hello" with skip 2 = "llo"
      # "hello" with skip 2 = "llo"
      result = UnixTools.Cmp.compare_bytes("hello", "hello", %{skip: {2, 2}})
      assert result == :equal
    end

    test "skip reveals difference" do
      # "xxhello" skip 2 = "hello"
      # "xxworld" skip 2 = "world"
      result = UnixTools.Cmp.compare_bytes("xxhello", "xxworld", %{skip: {2, 2}})
      assert {:differ, diffs} = result
      assert length(diffs) > 0
    end

    test "skip past end of file" do
      result = UnixTools.Cmp.compare_bytes("hi", "hello", %{skip: {10, 10}})
      assert result == :equal
    end

    test "asymmetric skip" do
      # "AAhello" skip 2 = "hello"
      # "Bhello" skip 1 = "hello"
      result = UnixTools.Cmp.compare_bytes("AAhello", "Bhello", %{skip: {2, 1}})
      assert result == :equal
    end
  end

  # ---------------------------------------------------------------------------
  # Test: compare_bytes — max_bytes option
  # ---------------------------------------------------------------------------

  describe "compare_bytes with max_bytes" do
    test "max_bytes limits comparison" do
      # Only compare first 3 bytes — both are "hel"
      result = UnixTools.Cmp.compare_bytes("hello", "helps", %{max_bytes: 3})
      assert result == :equal
    end

    test "max_bytes with difference within limit" do
      result = UnixTools.Cmp.compare_bytes("hello", "hxllo", %{max_bytes: 3})
      assert {:differ, [{2, ?e, ?x}]} = result
    end

    test "max_bytes larger than file" do
      result = UnixTools.Cmp.compare_bytes("hi", "hi", %{max_bytes: 100})
      assert result == :equal
    end
  end

  # ---------------------------------------------------------------------------
  # Test: parse_skip
  # ---------------------------------------------------------------------------

  describe "parse_skip" do
    test "nil returns {0, 0}" do
      assert UnixTools.Cmp.parse_skip(nil) == {0, 0}
    end

    test "single number skips both files equally" do
      assert UnixTools.Cmp.parse_skip("10") == {10, 10}
    end

    test "colon-separated skips different amounts" do
      assert UnixTools.Cmp.parse_skip("5:10") == {5, 10}
    end
  end

  # ---------------------------------------------------------------------------
  # Test: format_result
  # ---------------------------------------------------------------------------

  describe "format_result" do
    test "equal files produce empty output with exit 0" do
      result = UnixTools.Cmp.format_result(:equal, "f1", "f2", "", %{})
      assert {:exit, 0, ""} = result
    end

    test "different files in default mode show first difference" do
      diff_result = {:differ, [{5, ?a, ?b}]}
      {:exit, 1, msg} = UnixTools.Cmp.format_result(diff_result, "f1", "f2", "abcd\n", %{verbose: false})
      assert msg =~ "f1 f2 differ: byte 5, line 2"
    end

    test "different files in silent mode produce no output" do
      diff_result = {:differ, [{1, ?a, ?b}]}
      result = UnixTools.Cmp.format_result(diff_result, "f1", "f2", "", %{silent: true})
      assert {:exit, 1, ""} = result
    end

    test "silent mode with equal files" do
      result = UnixTools.Cmp.format_result(:equal, "f1", "f2", "", %{silent: true})
      assert {:exit, 0, ""} = result
    end

    test "verbose mode lists all differences" do
      diff_result = {:differ, [{1, ?a, ?x}, {3, ?c, ?z}]}
      {:exit, 1, msg} = UnixTools.Cmp.format_result(diff_result, "f1", "f2", "", %{verbose: true})
      lines = String.split(msg, "\n")
      assert length(lines) == 2
    end

    test "EOF on file1" do
      result = UnixTools.Cmp.format_result({:eof, :file1, 3}, "f1", "f2", "", %{})
      assert {:exit, 1, msg} = result
      assert msg =~ "EOF on f1"
    end

    test "EOF on file2" do
      result = UnixTools.Cmp.format_result({:eof, :file2, 5}, "f1", "f2", "", %{})
      assert {:exit, 1, msg} = result
      assert msg =~ "EOF on f2"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: format_octal
  # ---------------------------------------------------------------------------

  describe "format_octal" do
    test "formats byte as 3-digit octal" do
      assert UnixTools.Cmp.format_octal(?a) == "141"
      assert UnixTools.Cmp.format_octal(?A) == "101"
      assert UnixTools.Cmp.format_octal(0) == "000"
      assert UnixTools.Cmp.format_octal(255) == "377"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: format_char
  # ---------------------------------------------------------------------------

  describe "format_char" do
    test "printable ASCII shown as character" do
      assert UnixTools.Cmp.format_char(?a) == "a"
      assert UnixTools.Cmp.format_char(?Z) == "Z"
      assert UnixTools.Cmp.format_char(?0) == "0"
      assert UnixTools.Cmp.format_char(32) == " "
    end

    test "non-printable shown as octal escape" do
      result = UnixTools.Cmp.format_char(0)
      assert result =~ "\\"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: count_lines
  # ---------------------------------------------------------------------------

  describe "count_lines" do
    test "no newlines means line 1" do
      assert UnixTools.Cmp.count_lines("abcdef", 0, 3) == 1
    end

    test "one newline before position" do
      assert UnixTools.Cmp.count_lines("ab\ncd\nef", 0, 5) == 2
    end

    test "two newlines before position" do
      assert UnixTools.Cmp.count_lines("a\nb\nc", 0, 5) == 3
    end

    test "skip bytes are excluded" do
      assert UnixTools.Cmp.count_lines("\n\nabcdef", 2, 3) == 1
    end
  end

  # ---------------------------------------------------------------------------
  # Test: format_result with print_bytes
  # ---------------------------------------------------------------------------

  describe "format_result with print_bytes" do
    test "default mode with print_bytes shows octal and char" do
      diff_result = {:differ, [{1, ?a, ?b}]}
      {:exit, 1, msg} = UnixTools.Cmp.format_result(diff_result, "f1", "f2", "", %{print_bytes: true})
      assert msg =~ "141"
      assert msg =~ "142"
    end

    test "verbose mode with print_bytes shows details per line" do
      diff_result = {:differ, [{1, ?a, ?b}, {3, ?c, ?d}]}
      {:exit, 1, msg} = UnixTools.Cmp.format_result(
        diff_result, "f1", "f2", "",
        %{verbose: true, print_bytes: true}
      )
      lines = String.split(msg, "\n")
      assert length(lines) == 2
      assert Enum.at(lines, 0) =~ "a"
      assert Enum.at(lines, 1) =~ "c"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Combined skip + max_bytes
  # ---------------------------------------------------------------------------

  describe "compare_bytes with skip + max_bytes" do
    test "skip then limit" do
      # "XXhello" skip 2 = "hello", max 3 = "hel"
      # "YYhelps" skip 2 = "helps", max 3 = "hel"
      result = UnixTools.Cmp.compare_bytes("XXhello", "YYhelps", %{skip: {2, 2}, max_bytes: 3})
      assert result == :equal
    end
  end

  # ---------------------------------------------------------------------------
  # Test: File-based comparison
  # ---------------------------------------------------------------------------

  describe "file-based comparison" do
    @tag :tmp_dir
    test "compares identical files", %{tmp_dir: tmp} do
      path1 = Path.join(tmp, "file1.txt")
      path2 = Path.join(tmp, "file2.txt")
      File.write!(path1, "hello world")
      File.write!(path2, "hello world")

      bin1 = File.read!(path1)
      bin2 = File.read!(path2)
      assert UnixTools.Cmp.compare_bytes(bin1, bin2) == :equal
    end

    @tag :tmp_dir
    test "detects difference in files", %{tmp_dir: tmp} do
      path1 = Path.join(tmp, "file1.txt")
      path2 = Path.join(tmp, "file2.txt")
      File.write!(path1, "hello world")
      File.write!(path2, "hello earth")

      bin1 = File.read!(path1)
      bin2 = File.read!(path2)
      result = UnixTools.Cmp.compare_bytes(bin1, bin2)
      assert {:differ, diffs} = result
      assert length(diffs) > 0
    end

    @tag :tmp_dir
    test "detects EOF when files differ in length", %{tmp_dir: tmp} do
      path1 = Path.join(tmp, "short.txt")
      path2 = Path.join(tmp, "long.txt")
      File.write!(path1, "abc")
      File.write!(path2, "abcdef")

      bin1 = File.read!(path1)
      bin2 = File.read!(path2)
      result = UnixTools.Cmp.compare_bytes(bin1, bin2)
      assert {:eof, :file1, 3} = result
    end
  end
end
