defmodule TailTest do
  @moduledoc """
  Tests for the tail tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Business logic (tail_lines, tail_bytes, parse_count).
  3. The +NUM syntax for starting from a specific line/byte.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "tail.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the tail spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "no arguments returns ParseResult" do
      assert {:ok, %ParseResult{}} = parse_argv(["tail"])
    end

    test "file arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["tail", "file1.txt"])
      assert arguments["files"] == ["file1.txt"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Flags
  # ---------------------------------------------------------------------------

  describe "flags" do
    test "-n sets lines value" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["tail", "-n", "5"])
      assert flags["lines"] == "5"
    end

    test "-f sets follow to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["tail", "-f"])
      assert flags["follow"] == true
    end

    test "-q sets quiet to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["tail", "-q"])
      assert flags["quiet"] == true
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help and --version
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["tail", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["tail", "--help"])
      assert text =~ "tail"
    end
  end

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["tail", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["tail", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["tail", "--unknown"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - parse_count
  # ---------------------------------------------------------------------------

  describe "parse_count/1" do
    test "plain number returns {value, false}" do
      assert UnixTools.Tail.parse_count("10") == {10, false}
    end

    test "minus prefix returns {value, false}" do
      assert UnixTools.Tail.parse_count("-5") == {5, false}
    end

    test "plus prefix returns {value, true}" do
      assert UnixTools.Tail.parse_count("+3") == {3, true}
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - tail_lines
  # ---------------------------------------------------------------------------

  describe "tail_lines/4" do
    test "extracts last N lines" do
      content = "line1\nline2\nline3\nline4\nline5\n"
      result = UnixTools.Tail.tail_lines(content, 3, false, "\n")
      assert result == "line3\nline4\nline5\n"
    end

    test "extracts all lines when count exceeds total" do
      content = "line1\nline2\n"
      result = UnixTools.Tail.tail_lines(content, 10, false, "\n")
      assert result == "line1\nline2\n"
    end

    test "+N syntax starts from line N" do
      content = "line1\nline2\nline3\nline4\nline5\n"
      result = UnixTools.Tail.tail_lines(content, 3, true, "\n")
      assert result == "line3\nline4\nline5\n"
    end

    test "+1 returns all lines" do
      content = "line1\nline2\nline3\n"
      result = UnixTools.Tail.tail_lines(content, 1, true, "\n")
      assert result == "line1\nline2\nline3\n"
    end

    test "handles empty content" do
      result = UnixTools.Tail.tail_lines("", 5, false, "\n")
      assert result == ""
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - tail_bytes
  # ---------------------------------------------------------------------------

  describe "tail_bytes/3" do
    test "extracts last N bytes" do
      content = "hello world"
      result = UnixTools.Tail.tail_bytes(content, 5, false)
      assert result == "world"
    end

    test "+N syntax starts from byte N" do
      content = "hello world"
      result = UnixTools.Tail.tail_bytes(content, 7, true)
      assert result == "world"
    end

    test "returns all content when count exceeds size" do
      content = "hi"
      result = UnixTools.Tail.tail_bytes(content, 100, false)
      assert result == "hi"
    end
  end
end
