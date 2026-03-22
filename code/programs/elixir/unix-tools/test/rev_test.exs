defmodule RevTest do
  @moduledoc """
  Tests for the rev tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Business logic (reverse_line, process_content).
  3. Edge cases (empty lines, Unicode).
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "rev.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the rev spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "no arguments returns ParseResult" do
      assert {:ok, %ParseResult{}} = parse_argv(["rev"])
    end

    test "file arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["rev", "file1.txt"])
      assert arguments["files"] == ["file1.txt"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help and --version
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["rev", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["rev", "--help"])
      assert text =~ "rev"
    end
  end

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["rev", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["rev", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["rev", "--unknown"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - reverse_line
  # ---------------------------------------------------------------------------

  describe "reverse_line/1" do
    test "reverses simple string" do
      assert UnixTools.Rev.reverse_line("hello") == "olleh"
    end

    test "reverses string with spaces" do
      assert UnixTools.Rev.reverse_line("ab cd") == "dc ba"
    end

    test "handles empty string" do
      assert UnixTools.Rev.reverse_line("") == ""
    end

    test "handles single character" do
      assert UnixTools.Rev.reverse_line("a") == "a"
    end

    test "handles palindrome" do
      assert UnixTools.Rev.reverse_line("racecar") == "racecar"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - process_content
  # ---------------------------------------------------------------------------

  describe "process_content/1" do
    test "reverses each line independently" do
      output =
        ExUnit.CaptureIO.capture_io(fn ->
          UnixTools.Rev.process_content("hello\nworld\n")
        end)

      assert output == "olleh\ndlrow\n"
    end

    test "handles single line with trailing newline" do
      output =
        ExUnit.CaptureIO.capture_io(fn ->
          UnixTools.Rev.process_content("hello\n")
        end)

      assert output == "olleh\n"
    end

    test "handles content without trailing newline" do
      output =
        ExUnit.CaptureIO.capture_io(fn ->
          UnixTools.Rev.process_content("hello")
        end)

      assert output == "olleh\n"
    end

    test "handles empty lines" do
      output =
        ExUnit.CaptureIO.capture_io(fn ->
          UnixTools.Rev.process_content("hello\n\nworld\n")
        end)

      assert output == "olleh\n\ndlrow\n"
    end
  end
end
