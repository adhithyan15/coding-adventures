defmodule CpTest do
  @moduledoc """
  Tests for the cp tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Flag parsing (-f, -n, -R, -v, -u, -a).
  3. Business logic (resolve_destination, should_skip?, split_sources_and_dest).
  4. File copy operations (single file, directory, no-clobber, force, verbose).
  """

  use ExUnit.Case, async: false

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "cp.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the cp spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Helper: create a temporary directory for file operations
  # ---------------------------------------------------------------------------

  defp with_tmp_dir(fun) do
    tmp = Path.join(System.tmp_dir!(), "cp_test_#{:rand.uniform(1_000_000)}")
    File.mkdir_p!(tmp)

    try do
      fun.(tmp)
    after
      File.rm_rf!(tmp)
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Default behavior
  # ---------------------------------------------------------------------------

  describe "default behavior" do
    test "source and destination arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} =
        parse_argv(["cp", "src.txt", "dest.txt"])

      assert arguments["sources"] == ["src.txt", "dest.txt"]
    end

    test "multiple sources and destination are captured" do
      {:ok, %ParseResult{arguments: arguments}} =
        parse_argv(["cp", "a.txt", "b.txt", "dir/"])

      assert arguments["sources"] == ["a.txt", "b.txt", "dir/"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Flags
  # ---------------------------------------------------------------------------

  describe "flags" do
    test "-f sets force to true" do
      {:ok, %ParseResult{flags: flags}} =
        parse_argv(["cp", "-f", "src.txt", "dest.txt"])

      assert flags["force"] == true
    end

    test "-n sets no-clobber to true" do
      {:ok, %ParseResult{flags: flags}} =
        parse_argv(["cp", "-n", "src.txt", "dest.txt"])

      assert flags["no_clobber"] == true
    end

    test "-R sets recursive to true" do
      {:ok, %ParseResult{flags: flags}} =
        parse_argv(["cp", "-R", "src/", "dest/"])

      assert flags["recursive"] == true
    end

    test "-v sets verbose to true" do
      {:ok, %ParseResult{flags: flags}} =
        parse_argv(["cp", "-v", "src.txt", "dest.txt"])

      assert flags["verbose"] == true
    end

    test "-u sets update to true" do
      {:ok, %ParseResult{flags: flags}} =
        parse_argv(["cp", "-u", "src.txt", "dest.txt"])

      assert flags["update"] == true
    end

    test "-a sets archive to true" do
      {:ok, %ParseResult{flags: flags}} =
        parse_argv(["cp", "-a", "src/", "dest/"])

      assert flags["archive"] == true
    end

    test "--recursive long flag works" do
      {:ok, %ParseResult{flags: flags}} =
        parse_argv(["cp", "--recursive", "src/", "dest/"])

      assert flags["recursive"] == true
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help and --version
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["cp", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["cp", "--help"])
      assert text =~ "cp"
    end
  end

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["cp", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["cp", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["cp", "--unknown"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - resolve_destination
  # ---------------------------------------------------------------------------

  describe "resolve_destination/3" do
    test "when dest is a directory, appends source basename" do
      assert UnixTools.Cp.resolve_destination("docs/readme.txt", "/backup", true) ==
               "/backup/readme.txt"
    end

    test "when dest is not a directory, returns dest as-is" do
      assert UnixTools.Cp.resolve_destination("a.txt", "b.txt", false) == "b.txt"
    end

    test "handles nested source paths" do
      assert UnixTools.Cp.resolve_destination("/home/user/file.txt", "/tmp", true) ==
               "/tmp/file.txt"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - split_sources_and_dest
  # ---------------------------------------------------------------------------

  describe "split_sources_and_dest/1" do
    test "splits two arguments into one source and one dest" do
      {sources, dest} = UnixTools.Cp.split_sources_and_dest(["a.txt", "b.txt"])
      assert sources == ["a.txt"]
      assert dest == "b.txt"
    end

    test "splits three arguments into two sources and one dest" do
      {sources, dest} = UnixTools.Cp.split_sources_and_dest(["a.txt", "b.txt", "dir/"])
      assert sources == ["a.txt", "b.txt"]
      assert dest == "dir/"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - should_skip?
  # ---------------------------------------------------------------------------

  describe "should_skip?/3" do
    test "skips when no_clobber is true and dest exists" do
      with_tmp_dir(fn tmp ->
        src = Path.join(tmp, "src.txt")
        dest = Path.join(tmp, "dest.txt")
        File.write!(src, "source")
        File.write!(dest, "dest")

        opts = %{no_clobber: true, update: false}
        assert UnixTools.Cp.should_skip?(src, dest, opts) == true
      end)
    end

    test "does not skip when no_clobber is true but dest does not exist" do
      with_tmp_dir(fn tmp ->
        src = Path.join(tmp, "src.txt")
        dest = Path.join(tmp, "nonexistent.txt")
        File.write!(src, "source")

        opts = %{no_clobber: true, update: false}
        assert UnixTools.Cp.should_skip?(src, dest, opts) == false
      end)
    end

    test "does not skip by default" do
      with_tmp_dir(fn tmp ->
        src = Path.join(tmp, "src.txt")
        dest = Path.join(tmp, "dest.txt")
        File.write!(src, "source")
        File.write!(dest, "dest")

        opts = %{no_clobber: false, update: false}
        assert UnixTools.Cp.should_skip?(src, dest, opts) == false
      end)
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - copy_single
  # ---------------------------------------------------------------------------

  describe "copy_single/3" do
    test "copies a single file" do
      with_tmp_dir(fn tmp ->
        src = Path.join(tmp, "src.txt")
        dest = Path.join(tmp, "dest.txt")
        File.write!(src, "hello world")

        opts = %{recursive: false, force: false, no_clobber: false, verbose: false, update: false}
        UnixTools.Cp.copy_single(src, dest, opts)

        assert File.read!(dest) == "hello world"
      end)
    end

    test "copies a file preserving content" do
      with_tmp_dir(fn tmp ->
        src = Path.join(tmp, "data.bin")
        dest = Path.join(tmp, "copy.bin")
        content = :crypto.strong_rand_bytes(256)
        File.write!(src, content)

        opts = %{recursive: false, force: false, no_clobber: false, verbose: false, update: false}
        UnixTools.Cp.copy_single(src, dest, opts)

        assert File.read!(dest) == content
      end)
    end

    test "no-clobber prevents overwriting" do
      with_tmp_dir(fn tmp ->
        src = Path.join(tmp, "src.txt")
        dest = Path.join(tmp, "dest.txt")
        File.write!(src, "new content")
        File.write!(dest, "original content")

        opts = %{recursive: false, force: false, no_clobber: true, verbose: false, update: false}
        UnixTools.Cp.copy_single(src, dest, opts)

        assert File.read!(dest) == "original content"
      end)
    end

    test "recursive copies a directory" do
      with_tmp_dir(fn tmp ->
        src_dir = Path.join(tmp, "src_dir")
        dest_dir = Path.join(tmp, "dest_dir")
        File.mkdir_p!(src_dir)
        File.write!(Path.join(src_dir, "file.txt"), "inside dir")

        opts = %{recursive: true, force: false, no_clobber: false, verbose: false, update: false}
        UnixTools.Cp.copy_single(src_dir, dest_dir, opts)

        assert File.read!(Path.join(dest_dir, "file.txt")) == "inside dir"
      end)
    end

    test "force removes destination before copying" do
      with_tmp_dir(fn tmp ->
        src = Path.join(tmp, "src.txt")
        dest = Path.join(tmp, "dest.txt")
        File.write!(src, "new content")
        File.write!(dest, "old content")

        opts = %{recursive: false, force: true, no_clobber: false, verbose: false, update: false}
        UnixTools.Cp.copy_single(src, dest, opts)

        assert File.read!(dest) == "new content"
      end)
    end

    test "verbose outputs the copy operation" do
      with_tmp_dir(fn tmp ->
        src = Path.join(tmp, "src.txt")
        dest = Path.join(tmp, "dest.txt")
        File.write!(src, "content")

        opts = %{recursive: false, force: false, no_clobber: false, verbose: true, update: false}

        output =
          ExUnit.CaptureIO.capture_io(fn ->
            UnixTools.Cp.copy_single(src, dest, opts)
          end)

        assert output =~ src
        assert output =~ dest
        assert output =~ "->"
      end)
    end
  end
end
