defmodule TrTest do
  @moduledoc """
  Tests for the tr tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Flag parsing (-c, -d, -s, -t).
  3. Business logic (expand_set, translate_chars, delete_chars, squeeze_chars).
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "tr.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the tr spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "set1 is captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["tr", "abc"])
      assert arguments["set1"] == "abc"
    end

    test "set1 and set2 are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["tr", "abc", "xyz"])
      assert arguments["set1"] == "abc"
      assert arguments["set2"] == "xyz"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Flags
  # ---------------------------------------------------------------------------

  describe "flags" do
    test "-d sets delete to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["tr", "-d", "abc"])
      assert flags["delete"] == true
    end

    test "-s sets squeeze_repeats to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["tr", "-s", "abc"])
      assert flags["squeeze_repeats"] == true
    end

    test "-c sets complement to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["tr", "-c", "abc", "x"])
      assert flags["complement"] == true
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help and --version
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["tr", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["tr", "--help"])
      assert text =~ "tr"
    end
  end

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["tr", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["tr", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - expand_set
  # ---------------------------------------------------------------------------

  describe "expand_set/1" do
    test "expands literal characters" do
      assert UnixTools.Tr.expand_set("abc") == ["a", "b", "c"]
    end

    test "expands character range" do
      result = UnixTools.Tr.expand_set("a-e")
      assert result == ["a", "b", "c", "d", "e"]
    end

    test "expands digit range" do
      result = UnixTools.Tr.expand_set("0-9")
      assert length(result) == 10
      assert hd(result) == "0"
      assert List.last(result) == "9"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - translate_chars
  # ---------------------------------------------------------------------------

  describe "translate_chars/4" do
    test "translates simple characters" do
      result = UnixTools.Tr.translate_chars("hello", ["l"], ["r"], false)
      assert result == "herro"
    end

    test "translates with squeeze" do
      result = UnixTools.Tr.translate_chars("aabbcc", ["a", "b", "c"], ["x", "y", "z"], true)
      assert result == "xyz"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - delete_chars
  # ---------------------------------------------------------------------------

  describe "delete_chars/4" do
    test "deletes specified characters" do
      result = UnixTools.Tr.delete_chars("hello", ["l"], false, [])
      assert result == "heo"
    end

    test "deletes multiple characters" do
      result = UnixTools.Tr.delete_chars("hello world", ["l", "o"], false, [])
      assert result == "he wrd"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - squeeze_chars
  # ---------------------------------------------------------------------------

  describe "squeeze_chars/2" do
    test "squeezes repeated characters" do
      result = UnixTools.Tr.squeeze_chars("aabbcc", ["a", "b", "c"])
      assert result == "abc"
    end

    test "only squeezes characters in set" do
      result = UnixTools.Tr.squeeze_chars("aabbcc", ["a"])
      assert result == "abbcc"
    end

    test "handles no repeats" do
      result = UnixTools.Tr.squeeze_chars("abc", ["a", "b", "c"])
      assert result == "abc"
    end
  end
end
