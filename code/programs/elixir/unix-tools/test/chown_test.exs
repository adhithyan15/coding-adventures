defmodule ChownTest do
  @moduledoc """
  Tests for the chown tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version).
  2. Owner:group parsing (various formats).
  3. Chown argument building (flags, ownership spec, files).
  4. File info retrieval.

  Note: Actual chown operations require root privileges, so we test
  the parsing and argument building logic rather than executing chown
  as a non-root user.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "chown.json"]) |> Path.expand()

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: CLI parsing
  # ---------------------------------------------------------------------------

  describe "CLI parsing" do
    test "owner and file arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["chown", "alice", "file.txt"])
      assert arguments["owner_group"] == "alice"
      # Variadic args always return as list
      assert arguments["files"] == ["file.txt"]
    end

    test "owner:group and file" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["chown", "alice:staff", "file.txt"])
      assert arguments["owner_group"] == "alice:staff"
    end

    test "multiple files" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["chown", "alice", "f1", "f2"])
      assert arguments["files"] == ["f1", "f2"]
    end

    test "-R sets recursive" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["chown", "-R", "alice", "dir"])
      assert flags["recursive"] == true
    end

    test "-v sets verbose" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["chown", "-v", "alice", "file"])
      assert flags["verbose"] == true
    end

    test "-c sets changes" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["chown", "-c", "alice", "file"])
      assert flags["changes"] == true
    end

    test "-f sets silent" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["chown", "-f", "alice", "file"])
      assert flags["silent"] == true
    end

    test "-h sets no-dereference" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["chown", "-h", "alice", "file"])
      assert flags["no_dereference"] == true
    end

    test "--help returns help text" do
      {:ok, %HelpResult{text: text}} = parse_argv(["chown", "--help"])
      assert text =~ "chown"
    end

    test "--version returns version" do
      {:ok, %VersionResult{version: version}} = parse_argv(["chown", "--version"])
      assert version =~ "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: parse_owner_group
  # ---------------------------------------------------------------------------

  describe "parse_owner_group" do
    test "owner only" do
      assert UnixTools.Chown.parse_owner_group("alice") == {"alice", nil}
    end

    test "owner:group with colon" do
      assert UnixTools.Chown.parse_owner_group("alice:staff") == {"alice", "staff"}
    end

    test "owner: with trailing colon" do
      assert UnixTools.Chown.parse_owner_group("alice:") == {"alice", ""}
    end

    test ":group with leading colon" do
      assert UnixTools.Chown.parse_owner_group(":staff") == {nil, "staff"}
    end

    test "owner.group with dot" do
      assert UnixTools.Chown.parse_owner_group("alice.staff") == {"alice", "staff"}
    end

    test "numeric owner" do
      assert UnixTools.Chown.parse_owner_group("1000") == {"1000", nil}
    end

    test "numeric owner:group" do
      assert UnixTools.Chown.parse_owner_group("1000:1001") == {"1000", "1001"}
    end

    test ":numeric group" do
      assert UnixTools.Chown.parse_owner_group(":1001") == {nil, "1001"}
    end
  end

  # ---------------------------------------------------------------------------
  # Test: build_chown_args
  # ---------------------------------------------------------------------------

  describe "build_chown_args" do
    test "basic owner and file" do
      result = UnixTools.Chown.build_chown_args({"alice", nil}, ["file.txt"], %{})
      assert result == ["alice", "file.txt"]
    end

    test "owner:group and file" do
      result = UnixTools.Chown.build_chown_args({"alice", "staff"}, ["file.txt"], %{})
      assert result == ["alice:staff", "file.txt"]
    end

    test ":group only" do
      result = UnixTools.Chown.build_chown_args({nil, "staff"}, ["file.txt"], %{})
      assert result == [":staff", "file.txt"]
    end

    test "multiple files" do
      result = UnixTools.Chown.build_chown_args({"alice", nil}, ["f1", "f2", "f3"], %{})
      assert result == ["alice", "f1", "f2", "f3"]
    end

    test "with recursive flag" do
      result = UnixTools.Chown.build_chown_args({"alice", nil}, ["dir"], %{recursive: true})
      assert result == ["-R", "alice", "dir"]
    end

    test "with verbose flag" do
      result = UnixTools.Chown.build_chown_args({"alice", nil}, ["file"], %{verbose: true})
      assert result == ["-v", "alice", "file"]
    end

    test "with changes flag" do
      result = UnixTools.Chown.build_chown_args({"alice", nil}, ["file"], %{changes: true})
      assert result == ["-c", "alice", "file"]
    end

    test "with silent flag" do
      result = UnixTools.Chown.build_chown_args({"alice", nil}, ["file"], %{silent: true})
      assert result == ["-f", "alice", "file"]
    end

    test "with no-dereference flag" do
      result = UnixTools.Chown.build_chown_args({"alice", nil}, ["link"], %{no_dereference: true})
      assert result == ["-h", "alice", "link"]
    end

    test "with multiple flags" do
      result = UnixTools.Chown.build_chown_args(
        {"alice", "staff"},
        ["dir"],
        %{recursive: true, verbose: true}
      )
      assert "-R" in result
      assert "-v" in result
      assert "alice:staff" in result
      assert "dir" in result
    end
  end

  # ---------------------------------------------------------------------------
  # Test: get_file_info
  # ---------------------------------------------------------------------------

  describe "get_file_info" do
    @tag :tmp_dir
    test "retrieves file info", %{tmp_dir: tmp} do
      path = Path.join(tmp, "test.txt")
      File.write!(path, "content")

      {:ok, info} = UnixTools.Chown.get_file_info(path)
      assert is_integer(info.uid)
      assert is_integer(info.gid)
      assert info.type == :regular
    end

    @tag :tmp_dir
    test "retrieves directory info", %{tmp_dir: tmp} do
      dir = Path.join(tmp, "subdir")
      File.mkdir_p!(dir)

      {:ok, info} = UnixTools.Chown.get_file_info(dir)
      assert info.type == :directory
    end

    test "returns error for nonexistent file" do
      {:error, reason} = UnixTools.Chown.get_file_info("/nonexistent/path/xyz")
      assert reason == :enoent
    end
  end

  # ---------------------------------------------------------------------------
  # Test: execute_chown (limited — requires root for real changes)
  # ---------------------------------------------------------------------------

  describe "execute_chown" do
    @tag :tmp_dir
    test "chown to current user succeeds", %{tmp_dir: tmp} do
      path = Path.join(tmp, "test.txt")
      File.write!(path, "content")

      # Get current user
      {whoami_output, 0} = System.cmd("whoami", [])
      current_user = String.trim(whoami_output)

      # Chown to self should succeed
      {_output, exit_code} = UnixTools.Chown.execute_chown({current_user, nil}, [path], %{})
      assert exit_code == 0
    end

    @tag :tmp_dir
    test "chown to nonexistent user fails", %{tmp_dir: tmp} do
      path = Path.join(tmp, "test.txt")
      File.write!(path, "content")

      {_output, exit_code} = UnixTools.Chown.execute_chown(
        {"nonexistent_user_xyz_99999", nil}, [path], %{}
      )
      assert exit_code != 0
    end
  end
end
