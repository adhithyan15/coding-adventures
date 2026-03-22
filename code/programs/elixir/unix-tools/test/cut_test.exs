defmodule CutTest do
  @moduledoc """
  Tests for the cut tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Range parsing (single, closed, open-ended, from-start).
  3. Byte cutting.
  4. Character cutting.
  5. Field cutting with delimiters.
  6. Complement mode.
  7. Only-delimited suppression.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "cut.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the cut spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: CLI parsing
  # ---------------------------------------------------------------------------

  describe "CLI parsing" do
    test "-f flag captures field list" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["cut", "-f", "1,3"])
      assert flags["fields"] == "1,3"
    end

    test "-d flag captures delimiter" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["cut", "-f", "1", "-d", ":"])
      assert flags["delimiter"] == ":"
    end

    test "-b flag captures byte list" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["cut", "-b", "1-3"])
      assert flags["bytes"] == "1-3"
    end

    test "-c flag captures character list" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["cut", "-c", "2-4"])
      assert flags["characters"] == "2-4"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help flag
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["cut", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["cut", "--help"])
      assert text =~ "cut"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --version flag
  # ---------------------------------------------------------------------------

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["cut", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["cut", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags produce errors
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["cut", "--unknown"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Range parsing
  # ---------------------------------------------------------------------------

  describe "parse_ranges/1" do
    test "single position" do
      assert UnixTools.Cut.parse_ranges("5") == [{5, 5}]
    end

    test "closed range" do
      assert UnixTools.Cut.parse_ranges("2-7") == [{2, 7}]
    end

    test "open-ended range" do
      assert UnixTools.Cut.parse_ranges("3-") == [{3, :infinity}]
    end

    test "from-start range" do
      assert UnixTools.Cut.parse_ranges("-4") == [{1, 4}]
    end

    test "multiple ranges" do
      result = UnixTools.Cut.parse_ranges("1-3,5,7-")
      assert result == [{1, 3}, {5, 5}, {7, :infinity}]
    end

    test "ranges are sorted by start position" do
      result = UnixTools.Cut.parse_ranges("5,1-3")
      assert result == [{1, 3}, {5, 5}]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Position checking
  # ---------------------------------------------------------------------------

  describe "position_included?/2" do
    test "position within closed range" do
      assert UnixTools.Cut.position_included?(3, [{1, 5}]) == true
    end

    test "position outside range" do
      assert UnixTools.Cut.position_included?(6, [{1, 3}]) == false
    end

    test "position in open-ended range" do
      assert UnixTools.Cut.position_included?(100, [{5, :infinity}]) == true
    end

    test "position before open-ended range" do
      assert UnixTools.Cut.position_included?(3, [{5, :infinity}]) == false
    end

    test "position at range boundary" do
      assert UnixTools.Cut.position_included?(5, [{5, 5}]) == true
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Byte cutting
  # ---------------------------------------------------------------------------

  describe "cut_bytes/3" do
    test "select first 3 bytes" do
      assert UnixTools.Cut.cut_bytes("abcdef", [{1, 3}]) == "abc"
    end

    test "select single byte" do
      assert UnixTools.Cut.cut_bytes("abcdef", [{4, 4}]) == "d"
    end

    test "select open-ended range" do
      assert UnixTools.Cut.cut_bytes("abcdef", [{3, :infinity}]) == "cdef"
    end

    test "select multiple ranges" do
      assert UnixTools.Cut.cut_bytes("abcdef", [{1, 2}, {5, 6}]) == "abef"
    end

    test "complement mode" do
      result = UnixTools.Cut.cut_bytes("abcdef", [{2, 4}], %{complement: true})
      assert result == "aef"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Character cutting
  # ---------------------------------------------------------------------------

  describe "cut_characters/3" do
    test "select character range" do
      assert UnixTools.Cut.cut_characters("abcdef", [{2, 4}]) == "bcd"
    end

    test "select single character" do
      assert UnixTools.Cut.cut_characters("hello", [{1, 1}]) == "h"
    end

    test "select from end" do
      assert UnixTools.Cut.cut_characters("hello", [{4, :infinity}]) == "lo"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Field cutting
  # ---------------------------------------------------------------------------

  describe "cut_fields/3" do
    test "select field by delimiter" do
      assert UnixTools.Cut.cut_fields("a:b:c:d", [{2, 2}], %{delimiter: ":"}) ==
               {:ok, "b"}
    end

    test "select multiple fields" do
      assert UnixTools.Cut.cut_fields("a:b:c:d", [{1, 1}, {3, 3}], %{delimiter: ":"}) ==
               {:ok, "a:c"}
    end

    test "select field range" do
      assert UnixTools.Cut.cut_fields("a:b:c:d", [{2, 3}], %{delimiter: ":"}) ==
               {:ok, "b:c"}
    end

    test "custom output delimiter" do
      result = UnixTools.Cut.cut_fields("a:b:c", [{1, 1}, {3, 3}],
        %{delimiter: ":", output_delimiter: ","})
      assert result == {:ok, "a,c"}
    end

    test "tab is default delimiter" do
      assert UnixTools.Cut.cut_fields("a\tb\tc", [{2, 2}], %{}) == {:ok, "b"}
    end

    test "line without delimiter is passed through" do
      assert UnixTools.Cut.cut_fields("nocolon", [{1, 1}], %{delimiter: ":"}) ==
               {:ok, "nocolon"}
    end

    test "only-delimited suppresses lines without delimiter" do
      result = UnixTools.Cut.cut_fields("nocolon", [{1, 1}],
        %{delimiter: ":", only_delimited: true})
      assert result == :suppress
    end

    test "complement selects non-matching fields" do
      result = UnixTools.Cut.cut_fields("a:b:c:d", [{2, 2}],
        %{delimiter: ":", complement: true})
      assert result == {:ok, "a:c:d"}
    end
  end
end
