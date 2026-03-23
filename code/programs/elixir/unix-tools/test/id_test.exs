defmodule IdTest do
  @moduledoc """
  Tests for the id tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version).
  2. Parsing of `id` command output.
  3. Group list parsing.
  4. Full identity string formatting.
  5. Live system info retrieval.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "id.json"]) |> Path.expand()

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
      assert {:ok, %ParseResult{}} = parse_argv(["id"])
    end

    test "-u sets user flag" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["id", "-u"])
      assert flags["user"] == true
    end

    test "-g sets group flag" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["id", "-g"])
      assert flags["group"] == true
    end

    test "-G sets groups flag" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["id", "-G"])
      assert flags["groups"] == true
    end

    test "-n sets name flag" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["id", "-un"])
      assert flags["name"] == true
      assert flags["user"] == true
    end

    test "user argument is captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["id", "alice"])
      assert arguments["user_name"] == "alice"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help flag
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["id", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["id", "--help"])
      assert text =~ "id"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --version flag
  # ---------------------------------------------------------------------------

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["id", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["id", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["id", "--unknown"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: parse_id_output/1
  # ---------------------------------------------------------------------------

  describe "parse_id_output/1" do
    test "parses standard id output" do
      output = "uid=501(testuser) gid=20(staff) groups=20(staff),12(everyone)"
      result = UnixTools.IdTool.parse_id_output(output)

      assert result.uid == 501
      assert result.uid_name == "testuser"
      assert result.gid == 20
      assert result.gid_name == "staff"
      assert result.group_list == [{20, "staff"}, {12, "everyone"}]
    end

    test "parses output with many groups" do
      output = "uid=0(root) gid=0(root) groups=0(root),1(bin),2(daemon)"
      result = UnixTools.IdTool.parse_id_output(output)

      assert result.uid == 0
      assert result.uid_name == "root"
      assert length(result.group_list) == 3
    end

    test "handles single group" do
      output = "uid=1000(user) gid=1000(user) groups=1000(user)"
      result = UnixTools.IdTool.parse_id_output(output)

      assert result.group_list == [{1000, "user"}]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: parse_group_list/1
  # ---------------------------------------------------------------------------

  describe "parse_group_list/1" do
    test "parses comma-separated groups" do
      result = UnixTools.IdTool.parse_group_list("20(staff),12(everyone)")
      assert result == [{20, "staff"}, {12, "everyone"}]
    end

    test "parses single group" do
      result = UnixTools.IdTool.parse_group_list("0(root)")
      assert result == [{0, "root"}]
    end

    test "parses groups with hyphens in names" do
      result = UnixTools.IdTool.parse_group_list("100(my-group)")
      assert result == [{100, "my-group"}]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: format_full/1
  # ---------------------------------------------------------------------------

  describe "format_full/1" do
    test "formats identity string" do
      info = %{
        uid: 501,
        uid_name: "testuser",
        gid: 20,
        gid_name: "staff",
        group_list: [{20, "staff"}, {12, "everyone"}]
      }

      result = UnixTools.IdTool.format_full(info)
      assert result == "uid=501(testuser) gid=20(staff) groups=20(staff),12(everyone)"
    end

    test "formats root identity" do
      info = %{
        uid: 0,
        uid_name: "root",
        gid: 0,
        gid_name: "root",
        group_list: [{0, "root"}]
      }

      result = UnixTools.IdTool.format_full(info)
      assert result == "uid=0(root) gid=0(root) groups=0(root)"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: get_user_info/0 (live system test)
  # ---------------------------------------------------------------------------

  describe "get_user_info/0" do
    test "returns info with valid uid" do
      info = UnixTools.IdTool.get_user_info()
      assert is_integer(info.uid)
      assert info.uid >= 0
    end

    test "returns info with non-empty username" do
      info = UnixTools.IdTool.get_user_info()
      assert is_binary(info.uid_name)
      assert String.length(info.uid_name) > 0
    end

    test "returns non-empty group list" do
      info = UnixTools.IdTool.get_user_info()
      assert is_list(info.group_list)
      assert length(info.group_list) > 0
    end
  end
end
