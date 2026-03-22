defmodule LnTest do
  @moduledoc """
  Tests for the ln tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Flag parsing (-s for symbolic, -f for force, -v for verbose).
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "ln.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the ln spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "target argument is captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["ln", "target.txt"])
      assert arguments["targets"] == ["target.txt"]
    end

    test "target and link name are captured" do
      {:ok, %ParseResult{arguments: arguments}} =
        parse_argv(["ln", "target.txt", "link.txt"])

      assert arguments["targets"] == ["target.txt", "link.txt"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Flags
  # ---------------------------------------------------------------------------

  describe "flags" do
    test "-s sets symbolic to true" do
      {:ok, %ParseResult{flags: flags}} =
        parse_argv(["ln", "-s", "target.txt", "link.txt"])

      assert flags["symbolic"] == true
    end

    test "-f sets force to true" do
      {:ok, %ParseResult{flags: flags}} =
        parse_argv(["ln", "-f", "target.txt", "link.txt"])

      assert flags["force"] == true
    end

    test "-v sets verbose to true" do
      {:ok, %ParseResult{flags: flags}} =
        parse_argv(["ln", "-v", "target.txt", "link.txt"])

      assert flags["verbose"] == true
    end

    test "--symbolic long flag works" do
      {:ok, %ParseResult{flags: flags}} =
        parse_argv(["ln", "--symbolic", "target.txt"])

      assert flags["symbolic"] == true
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help and --version
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["ln", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["ln", "--help"])
      assert text =~ "ln"
    end
  end

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["ln", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["ln", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["ln", "--unknown"])
    end
  end
end
