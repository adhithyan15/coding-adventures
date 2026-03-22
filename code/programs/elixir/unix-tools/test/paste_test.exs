defmodule PasteTest do
  @moduledoc """
  Tests for the paste tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version).
  2. Parallel mode (default) -- merging corresponding lines.
  3. Serial mode (-s) -- pasting all lines onto one line.
  4. Delimiter parsing and cycling.
  5. Unequal-length inputs.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "paste.json"]) |> Path.expand()

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
    test "file arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["paste", "f1", "f2"])
      assert arguments["files"] == ["f1", "f2"]
    end

    test "-s sets serial to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["paste", "-s", "f1"])
      assert flags["serial"] == true
    end

    test "-d sets delimiters" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["paste", "-d", ",", "f1", "f2"])
      assert flags["delimiters"] == ","
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help flag
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["paste", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["paste", "--help"])
      assert text =~ "paste"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --version flag
  # ---------------------------------------------------------------------------

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["paste", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["paste", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["paste", "--unknown", "f1"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Delimiter parsing
  # ---------------------------------------------------------------------------

  describe "parse_delimiters/1" do
    test "nil returns tab" do
      assert UnixTools.Paste.parse_delimiters(nil) == ["\t"]
    end

    test "empty string returns tab" do
      assert UnixTools.Paste.parse_delimiters("") == ["\t"]
    end

    test "single character" do
      assert UnixTools.Paste.parse_delimiters(",") == [","]
    end

    test "multiple characters" do
      assert UnixTools.Paste.parse_delimiters(",:") == [",", ":"]
    end

    test "escaped newline" do
      assert UnixTools.Paste.parse_delimiters("\\n") == ["\n"]
    end

    test "escaped tab" do
      assert UnixTools.Paste.parse_delimiters("\\t") == ["\t"]
    end

    test "escaped backslash" do
      assert UnixTools.Paste.parse_delimiters("\\\\") == ["\\"]
    end

    test "escaped zero (empty delimiter)" do
      assert UnixTools.Paste.parse_delimiters("\\0") == [""]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Parallel mode
  # ---------------------------------------------------------------------------

  describe "paste_parallel/2" do
    test "two columns with tab delimiter" do
      result = UnixTools.Paste.paste_parallel([["a", "b"], ["1", "2"]])
      assert result == ["a\t1", "b\t2"]
    end

    test "two columns with custom delimiter" do
      result = UnixTools.Paste.paste_parallel([["a", "b"], ["1", "2"]], [","])
      assert result == ["a,1", "b,2"]
    end

    test "three columns" do
      result = UnixTools.Paste.paste_parallel([["a"], ["1"], ["x"]], ["\t"])
      assert result == ["a\t1\tx"]
    end

    test "unequal lengths pad with empty strings" do
      result = UnixTools.Paste.paste_parallel([["a", "b", "c"], ["1"]], ["\t"])
      assert result == ["a\t1", "b\t", "c\t"]
    end

    test "cycling delimiters" do
      result = UnixTools.Paste.paste_parallel(
        [["a"], ["1"], ["x"]],
        [",", ":"]
      )
      assert result == ["a,1:x"]
    end

    test "empty columns" do
      result = UnixTools.Paste.paste_parallel([[], []])
      assert result == []
    end

    test "single column" do
      result = UnixTools.Paste.paste_parallel([["a", "b", "c"]])
      assert result == ["a", "b", "c"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Serial mode
  # ---------------------------------------------------------------------------

  describe "paste_serial/2" do
    test "joins all lines with delimiter" do
      result = UnixTools.Paste.paste_serial([["a", "b", "c"]])
      assert result == ["a\tb\tc"]
    end

    test "custom delimiter" do
      result = UnixTools.Paste.paste_serial([["a", "b", "c"]], [","])
      assert result == ["a,b,c"]
    end

    test "multiple files each produce one line" do
      result = UnixTools.Paste.paste_serial(
        [["a", "b"], ["1", "2"]],
        [","]
      )
      assert result == ["a,b", "1,2"]
    end

    test "cycling delimiters in serial mode" do
      result = UnixTools.Paste.paste_serial([["a", "b", "c", "d"]], [",", ":"])
      assert result == ["a,b:c,d"]
    end

    test "single element" do
      result = UnixTools.Paste.paste_serial([["hello"]])
      assert result == ["hello"]
    end
  end
end
