defmodule PrintenvTest do
  @moduledoc """
  Tests for the printenv tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Flag parsing (-0 for NUL termination).
  3. Variable argument handling.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "printenv.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the printenv spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "no arguments returns ParseResult" do
      assert {:ok, %ParseResult{}} = parse_argv(["printenv"])
    end

    test "variable arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["printenv", "HOME", "PATH"])
      assert arguments["variables"] == ["HOME", "PATH"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Flags
  # ---------------------------------------------------------------------------

  describe "flags" do
    test "-0 sets null to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["printenv", "-0"])
      assert flags["null"] == true
    end

    test "--null long flag works" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["printenv", "--null"])
      assert flags["null"] == true
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help and --version
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["printenv", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["printenv", "--help"])
      assert text =~ "printenv"
    end
  end

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["printenv", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["printenv", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["printenv", "--unknown"])
    end
  end
end
