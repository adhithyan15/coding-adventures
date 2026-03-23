defmodule UnixTools.Ls do
  @moduledoc """
  ls -- list directory contents.

  ## What This Program Does

  This is a reimplementation of the GNU `ls` utility in Elixir. It lists
  information about files and directory contents.

  ## How ls Works

  At its simplest:

      ls              =>   lists current directory
      ls /tmp         =>   lists /tmp
      ls -l           =>   long listing with permissions, size, date
      ls -la          =>   long listing including hidden files

  ## Display Modes

  ls has several ways to display its output:

  | Flag | Mode                | Example Output                         |
  |------|---------------------|----------------------------------------|
  | (none)| Multi-column       | file1  file2  file3                    |
  | -1   | One per line        | file1\\nfile2\\nfile3                    |
  | -l   | Long format         | -rw-r--r--  1 user  group  1234  ...   |

  ## Sorting

  By default, ls sorts alphabetically. Other sort modes:

  | Flag | Sort By        | Example                                |
  |------|----------------|----------------------------------------|
  | -S   | Size           | Largest files first                    |
  | -t   | Modification   | Newest files first                     |
  | -X   | Extension      | .c before .h before .txt               |
  | -U   | Unsorted       | Directory order (fastest)              |
  | -r   | Reverse        | Reverses any sort mode                 |

  ## Hidden Files

  - By default, files starting with `.` are hidden.
  - `-a` shows ALL entries including `.` and `..`.
  - `-A` shows almost all (hidden files, but not `.` and `..`).

  ## Human-Readable Sizes

  With `-h`, sizes are displayed as K, M, G instead of raw bytes:

      1024     =>   1K
      1048576  =>   1M

  ## Classify (-F)

  Appends an indicator character to each entry:

  - `/` for directories
  - `*` for executables
  - `@` for symlinks
  - `|` for FIFOs
  - `=` for sockets

  ## Implementation Approach

  Business logic is organized as pure functions:

  1. `list_entries/2` gathers entries from the filesystem.
  2. `sort_entries/2` sorts them according to flags.
  3. `format_entry/2` formats each entry for display.
  4. `format_size_human/1` converts bytes to human-readable form.
  """

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Entry point
  # ---------------------------------------------------------------------------

  @doc """
  Entry point. Receives `argv` as a list of strings.
  """
  def main(argv) do
    spec_path = resolve_spec_path()

    case Parser.parse(spec_path, ["ls" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        opts = %{
          show_all: !!flags["all"],
          almost_all: !!flags["almost_all"],
          long_format: !!flags["long"],
          human_readable: !!flags["human_readable"],
          reverse_sort: !!flags["reverse"],
          recursive: !!flags["recursive"],
          sort_by_size: !!flags["sort_by_size"],
          sort_by_time: !!flags["sort_by_time"],
          sort_by_extension: !!flags["sort_by_extension"],
          unsorted: !!flags["unsorted"],
          classify: !!flags["classify"],
          one_per_line: !!flags["one_per_line"],
          inode: !!flags["inode"],
          directory_only: !!flags["directory"]
        }

        paths = normalize_paths(arguments["files"])

        if length(paths) > 1 do
          # When multiple paths are given, label each section.
          Enum.each(paths, fn path ->
            IO.puts("#{path}:")
            list_and_print(path, opts)
            IO.puts("")
          end)
        else
          list_and_print(hd(paths), opts)
        end

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "ls: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic
  # ---------------------------------------------------------------------------

  @doc """
  List entries in a directory, applying visibility filters.

  - By default, entries starting with `.` are hidden.
  - With `show_all`, all entries are shown (including `.` and `..`).
  - With `almost_all`, hidden files are shown but `.` and `..` are excluded.

  Returns a list of entry name strings.

  ## Examples

      iex> entries = [".", "..", ".hidden", "visible", "also_visible"]
      iex> UnixTools.Ls.filter_entries(entries, %{show_all: false, almost_all: false})
      ["visible", "also_visible"]

      iex> entries = [".", "..", ".hidden", "visible"]
      iex> UnixTools.Ls.filter_entries(entries, %{show_all: true, almost_all: false})
      [".", "..", ".hidden", "visible"]

      iex> entries = [".", "..", ".hidden", "visible"]
      iex> UnixTools.Ls.filter_entries(entries, %{show_all: false, almost_all: true})
      [".hidden", "visible"]
  """
  def filter_entries(entries, opts) do
    cond do
      opts.show_all ->
        entries

      opts.almost_all ->
        Enum.reject(entries, fn name -> name == "." or name == ".." end)

      true ->
        Enum.reject(entries, fn name -> String.starts_with?(name, ".") end)
    end
  end

  @doc """
  List entries from a directory path.

  If `directory_only` is true, lists the directory itself rather than its
  contents. Otherwise, reads the directory with `File.ls!/1` and adds
  `.` and `..` entries (needed for `-a`).
  """
  def list_entries(path, opts) do
    if opts.directory_only do
      [path]
    else
      case File.ls(path) do
        {:ok, entries} ->
          all_entries = [".", ".." | entries]
          filter_entries(all_entries, opts)

        {:error, :enotdir} ->
          # It's a file, not a directory — just list it.
          [path]

        {:error, reason} ->
          IO.puts(:stderr, "ls: cannot access '#{path}': #{:file.format_error(reason)}")
          []
      end
    end
  end

  @doc """
  Sort entries according to the flags.

  ## Sort Modes

  The sort mode is determined by examining flags in priority order:

  1. `unsorted` — no sorting at all (directory order)
  2. `sort_by_size` — largest first
  3. `sort_by_time` — newest first
  4. `sort_by_extension` — alphabetical by extension
  5. Default — alphabetical by name (case-insensitive)

  After sorting, if `reverse_sort` is true, the list is reversed.
  """
  def sort_entries(entries, dir, opts) do
    sorted =
      cond do
        opts.unsorted ->
          entries

        opts.sort_by_size ->
          Enum.sort_by(entries, fn name ->
            stat_path = if name in [".", ".."], do: dir, else: Path.join(dir, name)

            case File.stat(stat_path) do
              {:ok, stat_info} -> -stat_info.size
              _ -> 0
            end
          end)

        opts.sort_by_time ->
          Enum.sort_by(entries, fn name ->
            stat_path = if name in [".", ".."], do: dir, else: Path.join(dir, name)

            case File.stat(stat_path) do
              {:ok, stat_info} -> stat_info.mtime
              _ -> {{0, 0, 0}, {0, 0, 0}}
            end
          end, :desc)

        opts.sort_by_extension ->
          Enum.sort_by(entries, fn name -> Path.extname(name) end)

        true ->
          Enum.sort_by(entries, fn name -> String.downcase(name) end)
      end

    if opts.reverse_sort, do: Enum.reverse(sorted), else: sorted
  end

  @doc """
  Format a file size in human-readable form.

  Converts bytes to the largest unit where the value is >= 1:

      format_size_human(0)          => "0"
      format_size_human(512)        => "512"
      format_size_human(1024)       => "1K"
      format_size_human(1048576)    => "1M"
      format_size_human(1073741824) => "1G"

  ## How the Algorithm Works

  We divide by 1024 repeatedly until the value is < 1024 or we run out
  of units. The units are: (bytes), K, M, G, T, P.

  ## Examples

      iex> UnixTools.Ls.format_size_human(0)
      "0"

      iex> UnixTools.Ls.format_size_human(1024)
      "1K"

      iex> UnixTools.Ls.format_size_human(1536)
      "2K"

      iex> UnixTools.Ls.format_size_human(1048576)
      "1M"
  """
  def format_size_human(size) when size < 1024, do: "#{size}"

  def format_size_human(size) do
    units = ["K", "M", "G", "T", "P"]
    format_size_human(size / 1024, units)
  end

  defp format_size_human(size, [unit]) do
    "#{round(size)}#{unit}"
  end

  defp format_size_human(size, [unit | _rest]) when size < 1024 do
    "#{round(size)}#{unit}"
  end

  defp format_size_human(size, [_unit | remaining]) do
    format_size_human(size / 1024, remaining)
  end

  @doc """
  Map a file type atom to a permission string prefix.

  In `ls -l`, the first character indicates the file type:

      format_type(:regular)   => "-"
      format_type(:directory) => "d"
      format_type(:symlink)   => "l"
  """
  def format_type(:regular), do: "-"
  def format_type(:directory), do: "d"
  def format_type(:symlink), do: "l"
  def format_type(:device), do: "c"
  def format_type(:other), do: "?"
  def format_type(_), do: "?"

  @doc """
  Convert a numeric permission mode to a 9-character rwx string.

  Unix permissions use 3 sets of 3 bits: owner, group, other.
  Each set has read (4), write (2), execute (1):

      format_permissions(0o755) => "rwxr-xr-x"
      format_permissions(0o644) => "rw-r--r--"

  ## How It Works

  We extract each 3-bit group using bitwise AND and shifts:

      0o755 = 111 101 101 (binary)
        owner:  rwx  (7 = 4+2+1)
        group:  r-x  (5 = 4+1)
        other:  r-x  (5 = 4+1)

  ## Examples

      iex> UnixTools.Ls.format_permissions(0o755)
      "rwxr-xr-x"

      iex> UnixTools.Ls.format_permissions(0o644)
      "rw-r--r--"

      iex> UnixTools.Ls.format_permissions(0o000)
      "---------"
  """
  def format_permissions(mode) do
    # -------------------------------------------------------------------------
    # Extract each 3-bit group and convert to rwx string.
    # -------------------------------------------------------------------------

    [
      if(Bitwise.band(mode, 0o400) != 0, do: "r", else: "-"),
      if(Bitwise.band(mode, 0o200) != 0, do: "w", else: "-"),
      if(Bitwise.band(mode, 0o100) != 0, do: "x", else: "-"),
      if(Bitwise.band(mode, 0o040) != 0, do: "r", else: "-"),
      if(Bitwise.band(mode, 0o020) != 0, do: "w", else: "-"),
      if(Bitwise.band(mode, 0o010) != 0, do: "x", else: "-"),
      if(Bitwise.band(mode, 0o004) != 0, do: "r", else: "-"),
      if(Bitwise.band(mode, 0o002) != 0, do: "w", else: "-"),
      if(Bitwise.band(mode, 0o001) != 0, do: "x", else: "-")
    ]
    |> Enum.join()
  end

  @doc """
  Append a classifier character to an entry name based on its type.

  This implements the `-F` / `--classify` flag:

  | Type       | Suffix | Example     |
  |------------|--------|-------------|
  | directory  | /      | bin/        |
  | symlink    | @      | link@       |
  | executable | *      | script*     |
  | regular    | (none) | file.txt    |

  ## Examples

      iex> UnixTools.Ls.classify_entry("bin", :directory, 0o755)
      "bin/"

      iex> UnixTools.Ls.classify_entry("script", :regular, 0o755)
      "script*"

      iex> UnixTools.Ls.classify_entry("file.txt", :regular, 0o644)
      "file.txt"
  """
  def classify_entry(name, file_type, mode) do
    suffix =
      cond do
        file_type == :directory -> "/"
        file_type == :symlink -> "@"
        Bitwise.band(mode, 0o111) != 0 -> "*"
        true -> ""
      end

    name <> suffix
  end

  @doc """
  Format a long listing line for a single entry.

  The long format looks like:

      -rw-r--r--  1234  Jan 15 10:30  file.txt

  We simplify GNU ls's format slightly — we omit user/group names and
  hard link count since Elixir's `File.stat` doesn't expose these easily.
  """
  def format_long_entry(name, dir, opts) do
    stat_path = if name in [".", ".."], do: dir, else: Path.join(dir, name)

    case File.lstat(stat_path) do
      {:ok, stat_info} ->
        type_char = format_type(stat_info.type)
        perms = format_permissions(stat_info.mode)

        size_str =
          if opts.human_readable do
            format_size_human(stat_info.size)
          else
            "#{stat_info.size}"
          end

        {{_year, month, day}, {hour, minute, _sec}} = stat_info.mtime
        months = ~w(Jan Feb Mar Apr May Jun Jul Aug Sep Oct Nov Dec)
        month_str = Enum.at(months, month - 1, "???")
        date_str = "#{month_str} #{String.pad_leading("#{day}", 2)} #{String.pad_leading("#{hour}", 2, "0")}:#{String.pad_leading("#{minute}", 2, "0")}"

        display_name =
          if opts.classify do
            classify_entry(name, stat_info.type, stat_info.mode)
          else
            name
          end

        inode_prefix =
          if opts.inode do
            "#{stat_info.inode} "
          else
            ""
          end

        "#{inode_prefix}#{type_char}#{perms}  #{String.pad_leading(size_str, 8)}  #{date_str}  #{display_name}"

      {:error, reason} ->
        "? (#{:file.format_error(reason)}) #{name}"
    end
  end

  # ---------------------------------------------------------------------------
  # Orchestration
  # ---------------------------------------------------------------------------

  defp list_and_print(path, opts) do
    entries = list_entries(path, opts)
    sorted = sort_entries(entries, path, opts)

    if opts.long_format do
      Enum.each(sorted, fn name ->
        IO.puts(format_long_entry(name, path, opts))
      end)
    else
      display_entries =
        if opts.classify do
          Enum.map(sorted, fn name ->
            stat_path = if name in [".", ".."], do: path, else: Path.join(path, name)

            case File.lstat(stat_path) do
              {:ok, stat_info} -> classify_entry(name, stat_info.type, stat_info.mode)
              _ -> name
            end
          end)
        else
          sorted
        end

      if opts.one_per_line or opts.long_format do
        Enum.each(display_entries, &IO.puts/1)
      else
        IO.puts(Enum.join(display_entries, "  "))
      end
    end

    # -------------------------------------------------------------------------
    # Recursive listing
    # -------------------------------------------------------------------------

    if opts.recursive do
      subdirs =
        sorted
        |> Enum.reject(fn name -> name in [".", ".."] end)
        |> Enum.filter(fn name -> File.dir?(Path.join(path, name)) end)

      Enum.each(subdirs, fn subdir ->
        subpath = Path.join(path, subdir)
        IO.puts("\n#{subpath}:")
        list_and_print(subpath, opts)
      end)
    end
  end

  defp normalize_paths(nil), do: ["."]
  defp normalize_paths(paths) when is_list(paths) and length(paths) == 0, do: ["."]
  defp normalize_paths(paths) when is_list(paths), do: paths
  defp normalize_paths(path) when is_binary(path), do: [path]

  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "ls.json"),
        else: nil
      ),
      "ls.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "ls.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find ls.json spec file"
  end
end
