defmodule MvTest do
  @moduledoc """
  Tests for the mv tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Flag parsing (-f, -n, -v, -u).
  3. Business logic (resolve_destination, should_skip?, split_sources_and_dest).
  4. File move operations (rename, no-clobber, verbose).
  """

  use ExUnit.Case, async: false

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "mv.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the mv spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Helper: create a temporary directory for file operations
  # ---------------------------------------------------------------------------

  defp with_tmp_dir(fun) do
    tmp = Path.join(System.tmp_dir!(), "mv_test_#{:rand.uniform(1_000_000)}")
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
        parse_argv(["mv", "src.txt", "dest.txt"])

      assert arguments["sources"] == ["src.txt", "dest.txt"]
    end

    test "multiple sources and destination are captured" do
      {:ok, %ParseResult{arguments: arguments}} =
        parse_argv(["mv", "a.txt", "b.txt", "dir/"])

      assert arguments["sources"] == ["a.txt", "b.txt", "dir/"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Flags
  # ---------------------------------------------------------------------------

  describe "flags" do
    test "-f sets force to true" do
      {:ok, %ParseResult{flags: flags}} =
        parse_argv(["mv", "-f", "src.txt", "dest.txt"])

      assert flags["force"] == true
    end

    test "-n sets no-clobber to true" do
      {:ok, %ParseResult{flags: flags}} =
        parse_argv(["mv", "-n", "src.txt", "dest.txt"])

      assert flags["no_clobber"] == true
    end

    test "-v sets verbose to true" do
      {:ok, %ParseResult{flags: flags}} =
        parse_argv(["mv", "-v", "src.txt", "dest.txt"])

      assert flags["verbose"] == true
    end

    test "-u sets update to true" do
      {:ok, %ParseResult{flags: flags}} =
        parse_argv(["mv", "-u", "src.txt", "dest.txt"])

      assert flags["update"] == true
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help and --version
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["mv", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["mv", "--help"])
      assert text =~ "mv"
    end
  end

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["mv", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["mv", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["mv", "--unknown"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - resolve_destination
  # ---------------------------------------------------------------------------

  describe "resolve_destination/3" do
    test "when dest is a directory, appends source basename" do
      assert UnixTools.Mv.resolve_destination("docs/file.txt", "/backup", true) ==
               "/backup/file.txt"
    end

    test "when dest is not a directory, returns dest as-is" do
      assert UnixTools.Mv.resolve_destination("old.txt", "new.txt", false) == "new.txt"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - split_sources_and_dest
  # ---------------------------------------------------------------------------

  describe "split_sources_and_dest/1" do
    test "splits two arguments into one source and one dest" do
      {sources, dest} = UnixTools.Mv.split_sources_and_dest(["a.txt", "b.txt"])
      assert sources == ["a.txt"]
      assert dest == "b.txt"
    end

    test "splits three arguments into two sources and one dest" do
      {sources, dest} = UnixTools.Mv.split_sources_and_dest(["a.txt", "b.txt", "dir/"])
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
        assert UnixTools.Mv.should_skip?(src, dest, opts) == true
      end)
    end

    test "does not skip when no_clobber is false" do
      with_tmp_dir(fn tmp ->
        src = Path.join(tmp, "src.txt")
        dest = Path.join(tmp, "dest.txt")
        File.write!(src, "source")
        File.write!(dest, "dest")

        opts = %{no_clobber: false, update: false}
        assert UnixTools.Mv.should_skip?(src, dest, opts) == false
      end)
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - move_single
  # ---------------------------------------------------------------------------

  describe "move_single/3" do
    test "renames a file" do
      with_tmp_dir(fn tmp ->
        src = Path.join(tmp, "src.txt")
        dest = Path.join(tmp, "dest.txt")
        File.write!(src, "hello world")

        opts = %{force: false, no_clobber: false, verbose: false, update: false}
        UnixTools.Mv.move_single(src, dest, opts)

        assert File.read!(dest) == "hello world"
        refute File.exists?(src)
      end)
    end

    test "no-clobber prevents overwriting" do
      with_tmp_dir(fn tmp ->
        src = Path.join(tmp, "src.txt")
        dest = Path.join(tmp, "dest.txt")
        File.write!(src, "new content")
        File.write!(dest, "original content")

        opts = %{force: false, no_clobber: true, verbose: false, update: false}
        UnixTools.Mv.move_single(src, dest, opts)

        assert File.read!(dest) == "original content"
        assert File.exists?(src)
      end)
    end

    test "verbose outputs the rename operation" do
      with_tmp_dir(fn tmp ->
        src = Path.join(tmp, "src.txt")
        dest = Path.join(tmp, "dest.txt")
        File.write!(src, "content")

        opts = %{force: false, no_clobber: false, verbose: true, update: false}

        output =
          ExUnit.CaptureIO.capture_io(fn ->
            UnixTools.Mv.move_single(src, dest, opts)
          end)

        assert output =~ "renamed"
        assert output =~ src
        assert output =~ dest
      end)
    end

    test "moves a file into a directory" do
      with_tmp_dir(fn tmp ->
        src = Path.join(tmp, "file.txt")
        dest_dir = Path.join(tmp, "subdir")
        File.write!(src, "content")
        File.mkdir_p!(dest_dir)

        dest = Path.join(dest_dir, "file.txt")
        opts = %{force: false, no_clobber: false, verbose: false, update: false}
        UnixTools.Mv.move_single(src, dest, opts)

        assert File.read!(dest) == "content"
        refute File.exists?(src)
      end)
    end
  end
end
