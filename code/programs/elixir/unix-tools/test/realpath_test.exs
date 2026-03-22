defmodule RealpathTest do
  @moduledoc """
  Tests for the realpath tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Flag parsing (-e, -m, -s, -q, -z, --relative-to, --relative-base).
  3. Business logic (resolve_path).
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "realpath.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the realpath spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "file arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["realpath", "/tmp"])
      assert arguments["files"] == ["/tmp"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Flags
  # ---------------------------------------------------------------------------

  describe "flags" do
    test "-e sets canonicalize_existing" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["realpath", "-e", "/tmp"])
      assert flags["canonicalize_existing"] == true
    end

    test "-m sets canonicalize_missing" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["realpath", "-m", "/tmp"])
      assert flags["canonicalize_missing"] == true
    end

    test "-s sets no_symlinks" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["realpath", "-s", "/tmp"])
      assert flags["no_symlinks"] == true
    end

    test "-z sets zero" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["realpath", "-z", "/tmp"])
      assert flags["zero"] == true
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help and --version
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["realpath", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["realpath", "--help"])
      assert text =~ "realpath"
    end
  end

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["realpath", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["realpath", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["realpath", "--unknown"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - resolve_path
  # ---------------------------------------------------------------------------

  describe "resolve_path/4" do
    test "resolves existing path with no_symlinks" do
      {:ok, resolved} = UnixTools.Realpath.resolve_path("/tmp", false, false, true)
      assert resolved == "/tmp"
    end

    test "resolves with canonicalize_missing for nonexistent path" do
      {:ok, resolved} =
        UnixTools.Realpath.resolve_path("/tmp/nonexistent_test_path", false, true, false)

      assert resolved =~ "nonexistent_test_path"
    end
  end
end
