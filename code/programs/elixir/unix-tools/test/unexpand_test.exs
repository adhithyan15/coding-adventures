defmodule UnexpandTest do
  @moduledoc """
  Tests for the unexpand tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Flag parsing (-a for all, --first-only, -t for tab stops).
  3. Business logic (unexpand_line).
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "unexpand.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the unexpand spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "no arguments returns ParseResult" do
      assert {:ok, %ParseResult{}} = parse_argv(["unexpand"])
    end

    test "file arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["unexpand", "file.txt"])
      assert arguments["files"] == ["file.txt"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Flags
  # ---------------------------------------------------------------------------

  describe "flags" do
    test "-a sets all to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["unexpand", "-a", "file.txt"])
      assert flags["all"] == true
    end

    test "-t sets tab stops" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["unexpand", "-t", "4", "file.txt"])
      assert flags["tabs"] == "4"
    end

    test "--first-only flag works" do
      {:ok, %ParseResult{flags: flags}} =
        parse_argv(["unexpand", "--first-only", "file.txt"])

      assert flags["first_only"] == true
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help and --version
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["unexpand", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["unexpand", "--help"])
      assert text =~ "unexpand"
    end
  end

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["unexpand", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["unexpand", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["unexpand", "--unknown"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - unexpand_line
  # ---------------------------------------------------------------------------

  describe "unexpand_line/3" do
    test "converts leading spaces to tabs" do
      result = UnixTools.Unexpand.unexpand_line("        hello", 8, false)
      assert result == "\thello"
    end

    test "converts leading spaces with custom tab size" do
      result = UnixTools.Unexpand.unexpand_line("    hello", 4, false)
      assert result == "\thello"
    end

    test "handles line with no leading spaces" do
      result = UnixTools.Unexpand.unexpand_line("hello", 8, false)
      assert result == "hello"
    end

    test "preserves spaces that don't reach a tab stop" do
      result = UnixTools.Unexpand.unexpand_line("   hello", 8, false)
      assert result == "   hello"
    end
  end
end
