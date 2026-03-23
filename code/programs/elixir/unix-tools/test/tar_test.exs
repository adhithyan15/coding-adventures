defmodule TarTest do
  @moduledoc """
  Tests for the tar tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version).
  2. File collection (recursive directory traversal).
  3. Archive creation using :erl_tar.
  4. Archive listing.
  5. Archive extraction.
  6. Path stripping (--strip-components).
  7. Compression support (gzip).
  8. Round-trip: create then extract.
  """

  use ExUnit.Case, async: false

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "tar.json"]) |> Path.expand()

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Helper: create temp directory
  # ---------------------------------------------------------------------------

  defp with_tmp_dir(fun) do
    tmp = Path.join(System.tmp_dir!(), "tar_test_#{:rand.uniform(1_000_000)}")
    File.mkdir_p!(tmp)

    try do
      fun.(tmp)
    after
      File.rm_rf!(tmp)
    end
  end

  # ---------------------------------------------------------------------------
  # Test: CLI parsing
  # ---------------------------------------------------------------------------

  describe "CLI parsing" do
    test "-c sets create mode" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["tar", "-c", "-f", "a.tar", "file"])
      assert flags["create"] == true
    end

    test "-x sets extract mode" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["tar", "-x", "-f", "a.tar"])
      assert flags["extract"] == true
    end

    test "-t sets list mode" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["tar", "-t", "-f", "a.tar"])
      assert flags["list"] == true
    end

    test "-f sets archive file" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["tar", "-c", "-f", "archive.tar", "file"])
      assert flags["file"] == "archive.tar"
    end

    test "-v sets verbose" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["tar", "-c", "-v", "-f", "a.tar", "file"])
      assert flags["verbose"] == true
    end

    test "-z sets gzip compression" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["tar", "-c", "-z", "-f", "a.tar.gz", "file"])
      assert flags["gzip"] == true
    end

    test "-C sets directory" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["tar", "-x", "-f", "a.tar", "-C", "/tmp"])
      assert flags["directory"] == "/tmp"
    end

    test "-k sets keep-old-files" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["tar", "-x", "-k", "-f", "a.tar"])
      assert flags["keep_old_files"] == true
    end

    test "--help returns help text" do
      {:ok, %HelpResult{text: text}} = parse_argv(["tar", "--help"])
      assert text =~ "tar"
    end

    test "--version returns version" do
      {:ok, %VersionResult{version: version}} = parse_argv(["tar", "--version"])
      assert version =~ "1.0.0"
    end

    test "file arguments captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["tar", "-c", "-f", "a.tar", "f1", "f2"])
      assert arguments["files"] == ["f1", "f2"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: collect_files
  # ---------------------------------------------------------------------------

  describe "collect_files" do
    test "single file" do
      with_tmp_dir(fn tmp ->
        path = Path.join(tmp, "file.txt")
        File.write!(path, "content")

        result = UnixTools.Tar.collect_files(path)
        assert result == [path]
      end)
    end

    test "directory with files" do
      with_tmp_dir(fn tmp ->
        dir = Path.join(tmp, "mydir")
        File.mkdir_p!(dir)
        File.write!(Path.join(dir, "a.txt"), "a")
        File.write!(Path.join(dir, "b.txt"), "b")

        result = UnixTools.Tar.collect_files(dir)
        assert dir in result
        assert Path.join(dir, "a.txt") in result
        assert Path.join(dir, "b.txt") in result
      end)
    end

    test "nested directories" do
      with_tmp_dir(fn tmp ->
        dir = Path.join(tmp, "top")
        subdir = Path.join(dir, "sub")
        File.mkdir_p!(subdir)
        File.write!(Path.join(dir, "top.txt"), "top")
        File.write!(Path.join(subdir, "sub.txt"), "sub")

        result = UnixTools.Tar.collect_files(dir)
        assert length(result) == 4  # top dir, sub dir, 2 files
      end)
    end

    test "nonexistent path returns empty" do
      result = ExUnit.CaptureIO.capture_io(:stderr, fn ->
        send(self(), {:result, UnixTools.Tar.collect_files("/nonexistent/path")})
      end)

      assert result =~ "No such file"
      assert_received {:result, []}
    end
  end

  # ---------------------------------------------------------------------------
  # Test: strip_path_components
  # ---------------------------------------------------------------------------

  describe "strip_path_components" do
    test "nil strips nothing" do
      assert UnixTools.Tar.strip_path_components("a/b/c", nil) == "a/b/c"
    end

    test "strip 0 components" do
      assert UnixTools.Tar.strip_path_components("a/b/c", 0) == "a/b/c"
    end

    test "strip 1 component" do
      assert UnixTools.Tar.strip_path_components("a/b/c/file.txt", 1) == "b/c/file.txt"
    end

    test "strip 2 components" do
      assert UnixTools.Tar.strip_path_components("a/b/c/file.txt", 2) == "c/file.txt"
    end

    test "strip all components returns empty" do
      assert UnixTools.Tar.strip_path_components("a/b", 2) == ""
    end

    test "strip more than available returns empty" do
      assert UnixTools.Tar.strip_path_components("a", 5) == ""
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Archive creation and extraction round-trip
  # ---------------------------------------------------------------------------

  describe "archive round-trip" do
    test "create and list a tar archive" do
      with_tmp_dir(fn tmp ->
        # Create source files
        src_dir = Path.join(tmp, "source")
        File.mkdir_p!(src_dir)
        File.write!(Path.join(src_dir, "hello.txt"), "Hello, World!")
        File.write!(Path.join(src_dir, "data.txt"), "Some data here")

        archive_path = Path.join(tmp, "test.tar")

        # Create the archive
        opts = %{verbose: false, directory: nil, gzip: false}

        # Use :erl_tar directly to create
        files = [
          Path.join(src_dir, "hello.txt"),
          Path.join(src_dir, "data.txt")
        ]

        result = UnixTools.Tar.create_archive(files, archive_path, opts)
        assert result == :ok
        assert File.exists?(archive_path)

        # List the archive
        {:ok, entries} = UnixTools.Tar.list_archive_entries(archive_path, %{})
        assert length(entries) == 2
      end)
    end

    test "create and extract a tar archive" do
      with_tmp_dir(fn tmp ->
        # Create source files
        src_dir = Path.join(tmp, "source")
        File.mkdir_p!(src_dir)
        File.write!(Path.join(src_dir, "file1.txt"), "content1")
        File.write!(Path.join(src_dir, "file2.txt"), "content2")

        archive_path = Path.join(tmp, "test.tar")

        # Create using relative paths (change to src_dir)
        opts = %{verbose: false, directory: src_dir, gzip: false}
        result = UnixTools.Tar.create_archive(["file1.txt", "file2.txt"], archive_path, opts)
        assert result == :ok

        # Extract to a new directory
        extract_dir = Path.join(tmp, "extracted")
        File.mkdir_p!(extract_dir)

        extract_opts = %{directory: extract_dir, verbose: false, gzip: false, strip_components: nil}
        extract_result = UnixTools.Tar.extract_archive(archive_path, extract_opts)
        assert extract_result == :ok

        # Verify extracted files
        assert File.read!(Path.join(extract_dir, "file1.txt")) == "content1"
        assert File.read!(Path.join(extract_dir, "file2.txt")) == "content2"
      end)
    end

    test "create and extract with gzip compression" do
      with_tmp_dir(fn tmp ->
        # Create source file
        src_dir = Path.join(tmp, "source")
        File.mkdir_p!(src_dir)
        File.write!(Path.join(src_dir, "compressed.txt"), "This will be compressed")

        archive_path = Path.join(tmp, "test.tar.gz")

        # Create gzipped archive
        opts = %{verbose: false, directory: src_dir, gzip: true}
        result = UnixTools.Tar.create_archive(["compressed.txt"], archive_path, opts)
        assert result == :ok
        assert File.exists?(archive_path)

        # Extract gzipped archive
        extract_dir = Path.join(tmp, "extracted")
        File.mkdir_p!(extract_dir)

        extract_opts = %{directory: extract_dir, verbose: false, gzip: true, strip_components: nil}
        extract_result = UnixTools.Tar.extract_archive(archive_path, extract_opts)
        assert extract_result == :ok

        assert File.read!(Path.join(extract_dir, "compressed.txt")) == "This will be compressed"
      end)
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Archive with directories
  # ---------------------------------------------------------------------------

  describe "archive with directories" do
    test "create archive from directory tree" do
      with_tmp_dir(fn tmp ->
        # Create a directory tree
        src_dir = Path.join(tmp, "project")
        sub_dir = Path.join(src_dir, "src")
        File.mkdir_p!(sub_dir)
        File.write!(Path.join(src_dir, "README.md"), "# Project")
        File.write!(Path.join(sub_dir, "main.ex"), "defmodule Main do end")

        archive_path = Path.join(tmp, "project.tar")

        # Collect all files
        files = UnixTools.Tar.collect_files(src_dir)
        assert length(files) >= 4  # dir, subdir, 2 files

        # Create archive
        opts = %{verbose: false, directory: nil, gzip: false}
        result = UnixTools.Tar.create_archive(files, archive_path, opts)
        assert result == :ok
        assert File.exists?(archive_path)
      end)
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Edge cases
  # ---------------------------------------------------------------------------

  describe "edge cases" do
    test "empty file list creates empty archive" do
      with_tmp_dir(fn tmp ->
        archive_path = Path.join(tmp, "empty.tar")
        opts = %{verbose: false, directory: nil, gzip: false}
        result = UnixTools.Tar.create_archive([], archive_path, opts)
        assert result == :ok
        assert File.exists?(archive_path)
      end)
    end

    test "list nonexistent archive returns error" do
      result = UnixTools.Tar.list_archive_entries("/nonexistent.tar", %{})
      assert {:error, _reason} = result
    end

    test "extract nonexistent archive returns error" do
      result = UnixTools.Tar.extract_archive(
        "/nonexistent.tar",
        %{directory: "/tmp", verbose: false, gzip: false, strip_components: nil}
      )
      assert {:error, _reason} = result
    end
  end
end
