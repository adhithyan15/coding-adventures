defmodule MkdirTest do
  @moduledoc """
  Tests for the mkdir tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Flag parsing (-p for parents, -v for verbose, -m for mode).
  3. Business logic (parse_mode, create_directory).
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "mkdir.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the mkdir spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "directory arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["mkdir", "testdir"])
      assert arguments["directories"] == ["testdir"]
    end

    test "multiple directories are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["mkdir", "dir1", "dir2"])
      assert arguments["directories"] == ["dir1", "dir2"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Flags
  # ---------------------------------------------------------------------------

  describe "flags" do
    test "-p sets parents to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["mkdir", "-p", "testdir"])
      assert flags["parents"] == true
    end

    test "-v sets verbose to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["mkdir", "-v", "testdir"])
      assert flags["verbose"] == true
    end

    test "-m sets mode" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["mkdir", "-m", "755", "testdir"])
      assert flags["mode"] == "755"
    end

    test "--parents long flag works" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["mkdir", "--parents", "testdir"])
      assert flags["parents"] == true
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help and --version
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["mkdir", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["mkdir", "--help"])
      assert text =~ "mkdir"
    end
  end

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["mkdir", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["mkdir", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["mkdir", "--unknown"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - parse_mode
  # ---------------------------------------------------------------------------

  describe "parse_mode/1" do
    test "parses valid octal mode" do
      assert UnixTools.Mkdir.parse_mode("755") == {:ok, 0o755}
    end

    test "parses 644" do
      assert UnixTools.Mkdir.parse_mode("644") == {:ok, 0o644}
    end

    test "rejects invalid mode" do
      assert {:error, _} = UnixTools.Mkdir.parse_mode("xyz")
    end
  end
end
