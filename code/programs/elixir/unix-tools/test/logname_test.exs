defmodule LognameTest do
  @moduledoc """
  Tests for the logname tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Business logic (get_login_name/0 reads $LOGNAME and $USER).
  3. Fallback behavior (uses $USER when $LOGNAME is not set).
  4. Error case (neither variable is set).
  """

  use ExUnit.Case, async: false

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "logname.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the logname spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "no arguments returns ParseResult" do
      assert {:ok, %ParseResult{}} = parse_argv(["logname"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help flag
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["logname", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["logname", "--help"])
      assert text =~ "logname"
    end

    test "help text contains description" do
      {:ok, %HelpResult{text: text}} = parse_argv(["logname", "--help"])
      assert String.downcase(text) =~ "login"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --version flag
  # ---------------------------------------------------------------------------

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["logname", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["logname", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["logname", "--unknown"])
    end

    test "unknown short flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["logname", "-x"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic
  # ---------------------------------------------------------------------------

  describe "get_login_name/0" do
    test "returns $LOGNAME when it is set" do
      old_logname = System.get_env("LOGNAME")
      old_user = System.get_env("USER")

      try do
        System.put_env("LOGNAME", "loginuser")
        System.put_env("USER", "effectiveuser")
        assert {:ok, "loginuser"} = UnixTools.Logname.get_login_name()
      after
        if old_logname, do: System.put_env("LOGNAME", old_logname), else: System.delete_env("LOGNAME")
        if old_user, do: System.put_env("USER", old_user), else: System.delete_env("USER")
      end
    end

    test "falls back to $USER when $LOGNAME is not set" do
      old_logname = System.get_env("LOGNAME")
      old_user = System.get_env("USER")

      try do
        System.delete_env("LOGNAME")
        System.put_env("USER", "fallbackuser")
        assert {:ok, "fallbackuser"} = UnixTools.Logname.get_login_name()
      after
        if old_logname, do: System.put_env("LOGNAME", old_logname), else: System.delete_env("LOGNAME")
        if old_user, do: System.put_env("USER", old_user), else: System.delete_env("USER")
      end
    end

    test "returns :error when neither $LOGNAME nor $USER is set" do
      old_logname = System.get_env("LOGNAME")
      old_user = System.get_env("USER")

      try do
        System.delete_env("LOGNAME")
        System.delete_env("USER")
        assert :error = UnixTools.Logname.get_login_name()
      after
        if old_logname, do: System.put_env("LOGNAME", old_logname), else: System.delete_env("LOGNAME")
        if old_user, do: System.put_env("USER", old_user), else: System.delete_env("USER")
      end
    end

    test "prefers $LOGNAME over $USER" do
      old_logname = System.get_env("LOGNAME")
      old_user = System.get_env("USER")

      try do
        System.put_env("LOGNAME", "login_name")
        System.put_env("USER", "user_name")
        {:ok, result} = UnixTools.Logname.get_login_name()
        assert result == "login_name"
      after
        if old_logname, do: System.put_env("LOGNAME", old_logname), else: System.delete_env("LOGNAME")
        if old_user, do: System.put_env("USER", old_user), else: System.delete_env("USER")
      end
    end
  end
end
