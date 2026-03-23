defmodule GroupsTest do
  @moduledoc """
  Tests for the groups tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version).
  2. Group output parsing (both macOS and Linux formats).
  3. Live system group retrieval.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "groups.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: CLI parsing
  # ---------------------------------------------------------------------------

  describe "CLI parsing" do
    test "no arguments returns ParseResult" do
      assert {:ok, %ParseResult{}} = parse_argv(["groups"])
    end

    test "username argument is captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["groups", "alice"])
      assert arguments["users"] == ["alice"]
    end

    test "multiple usernames are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["groups", "alice", "bob"])
      assert arguments["users"] == ["alice", "bob"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help flag
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["groups", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["groups", "--help"])
      assert text =~ "groups"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --version flag
  # ---------------------------------------------------------------------------

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["groups", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["groups", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["groups", "--unknown"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: parse_groups_output/1
  # ---------------------------------------------------------------------------

  describe "parse_groups_output/1" do
    test "parses plain space-separated output" do
      result = UnixTools.Groups.parse_groups_output("staff everyone admin")
      assert result == ["staff", "everyone", "admin"]
    end

    test "parses macOS format with username prefix" do
      result = UnixTools.Groups.parse_groups_output("alice : staff everyone admin")
      assert result == ["staff", "everyone", "admin"]
    end

    test "parses single group" do
      result = UnixTools.Groups.parse_groups_output("staff")
      assert result == ["staff"]
    end

    test "handles trailing whitespace" do
      result = UnixTools.Groups.parse_groups_output("staff admin  ")
      assert result == ["staff", "admin"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: get_groups/0 (live system test)
  # ---------------------------------------------------------------------------

  describe "get_groups/0" do
    test "returns a non-empty list of strings" do
      groups = UnixTools.Groups.get_groups()
      assert is_list(groups)
      assert length(groups) > 0
      assert Enum.all?(groups, &is_binary/1)
    end

    test "at least one group is returned" do
      groups = UnixTools.Groups.get_groups()
      assert length(groups) >= 1
    end
  end

  # ---------------------------------------------------------------------------
  # Test: get_groups/1 (specific user, live system test)
  # ---------------------------------------------------------------------------

  describe "get_groups/1" do
    test "returns {:ok, groups} for current user" do
      current_user = System.get_env("USER")

      if current_user do
        result = UnixTools.Groups.get_groups(current_user)
        assert {:ok, groups} = result
        assert is_list(groups)
        assert length(groups) > 0
      end
    end

    test "returns {:error, _} for nonexistent user" do
      result = UnixTools.Groups.get_groups("nonexistent_user_xyz_12345")
      assert {:error, msg} = result
      assert msg =~ "no such user"
    end
  end
end
