defmodule HeadTest do
  @moduledoc """
  Tests for the head tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Business logic (head_lines, head_bytes).
  3. Edge cases (empty content, fewer lines than requested).
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "head.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the head spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "no arguments returns ParseResult" do
      assert {:ok, %ParseResult{}} = parse_argv(["head"])
    end

    test "file arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["head", "file1.txt", "file2.txt"])
      assert arguments["files"] == ["file1.txt", "file2.txt"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Flags
  # ---------------------------------------------------------------------------

  describe "flags" do
    test "-n sets lines count" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["head", "-n", "5"])
      assert flags["lines"] == 5
    end

    test "-c sets bytes count" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["head", "-c", "100"])
      assert flags["bytes"] == 100
    end

    test "-q sets quiet to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["head", "-q"])
      assert flags["quiet"] == true
    end

    test "-v sets verbose to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["head", "-v"])
      assert flags["verbose"] == true
    end

    test "-z sets zero_terminated to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["head", "-z"])
      assert flags["zero_terminated"] == true
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help flag
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["head", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["head", "--help"])
      assert text =~ "head"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --version flag
  # ---------------------------------------------------------------------------

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["head", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["head", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags produce errors
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["head", "--unknown"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - head_lines
  # ---------------------------------------------------------------------------

  describe "head_lines/3" do
    test "extracts first N lines" do
      content = "line1\nline2\nline3\nline4\nline5\n"
      result = UnixTools.Head.head_lines(content, 3, "\n")
      assert result == "line1\nline2\nline3\n"
    end

    test "returns all lines when count exceeds total" do
      content = "line1\nline2\n"
      result = UnixTools.Head.head_lines(content, 10, "\n")
      assert result == "line1\nline2\n"
    end

    test "handles empty content" do
      result = UnixTools.Head.head_lines("", 5, "\n")
      assert result == ""
    end

    test "handles single line without trailing newline" do
      content = "hello"
      result = UnixTools.Head.head_lines(content, 1, "\n")
      assert result == "hello\n"
    end

    test "respects NUL delimiter" do
      content = "a\0b\0c\0"
      result = UnixTools.Head.head_lines(content, 2, <<0>>)
      assert result == "a\0b\0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - head_bytes
  # ---------------------------------------------------------------------------

  describe "head_bytes/2" do
    test "extracts first N bytes" do
      content = "hello world"
      result = UnixTools.Head.head_bytes(content, 5)
      assert result == "hello"
    end

    test "returns all content when count exceeds size" do
      content = "hi"
      result = UnixTools.Head.head_bytes(content, 100)
      assert result == "hi"
    end

    test "handles empty content" do
      result = UnixTools.Head.head_bytes("", 5)
      assert result == ""
    end
  end
end
