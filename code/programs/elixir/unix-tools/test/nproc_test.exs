defmodule NprocTest do
  @moduledoc """
  Tests for the nproc tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Flag parsing (--all, --ignore).
  3. Business logic (get_cpu_count/1, compute_result/2).
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "nproc.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the nproc spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "no arguments returns ParseResult" do
      assert {:ok, %ParseResult{}} = parse_argv(["nproc"])
    end

    test "all flag defaults to false" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["nproc"])
      refute flags["all"]
    end

    test "ignore flag defaults to nil" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["nproc"])
      assert flags["ignore"] == nil
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --all flag
  # ---------------------------------------------------------------------------

  describe "--all flag" do
    test "sets all to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["nproc", "--all"])
      assert flags["all"] == true
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --ignore flag
  # ---------------------------------------------------------------------------

  describe "--ignore flag" do
    test "accepts an integer value" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["nproc", "--ignore=2"])
      assert flags["ignore"] == 2
    end

    test "accepts --ignore with separate value" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["nproc", "--ignore", "3"])
      assert flags["ignore"] == 3
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help flag
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["nproc", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["nproc", "--help"])
      assert text =~ "nproc"
    end

    test "help text contains description" do
      {:ok, %HelpResult{text: text}} = parse_argv(["nproc", "--help"])
      assert String.downcase(text) =~ "processing"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --version flag
  # ---------------------------------------------------------------------------

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["nproc", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["nproc", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["nproc", "--unknown"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic — get_cpu_count/1
  # ---------------------------------------------------------------------------

  describe "get_cpu_count/1" do
    test "returns a positive integer for available CPUs" do
      count = UnixTools.Nproc.get_cpu_count(false)
      assert is_integer(count)
      assert count > 0
    end

    test "returns a positive integer for all CPUs" do
      count = UnixTools.Nproc.get_cpu_count(true)
      assert is_integer(count)
      assert count > 0
    end

    test "all count is >= available count" do
      available = UnixTools.Nproc.get_cpu_count(false)
      all = UnixTools.Nproc.get_cpu_count(true)
      assert all >= available
    end

    test "available count matches System.schedulers_online()" do
      count = UnixTools.Nproc.get_cpu_count(false)
      assert count == System.schedulers_online()
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic — compute_result/2
  # ---------------------------------------------------------------------------

  describe "compute_result/2" do
    test "returns count when ignore is 0" do
      assert UnixTools.Nproc.compute_result(8, 0) == 8
    end

    test "subtracts ignore from count" do
      assert UnixTools.Nproc.compute_result(8, 2) == 6
    end

    test "never returns less than 1" do
      assert UnixTools.Nproc.compute_result(4, 10) == 1
    end

    test "returns 1 when count equals ignore" do
      assert UnixTools.Nproc.compute_result(4, 4) == 1
    end

    test "returns 1 when count is 1 and ignore is 0" do
      assert UnixTools.Nproc.compute_result(1, 0) == 1
    end

    test "returns 1 when count is 1 and ignore is 1" do
      assert UnixTools.Nproc.compute_result(1, 1) == 1
    end

    test "handles large ignore values" do
      assert UnixTools.Nproc.compute_result(2, 100) == 1
    end

    test "handles large cpu counts" do
      assert UnixTools.Nproc.compute_result(128, 4) == 124
    end
  end
end
