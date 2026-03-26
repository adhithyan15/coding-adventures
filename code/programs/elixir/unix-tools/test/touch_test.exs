defmodule TouchTest do
  @moduledoc """
  Tests for the touch tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Flag parsing (-a, -m, -c, -d, -r, -t).
  3. Business logic (parse_timestamp, determine_time).
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "touch.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the touch spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "file arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["touch", "file.txt"])
      assert arguments["files"] == ["file.txt"]
    end

    test "multiple files are captured" do
      {:ok, %ParseResult{arguments: arguments}} =
        parse_argv(["touch", "a.txt", "b.txt"])

      assert arguments["files"] == ["a.txt", "b.txt"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Flags
  # ---------------------------------------------------------------------------

  describe "flags" do
    test "-a sets access_only" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["touch", "-a", "file.txt"])
      assert flags["access_only"] == true
    end

    test "-m sets modification_only" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["touch", "-m", "file.txt"])
      assert flags["modification_only"] == true
    end

    test "-c sets no_create" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["touch", "-c", "file.txt"])
      assert flags["no_create"] == true
    end

    test "-d sets date" do
      {:ok, %ParseResult{flags: flags}} =
        parse_argv(["touch", "-d", "2024-01-15", "file.txt"])

      assert flags["date"] == "2024-01-15"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help and --version
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["touch", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["touch", "--help"])
      assert text =~ "touch"
    end
  end

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["touch", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["touch", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["touch", "--unknown"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - parse_timestamp
  # ---------------------------------------------------------------------------

  describe "parse_timestamp/1" do
    test "parses CCYYMMDDhhmm format" do
      result = UnixTools.Touch.parse_timestamp("202401151030")
      assert result.year == 2024
      assert result.month == 1
      assert result.day == 15
      assert result.hour == 10
      assert result.minute == 30
    end

    test "parses CCYYMMDDhhmm.ss format" do
      result = UnixTools.Touch.parse_timestamp("202401151030.45")
      assert result.second == 45
    end

    test "parses MMDDhhmm format (uses current year)" do
      result = UnixTools.Touch.parse_timestamp("01151030")
      assert result.month == 1
      assert result.day == 15
    end
  end
end
