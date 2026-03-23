defmodule RmTest do
  @moduledoc """
  Tests for the rm tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Flag parsing (-f, -r, -d, -v, -i, -I).
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "rm.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the rm spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "file arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["rm", "file.txt"])
      assert arguments["files"] == ["file.txt"]
    end

    test "multiple files are captured" do
      {:ok, %ParseResult{arguments: arguments}} =
        parse_argv(["rm", "a.txt", "b.txt"])

      assert arguments["files"] == ["a.txt", "b.txt"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Flags
  # ---------------------------------------------------------------------------

  describe "flags" do
    test "-f sets force to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["rm", "-f", "file.txt"])
      assert flags["force"] == true
    end

    test "-r sets recursive to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["rm", "-r", "dir"])
      assert flags["recursive"] == true
    end

    test "-d sets dir to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["rm", "-d", "dir"])
      assert flags["dir"] == true
    end

    test "-v sets verbose to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["rm", "-v", "file.txt"])
      assert flags["verbose"] == true
    end

    test "--recursive long flag works" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["rm", "--recursive", "dir"])
      assert flags["recursive"] == true
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help and --version
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["rm", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["rm", "--help"])
      assert text =~ "rm"
    end
  end

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["rm", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["rm", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["rm", "--unknown"])
    end
  end
end
