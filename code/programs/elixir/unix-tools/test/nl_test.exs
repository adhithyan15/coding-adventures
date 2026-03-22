defmodule NlTest do
  @moduledoc """
  Tests for the nl tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Flag parsing (-b, -h, -f, -i, -n, -w, -s, -v, -d, -p).
  3. Business logic (should_number, format_number).
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "nl.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the nl spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "no arguments returns ParseResult" do
      assert {:ok, %ParseResult{}} = parse_argv(["nl"])
    end

    test "file arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["nl", "file.txt"])
      assert arguments["files"] == ["file.txt"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Flags
  # ---------------------------------------------------------------------------

  describe "flags" do
    test "-b sets body_numbering" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["nl", "-b", "a"])
      assert flags["body_numbering"] == "a"
    end

    test "-w sets number_width" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["nl", "-w", "3"])
      assert flags["number_width"] == 3
    end

    test "-s sets number_separator" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["nl", "-s", ". "])
      assert flags["number_separator"] == ". "
    end

    test "-v sets starting_line_number" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["nl", "-v", "10"])
      assert flags["starting_line_number"] == 10
    end

    test "-i sets line_increment" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["nl", "-i", "5"])
      assert flags["line_increment"] == 5
    end

    test "-p sets no_renumber" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["nl", "-p"])
      assert flags["no_renumber"] == true
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help and --version
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["nl", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["nl", "--help"])
      assert text =~ "nl"
    end
  end

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["nl", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["nl", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["nl", "--unknown"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - should_number
  # ---------------------------------------------------------------------------

  describe "should_number/2" do
    test "style 'a' numbers all lines" do
      assert UnixTools.Nl.should_number("", "a") == true
      assert UnixTools.Nl.should_number("hello", "a") == true
    end

    test "style 't' numbers non-empty lines" do
      assert UnixTools.Nl.should_number("hello", "t") == true
      assert UnixTools.Nl.should_number("", "t") == false
    end

    test "style 'n' numbers no lines" do
      assert UnixTools.Nl.should_number("hello", "n") == false
      assert UnixTools.Nl.should_number("", "n") == false
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - format_number
  # ---------------------------------------------------------------------------

  describe "format_number/3" do
    test "right-justified with spaces (rn)" do
      assert UnixTools.Nl.format_number(1, "rn", 6) == "     1"
    end

    test "right-justified with zeros (rz)" do
      assert UnixTools.Nl.format_number(1, "rz", 6) == "000001"
    end

    test "left-justified (ln)" do
      assert UnixTools.Nl.format_number(1, "ln", 6) == "1     "
    end

    test "multi-digit number" do
      assert UnixTools.Nl.format_number(42, "rn", 6) == "    42"
    end
  end
end
