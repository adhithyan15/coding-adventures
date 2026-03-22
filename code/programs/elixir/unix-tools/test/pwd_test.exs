defmodule PwdTest do
  @moduledoc """
  Tests for the pwd tool.

  ## What These Tests Verify

  These tests exercise the full CLI Builder integration. We construct a
  `Parser.parse/2` call with our `pwd.json` spec and various argv values,
  then verify that the parser returns the correct result type and that the
  business logic produces the expected output.

  ## Why We Test Through CLI Builder

  The point of CLI Builder is that developers don't write parsing code.
  So our tests verify the *integration*: does our JSON spec, combined with
  CLI Builder's parser, produce the right behavior? This catches spec
  errors (wrong flag names, missing fields) as well as logic errors.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------
  # The spec file lives at the project root, one level above the test/ directory.

  @spec_path Path.join([__DIR__, "..", "pwd.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the pwd spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior (no flags) returns ParseResult
  # ---------------------------------------------------------------------------

  # When invoked with no flags, pwd should return a `ParseResult`
  # with both `physical` and `logical` set to `false` (the defaults).
  describe "default behavior (no flags)" do

    test "returns {:ok, %ParseResult{}}" do
      assert {:ok, %ParseResult{}} = parse_argv(["pwd"])
    end

    test "physical flag defaults to false" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["pwd"])
      refute flags["physical"]
    end

    test "logical flag defaults to false" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["pwd"])
      refute flags["logical"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: -P flag (physical mode)
  # ---------------------------------------------------------------------------

  # The `-P` flag should set `physical` to `true`.
  describe "-P flag (physical mode)" do

    test "short flag -P sets physical to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["pwd", "-P"])
      assert flags["physical"] == true
    end

    test "long flag --physical sets physical to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["pwd", "--physical"])
      assert flags["physical"] == true
    end
  end

  # ---------------------------------------------------------------------------
  # Test: -L flag (logical mode)
  # ---------------------------------------------------------------------------

  # The `-L` flag should set `logical` to `true`.
  describe "-L flag (logical mode)" do

    test "short flag -L sets logical to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["pwd", "-L"])
      assert flags["logical"] == true
    end

    test "long flag --logical sets logical to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["pwd", "--logical"])
      assert flags["logical"] == true
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help flag
  # ---------------------------------------------------------------------------

  # `--help` should return a `HelpResult` with non-empty text that includes
  # the program name and description.
  describe "--help flag" do

    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["pwd", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["pwd", "--help"])
      assert text =~ "pwd"
    end

    test "help text contains description keywords" do
      {:ok, %HelpResult{text: text}} = parse_argv(["pwd", "--help"])
      assert String.downcase(text) =~ "working directory"
    end

    test "-h also returns help" do
      assert {:ok, %HelpResult{}} = parse_argv(["pwd", "-h"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --version flag
  # ---------------------------------------------------------------------------

  # `--version` should return a `VersionResult` with "1.0.0".
  describe "--version flag" do

    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["pwd", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["pwd", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags produce errors
  # ---------------------------------------------------------------------------

  # Unknown flags should return `{:error, %ParseErrors{}}`.
  describe "unknown flags" do

    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["pwd", "--unknown"])
    end

    test "unknown short flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["pwd", "-x"])
    end

    test "error contains meaningful message" do
      {:error, %ParseErrors{errors: errors}} = parse_argv(["pwd", "--unknown"])
      assert length(errors) > 0
      assert hd(errors).message =~ "unknown"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic functions
  # ---------------------------------------------------------------------------

  # Test the pwd business logic functions directly, independent of
  # CLI Builder parsing.
  describe "business logic" do

    test "get_physical_pwd returns an absolute path string" do
      result = UnixTools.Pwd.get_physical_pwd()
      assert is_binary(result)
      assert String.starts_with?(result, "/")
    end

    test "get_logical_pwd returns an absolute path string" do
      result = UnixTools.Pwd.get_logical_pwd()
      assert is_binary(result)
      assert String.starts_with?(result, "/")
    end

    test "get_logical_pwd uses $PWD when it matches the real cwd" do
      physical = UnixTools.Pwd.get_physical_pwd()

      # Set $PWD to the physical path — should return it as-is.
      old_pwd = System.get_env("PWD")

      try do
        System.put_env("PWD", physical)
        result = UnixTools.Pwd.get_logical_pwd()
        assert result == physical
      after
        if old_pwd, do: System.put_env("PWD", old_pwd), else: System.delete_env("PWD")
      end
    end

    test "get_logical_pwd falls back when $PWD is invalid" do
      old_pwd = System.get_env("PWD")

      try do
        System.put_env("PWD", "/nonexistent/path/that/does/not/exist")
        result = UnixTools.Pwd.get_logical_pwd()
        # Should fall back to physical path — still a valid absolute path
        assert is_binary(result)
        assert String.starts_with?(result, "/")
      after
        if old_pwd, do: System.put_env("PWD", old_pwd), else: System.delete_env("PWD")
      end
    end

    test "get_logical_pwd falls back when $PWD is unset" do
      old_pwd = System.get_env("PWD")

      try do
        System.delete_env("PWD")
        result = UnixTools.Pwd.get_logical_pwd()
        assert is_binary(result)
        assert String.starts_with?(result, "/")
      after
        if old_pwd, do: System.put_env("PWD", old_pwd), else: System.delete_env("PWD")
      end
    end
  end
end
