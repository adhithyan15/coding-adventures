defmodule DirnameTest do
  @moduledoc """
  Tests for the dirname tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Business logic (compute_dirname with various paths).
  3. Edge cases (trailing slashes, root, no slashes, empty string).
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "dirname.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the dirname spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "single name argument is captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["dirname", "/usr/bin"])
      assert arguments["names"] == ["/usr/bin"]
    end

    test "multiple name arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} =
        parse_argv(["dirname", "/usr/bin", "/etc/hosts"])

      assert arguments["names"] == ["/usr/bin", "/etc/hosts"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Flags
  # ---------------------------------------------------------------------------

  describe "flags" do
    test "-z sets zero to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["dirname", "-z", "/usr/bin"])
      assert flags["zero"] == true
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help and --version
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["dirname", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["dirname", "--help"])
      assert text =~ "dirname"
    end
  end

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["dirname", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["dirname", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["dirname", "--unknown", "/usr"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - compute_dirname
  # ---------------------------------------------------------------------------

  describe "compute_dirname/1" do
    test "normal absolute path" do
      assert UnixTools.DirnameTool.compute_dirname("/usr/bin") == "/usr"
    end

    test "path with trailing slash" do
      assert UnixTools.DirnameTool.compute_dirname("/usr/") == "/"
    end

    test "path without slashes returns dot" do
      assert UnixTools.DirnameTool.compute_dirname("usr") == "."
    end

    test "root path returns root" do
      assert UnixTools.DirnameTool.compute_dirname("/") == "/"
    end

    test "dot returns dot" do
      assert UnixTools.DirnameTool.compute_dirname(".") == "."
    end

    test "double dot returns dot" do
      assert UnixTools.DirnameTool.compute_dirname("..") == "."
    end

    test "empty string returns dot" do
      assert UnixTools.DirnameTool.compute_dirname("") == "."
    end

    test "deep path" do
      assert UnixTools.DirnameTool.compute_dirname("/usr/local/bin/node") == "/usr/local/bin"
    end

    test "double-slash path" do
      assert UnixTools.DirnameTool.compute_dirname("//usr") == "/"
    end

    test "relative path with directory" do
      assert UnixTools.DirnameTool.compute_dirname("include/stdio.h") == "include"
    end
  end
end
