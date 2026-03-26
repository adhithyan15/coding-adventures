defmodule TeeTest do
  @moduledoc """
  Tests for the tee tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Flag parsing (-a for append, -i for ignore interrupts).
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "tee.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the tee spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "no arguments returns ParseResult" do
      assert {:ok, %ParseResult{}} = parse_argv(["tee"])
    end

    test "file arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} =
        parse_argv(["tee", "file1.txt", "file2.txt"])

      assert arguments["files"] == ["file1.txt", "file2.txt"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Flags
  # ---------------------------------------------------------------------------

  describe "flags" do
    test "-a sets append to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["tee", "-a", "file.txt"])
      assert flags["append"] == true
    end

    test "-i sets ignore_interrupts to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["tee", "-i", "file.txt"])
      assert flags["ignore_interrupts"] == true
    end

    test "--append long flag works" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["tee", "--append", "file.txt"])
      assert flags["append"] == true
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help and --version
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["tee", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["tee", "--help"])
      assert text =~ "tee"
    end
  end

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["tee", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["tee", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["tee", "--unknown"])
    end
  end
end
