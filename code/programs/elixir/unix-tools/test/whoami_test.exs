defmodule WhoamiTest do
  @moduledoc """
  Tests for the whoami tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Business logic (get_username/0 reads $USER correctly).
  3. Edge cases (missing $USER environment variable).
  """

  use ExUnit.Case, async: false

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "whoami.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the whoami spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "no arguments returns ParseResult" do
      assert {:ok, %ParseResult{}} = parse_argv(["whoami"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help flag
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["whoami", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["whoami", "--help"])
      assert text =~ "whoami"
    end

    test "help text contains description" do
      {:ok, %HelpResult{text: text}} = parse_argv(["whoami", "--help"])
      assert String.downcase(text) =~ "user"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --version flag
  # ---------------------------------------------------------------------------

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["whoami", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["whoami", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["whoami", "--unknown"])
    end

    test "unknown short flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["whoami", "-x"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic
  # ---------------------------------------------------------------------------

  describe "get_username/0" do
    test "returns {:ok, username} when $USER is set" do
      old_user = System.get_env("USER")

      try do
        System.put_env("USER", "testuser")
        assert {:ok, "testuser"} = UnixTools.Whoami.get_username()
      after
        if old_user, do: System.put_env("USER", old_user), else: System.delete_env("USER")
      end
    end

    test "returns :error when $USER is not set" do
      old_user = System.get_env("USER")

      try do
        System.delete_env("USER")
        assert :error = UnixTools.Whoami.get_username()
      after
        if old_user, do: System.put_env("USER", old_user), else: System.delete_env("USER")
      end
    end

    test "returns the current user name from the environment" do
      # This test verifies that get_username reads the actual $USER variable.
      expected = System.get_env("USER")

      if expected do
        assert {:ok, ^expected} = UnixTools.Whoami.get_username()
      end
    end
  end
end
