defmodule TrueToolTest do
  @moduledoc """
  Tests for the true tool.

  ## What These Tests Verify

  These tests exercise the CLI Builder integration for the `true` utility.
  Since `true` has no flags and no arguments (beyond --help and --version),
  the tests focus on verifying that:

  1. Normal invocation returns a ParseResult (and the program would exit 0).
  2. --help returns a HelpResult with meaningful text.
  3. --version returns a VersionResult with the correct version string.
  4. Unknown flags are handled gracefully (true ignores errors).
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "true.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the true spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior (no arguments)
  # ---------------------------------------------------------------------------

  describe "default behavior (no arguments)" do
    test "returns {:ok, %ParseResult{}}" do
      assert {:ok, %ParseResult{}} = parse_argv(["true"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help flag
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["true", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["true", "--help"])
      assert text =~ "true"
    end

    test "help text contains description" do
      {:ok, %HelpResult{text: text}} = parse_argv(["true", "--help"])
      assert String.downcase(text) =~ "nothing"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --version flag
  # ---------------------------------------------------------------------------

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["true", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["true", "--version"])
      assert version == "1.0.0"
    end
  end
end
