defmodule LsTest do
  @moduledoc """
  Tests for the ls tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version, errors).
  2. Flag parsing (-a, -A, -l, -h, -r, -R, -S, -t, -1, -F, -d).
  3. Business logic (filter_entries, sort_entries, format_size_human,
     format_permissions, classify_entry, format_type).
  """

  use ExUnit.Case, async: false

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "ls.json"]) |> Path.expand()

  # ---------------------------------------------------------------------------
  # Helper: parse argv against the ls spec
  # ---------------------------------------------------------------------------

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Helper: create a temporary directory for file operations
  # ---------------------------------------------------------------------------

  defp with_tmp_dir(fun) do
    tmp = Path.join(System.tmp_dir!(), "ls_test_#{:rand.uniform(1_000_000)}")
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
    test "no arguments returns ParseResult" do
      assert {:ok, %ParseResult{}} = parse_argv(["ls"])
    end

    test "directory argument is captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["ls", "/tmp"])
      assert arguments["files"] == ["/tmp"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Flags
  # ---------------------------------------------------------------------------

  describe "flags" do
    test "-a sets all to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["ls", "-a"])
      assert flags["all"] == true
    end

    test "-A sets almost_all to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["ls", "-A"])
      assert flags["almost_all"] == true
    end

    test "-l sets long to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["ls", "-l"])
      assert flags["long"] == true
    end

    test "-h sets human_readable to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["ls", "-h"])
      assert flags["human_readable"] == true
    end

    test "-r sets reverse to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["ls", "-r"])
      assert flags["reverse"] == true
    end

    test "-R sets recursive to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["ls", "-R"])
      assert flags["recursive"] == true
    end

    test "-S sets sort_by_size to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["ls", "-S"])
      assert flags["sort_by_size"] == true
    end

    test "-t sets sort_by_time to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["ls", "-t"])
      assert flags["sort_by_time"] == true
    end

    test "-1 sets one_per_line to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["ls", "-1"])
      assert flags["one_per_line"] == true
    end

    test "-F sets classify to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["ls", "-F"])
      assert flags["classify"] == true
    end

    test "-d sets directory to true" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["ls", "-d"])
      assert flags["directory"] == true
    end
  end

  # ---------------------------------------------------------------------------
  # Test: --help and --version
  # ---------------------------------------------------------------------------

  describe "--help flag" do
    test "returns {:ok, %HelpResult{}}" do
      assert {:ok, %HelpResult{}} = parse_argv(["ls", "--help"])
    end

    test "help text contains program name" do
      {:ok, %HelpResult{text: text}} = parse_argv(["ls", "--help"])
      assert text =~ "ls"
    end
  end

  describe "--version flag" do
    test "returns {:ok, %VersionResult{}}" do
      assert {:ok, %VersionResult{}} = parse_argv(["ls", "--version"])
    end

    test "version string is 1.0.0" do
      {:ok, %VersionResult{version: version}} = parse_argv(["ls", "--version"])
      assert version == "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Unknown flags
  # ---------------------------------------------------------------------------

  describe "unknown flags" do
    test "unknown long flag returns error" do
      assert {:error, %ParseErrors{}} = parse_argv(["ls", "--unknown"])
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - filter_entries
  # ---------------------------------------------------------------------------

  describe "filter_entries/2" do
    test "default: hides dotfiles" do
      entries = [".", "..", ".hidden", "visible", "also_visible"]
      opts = %{show_all: false, almost_all: false}
      result = UnixTools.Ls.filter_entries(entries, opts)
      assert result == ["visible", "also_visible"]
    end

    test "show_all: shows everything including . and .." do
      entries = [".", "..", ".hidden", "visible"]
      opts = %{show_all: true, almost_all: false}
      result = UnixTools.Ls.filter_entries(entries, opts)
      assert result == [".", "..", ".hidden", "visible"]
    end

    test "almost_all: shows hidden but not . and .." do
      entries = [".", "..", ".hidden", "visible"]
      opts = %{show_all: false, almost_all: true}
      result = UnixTools.Ls.filter_entries(entries, opts)
      assert result == [".hidden", "visible"]
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - sort_entries
  # ---------------------------------------------------------------------------

  describe "sort_entries/3" do
    test "default alphabetical sort (case-insensitive)" do
      with_tmp_dir(fn tmp ->
        File.write!(Path.join(tmp, "banana"), "")
        File.write!(Path.join(tmp, "apple"), "")
        File.write!(Path.join(tmp, "Cherry"), "")

        entries = ["banana", "apple", "Cherry"]

        opts = %{
          unsorted: false,
          sort_by_size: false,
          sort_by_time: false,
          sort_by_extension: false,
          reverse_sort: false
        }

        result = UnixTools.Ls.sort_entries(entries, tmp, opts)
        assert result == ["apple", "banana", "Cherry"]
      end)
    end

    test "reverse sort" do
      with_tmp_dir(fn tmp ->
        File.write!(Path.join(tmp, "a"), "")
        File.write!(Path.join(tmp, "b"), "")
        File.write!(Path.join(tmp, "c"), "")

        entries = ["a", "b", "c"]

        opts = %{
          unsorted: false,
          sort_by_size: false,
          sort_by_time: false,
          sort_by_extension: false,
          reverse_sort: true
        }

        result = UnixTools.Ls.sort_entries(entries, tmp, opts)
        assert result == ["c", "b", "a"]
      end)
    end

    test "sort by extension" do
      with_tmp_dir(fn tmp ->
        File.write!(Path.join(tmp, "file.txt"), "")
        File.write!(Path.join(tmp, "file.c"), "")
        File.write!(Path.join(tmp, "file.rb"), "")

        entries = ["file.txt", "file.c", "file.rb"]

        opts = %{
          unsorted: false,
          sort_by_size: false,
          sort_by_time: false,
          sort_by_extension: true,
          reverse_sort: false
        }

        result = UnixTools.Ls.sort_entries(entries, tmp, opts)
        assert result == ["file.c", "file.rb", "file.txt"]
      end)
    end

    test "unsorted preserves order" do
      with_tmp_dir(fn tmp ->
        entries = ["z", "a", "m"]

        opts = %{
          unsorted: true,
          sort_by_size: false,
          sort_by_time: false,
          sort_by_extension: false,
          reverse_sort: false
        }

        result = UnixTools.Ls.sort_entries(entries, tmp, opts)
        assert result == ["z", "a", "m"]
      end)
    end

    test "sort by size" do
      with_tmp_dir(fn tmp ->
        File.write!(Path.join(tmp, "small"), "ab")
        File.write!(Path.join(tmp, "large"), "abcdefghij")
        File.write!(Path.join(tmp, "medium"), "abcde")

        entries = ["small", "large", "medium"]

        opts = %{
          unsorted: false,
          sort_by_size: true,
          sort_by_time: false,
          sort_by_extension: false,
          reverse_sort: false
        }

        result = UnixTools.Ls.sort_entries(entries, tmp, opts)
        assert result == ["large", "medium", "small"]
      end)
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - format_size_human
  # ---------------------------------------------------------------------------

  describe "format_size_human/1" do
    test "small sizes stay as bytes" do
      assert UnixTools.Ls.format_size_human(0) == "0"
      assert UnixTools.Ls.format_size_human(512) == "512"
      assert UnixTools.Ls.format_size_human(1023) == "1023"
    end

    test "kilobytes" do
      assert UnixTools.Ls.format_size_human(1024) == "1K"
      assert UnixTools.Ls.format_size_human(2048) == "2K"
    end

    test "megabytes" do
      assert UnixTools.Ls.format_size_human(1_048_576) == "1M"
    end

    test "gigabytes" do
      assert UnixTools.Ls.format_size_human(1_073_741_824) == "1G"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - format_permissions
  # ---------------------------------------------------------------------------

  describe "format_permissions/1" do
    test "full permissions 0755" do
      assert UnixTools.Ls.format_permissions(0o755) == "rwxr-xr-x"
    end

    test "read-write permissions 0644" do
      assert UnixTools.Ls.format_permissions(0o644) == "rw-r--r--"
    end

    test "no permissions 0000" do
      assert UnixTools.Ls.format_permissions(0o000) == "---------"
    end

    test "full permissions 0777" do
      assert UnixTools.Ls.format_permissions(0o777) == "rwxrwxrwx"
    end

    test "write-only 0222" do
      assert UnixTools.Ls.format_permissions(0o222) == "-w--w--w-"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - format_type
  # ---------------------------------------------------------------------------

  describe "format_type/1" do
    test "regular file" do
      assert UnixTools.Ls.format_type(:regular) == "-"
    end

    test "directory" do
      assert UnixTools.Ls.format_type(:directory) == "d"
    end

    test "symlink" do
      assert UnixTools.Ls.format_type(:symlink) == "l"
    end

    test "unknown type" do
      assert UnixTools.Ls.format_type(:unknown) == "?"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - classify_entry
  # ---------------------------------------------------------------------------

  describe "classify_entry/3" do
    test "directories get / suffix" do
      assert UnixTools.Ls.classify_entry("bin", :directory, 0o755) == "bin/"
    end

    test "executable files get * suffix" do
      assert UnixTools.Ls.classify_entry("script", :regular, 0o755) == "script*"
    end

    test "symlinks get @ suffix" do
      assert UnixTools.Ls.classify_entry("link", :symlink, 0o777) == "link@"
    end

    test "regular non-executable files get no suffix" do
      assert UnixTools.Ls.classify_entry("file.txt", :regular, 0o644) == "file.txt"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: Business logic - list_entries
  # ---------------------------------------------------------------------------

  describe "list_entries/2" do
    test "lists files in a directory" do
      with_tmp_dir(fn tmp ->
        File.write!(Path.join(tmp, "a.txt"), "")
        File.write!(Path.join(tmp, "b.txt"), "")

        opts = %{show_all: false, almost_all: false, directory_only: false}
        result = UnixTools.Ls.list_entries(tmp, opts)

        assert "a.txt" in result
        assert "b.txt" in result
        refute "." in result
        refute ".." in result
      end)
    end

    test "lists hidden files with show_all" do
      with_tmp_dir(fn tmp ->
        File.write!(Path.join(tmp, ".hidden"), "")
        File.write!(Path.join(tmp, "visible"), "")

        opts = %{show_all: true, almost_all: false, directory_only: false}
        result = UnixTools.Ls.list_entries(tmp, opts)

        assert "." in result
        assert ".." in result
        assert ".hidden" in result
        assert "visible" in result
      end)
    end

    test "directory_only lists the directory itself" do
      with_tmp_dir(fn tmp ->
        opts = %{show_all: false, almost_all: false, directory_only: true}
        result = UnixTools.Ls.list_entries(tmp, opts)

        assert result == [tmp]
      end)
    end
  end
end
