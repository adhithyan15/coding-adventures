defmodule RmdirTest do
  @moduledoc """
  Tests for the rmdir tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Flag parsing (-p for parents, -v for verbose, --ignore-fail-on-non-empty).
  3. Business logic (get_parent_chain).
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "rmdir.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the rmdir spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "directory arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["rmdir", "testdir"])
      assert arguments["directories"] == ["testdir"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Flags
  # ---------------------------------------------------------------------------

  describe "flags" do
    test "-p sets parents to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["rmdir", "-p", "testdir"])
      assert flags["parents"] == true
    end

    test "-v sets verbose to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["rmdir", "-v", "testdir"])
      assert flags["verbose"] == true
    end

    test "--ignore-fail-on-non-empty works" do
      {:ok, %ParseResult{flags: flags}} =
        parse_argv(["rmdir", "--ignore-fail-on-non-empty", "testdir"])

      assert flags["ignore_fail"] == true
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help and --version
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["rmdir", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["rmdir", "--help"])
      assert text =~ "rmdir"
    end
  end

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["rmdir", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["rmdir", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["rmdir", "--unknown"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - get_parent_chain
  # ---------------------------------------------------------------------------

  describe "get_parent_chain/1" do
    test "returns chain for nested path" do
      chain = UnixTools.Rmdir.get_parent_chain("a/b/c")
      assert chain == ["a/b/c", "a/b", "a"]
    end

    test "returns single element for top-level directory" do
      chain = UnixTools.Rmdir.get_parent_chain("mydir")
      assert chain == ["mydir"]
    end

    test "returns chain for two-level path" do
      chain = UnixTools.Rmdir.get_parent_chain("a/b")
      assert chain == ["a/b", "a"]
    end
  end
end
