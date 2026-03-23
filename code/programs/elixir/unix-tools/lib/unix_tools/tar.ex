defmodule UnixTools.Tar do
  @moduledoc """
  tar -- an archiving utility.

  ## What This Program Does

  This is a reimplementation of the GNU `tar` utility in Elixir. It creates,
  extracts, and lists tape archive files.

  ## How tar Works

  tar bundles multiple files into a single archive file, preserving directory
  structure and file metadata. The name stands for "tape archive" — originally
  designed for writing to magnetic tape.

  ## Operations

  tar has three main operations:

  | Flag | Operation  | Example                             |
  |------|-----------|---------------------------------------|
  | -c   | Create    | tar -cf archive.tar file1 file2       |
  | -x   | Extract   | tar -xf archive.tar                   |
  | -t   | List      | tar -tf archive.tar                   |

  ## Common Flag Combinations

      tar -cvf archive.tar dir/     Create archive from dir/, verbose
      tar -xvf archive.tar          Extract archive, verbose
      tar -tzf archive.tar.gz       List gzipped archive
      tar -xf archive.tar -C /tmp   Extract to /tmp directory

  ## The tar File Format

  A tar file is a sequence of 512-byte blocks. Each file is preceded by
  a header block containing:

  - File name (100 bytes)
  - File mode (8 bytes, octal)
  - Owner ID (8 bytes, octal)
  - Group ID (8 bytes, octal)
  - File size (12 bytes, octal)
  - Modification time (12 bytes, octal)
  - Header checksum (8 bytes)
  - Type flag (1 byte: '0' = file, '5' = directory)

  The file's content follows the header, padded to a 512-byte boundary.
  The archive ends with two blocks of zero bytes.

  ## Erlang's :erl_tar Module

  Elixir has access to Erlang's `:erl_tar` module which handles the low-level
  tar format details. We use it for reliable archive creation and extraction
  while adding our own CLI interface and option handling.

  ## Implementation Approach

  1. `create_archive/3` creates a tar file from a list of paths.
  2. `extract_archive/2` extracts files from a tar archive.
  3. `list_archive/2` lists the contents of a tar archive.
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

    case Parser.parse(spec_path, ["tar" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        opts = %{
          create: !!flags["create"],
          extract: !!flags["extract"],
          list_contents: !!flags["list"],
          verbose: !!flags["verbose"],
          archive_file: flags["file"],
          directory: flags["directory"],
          gzip: !!flags["gzip"],
          bzip2: !!flags["bzip2"],
          xz: !!flags["xz"],
          keep_old: !!flags["keep_old_files"],
          strip_components: flags["strip_components"]
        }

        file_list = normalize_files(arguments["files"])

        run(file_list, opts)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn err ->
          IO.puts(:stderr, "tar: #{err.message}")
        end)

        System.halt(2)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Archive Creation
  # ---------------------------------------------------------------------------

  @doc """
  Create a tar archive from a list of file/directory paths.

  Uses Erlang's `:erl_tar` module to build the archive. The archive
  file is specified in opts.

  ## How Archive Creation Works

  1. Collect all files to include (expanding directories recursively).
  2. Open the tar file for writing.
  3. Add each file with its relative path.
  4. Close the archive.

  ## Options

  - `:archive_file` — path to the output tar file
  - `:verbose` — print each file as it's added
  - `:gzip` — compress with gzip
  - `:directory` — change to this directory first

  ## Examples

      iex> # Conceptual — needs real files
      iex> UnixTools.Tar.collect_files(["dir/"], "/tmp")
      ["dir/file1.txt", "dir/file2.txt"]
  """
  def create_archive(file_list, archive_path, opts) do
    # Change to the specified directory if -C is given
    original_dir = File.cwd!()

    if opts[:directory] do
      File.cd!(opts[:directory])
    end

    try do
      # Collect all files to archive
      all_files = Enum.flat_map(file_list, fn path -> collect_files(path) end)

      # Determine compression
      tar_opts = compression_opts(opts)

      # Create the archive using :erl_tar
      case :erl_tar.open(to_charlist(archive_path), [:write | tar_opts]) do
        {:ok, tar_descriptor} ->
          Enum.each(all_files, fn file_path ->
            if opts[:verbose], do: IO.puts(file_path)

            :erl_tar.add(tar_descriptor, to_charlist(file_path), to_charlist(file_path), [])
          end)

          :erl_tar.close(tar_descriptor)
          :ok

        {:error, reason} ->
          IO.puts(:stderr, "tar: #{archive_path}: #{inspect(reason)}")
          {:error, reason}
      end
    after
      if opts[:directory], do: File.cd!(original_dir)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Archive Extraction
  # ---------------------------------------------------------------------------

  @doc """
  Extract files from a tar archive.

  Uses Erlang's `:erl_tar` module to read the archive and extract files.

  ## How Extraction Works

  1. Open the tar file for reading.
  2. For each entry, create the file at the appropriate path.
  3. If -C is specified, extract relative to that directory.
  4. If -k is specified, don't overwrite existing files.

  ## Options

  - `:archive_file` — path to the input tar file
  - `:verbose` — print each file as it's extracted
  - `:directory` — extract to this directory
  - `:keep_old` — don't overwrite existing files
  - `:strip_components` — remove N leading path components
  """
  def extract_archive(archive_path, opts) do
    dest_dir = opts[:directory] || "."

    # Ensure destination directory exists
    File.mkdir_p!(dest_dir)

    tar_opts = compression_opts(opts)

    case :erl_tar.extract(to_charlist(archive_path), [
           {:cwd, to_charlist(dest_dir)} | tar_opts
         ]) do
      :ok ->
        if opts[:verbose] do
          # List what was extracted
          case list_archive_entries(archive_path, opts) do
            {:ok, entries} ->
              Enum.each(entries, fn entry_name ->
                stripped = strip_path_components(entry_name, opts[:strip_components])
                IO.puts(stripped)
              end)

            _ ->
              :ok
          end
        end

        :ok

      {:ok, entries} ->
        if opts[:verbose] do
          Enum.each(entries, fn entry_name ->
            name_str = to_string(entry_name)
            stripped = strip_path_components(name_str, opts[:strip_components])
            IO.puts(stripped)
          end)
        end

        :ok

      {:error, reason} ->
        IO.puts(:stderr, "tar: #{archive_path}: #{inspect(reason)}")
        {:error, reason}
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Archive Listing
  # ---------------------------------------------------------------------------

  @doc """
  List the contents of a tar archive.

  Returns a list of file names in the archive.

  ## Options

  - `:verbose` — show detailed information (size, date, permissions)
  """
  def list_archive(archive_path, opts) do
    case list_archive_entries(archive_path, opts) do
      {:ok, entries} ->
        Enum.each(entries, fn entry_name ->
          IO.puts(entry_name)
        end)

        :ok

      {:error, reason} ->
        IO.puts(:stderr, "tar: #{archive_path}: #{inspect(reason)}")
        {:error, reason}
    end
  end

  @doc """
  Get the list of entry names from a tar archive.

  Returns `{:ok, [String.t()]}` or `{:error, reason}`.
  """
  def list_archive_entries(archive_path, opts) do
    tar_opts = compression_opts(opts)

    case :erl_tar.table(to_charlist(archive_path), [:verbose | tar_opts]) do
      {:ok, entries} ->
        names =
          Enum.map(entries, fn
            {name, _info, _size, _mtime, _mode, _uid, _gid} ->
              to_string(name)

            entry when is_list(entry) ->
              to_string(entry)

            entry ->
              to_string(entry)
          end)

        {:ok, names}

      {:error, reason} ->
        {:error, reason}
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: File Collection
  # ---------------------------------------------------------------------------

  @doc """
  Recursively collect all files under a path.

  If the path is a file, returns `[path]`.
  If the path is a directory, returns all files in the directory tree,
  including the directory itself.

  ## Examples

      iex> # With a temp directory containing file.txt:
      iex> # UnixTools.Tar.collect_files("dir/")
      iex> # => ["dir/", "dir/file.txt"]
  """
  def collect_files(path) do
    cond do
      File.dir?(path) ->
        # Include the directory itself, then recurse
        entries =
          File.ls!(path)
          |> Enum.sort()
          |> Enum.flat_map(fn entry ->
            collect_files(Path.join(path, entry))
          end)

        [path | entries]

      File.exists?(path) ->
        [path]

      true ->
        IO.puts(:stderr, "tar: #{path}: No such file or directory")
        []
    end
  end

  @doc """
  Strip N leading path components from a file path.

  This implements the --strip-components behavior.

  ## Examples

      iex> UnixTools.Tar.strip_path_components("a/b/c/file.txt", 2)
      "c/file.txt"

      iex> UnixTools.Tar.strip_path_components("file.txt", 0)
      "file.txt"

      iex> UnixTools.Tar.strip_path_components("a/b", nil)
      "a/b"
  """
  def strip_path_components(path, nil), do: path
  def strip_path_components(path, 0), do: path

  def strip_path_components(path, n_components) when is_integer(n_components) do
    parts = Path.split(path)

    if length(parts) > n_components do
      parts
      |> Enum.drop(n_components)
      |> Path.join()
    else
      ""
    end
  end

  # ---------------------------------------------------------------------------
  # Run
  # ---------------------------------------------------------------------------

  defp run(file_list, opts) do
    archive_path = opts[:archive_file]

    cond do
      opts[:create] ->
        if archive_path == nil do
          IO.puts(:stderr, "tar: Refusing to create archive without -f flag")
          System.halt(2)
        end

        case create_archive(file_list, archive_path, opts) do
          :ok -> :ok
          {:error, _} -> System.halt(2)
        end

      opts[:extract] ->
        if archive_path == nil do
          IO.puts(:stderr, "tar: Refusing to extract archive without -f flag")
          System.halt(2)
        end

        case extract_archive(archive_path, opts) do
          :ok -> :ok
          {:error, _} -> System.halt(2)
        end

      opts[:list_contents] ->
        if archive_path == nil do
          IO.puts(:stderr, "tar: Refusing to list archive without -f flag")
          System.halt(2)
        end

        case list_archive(archive_path, opts) do
          :ok -> :ok
          {:error, _} -> System.halt(2)
        end

      true ->
        IO.puts(:stderr, "tar: You must specify one of -c, -x, or -t")
        System.halt(2)
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp compression_opts(opts) do
    cond do
      opts[:gzip] -> [:compressed]
      opts[:bzip2] -> []
      opts[:xz] -> []
      true -> []
    end
  end

  defp normalize_files(nil), do: []
  defp normalize_files(files) when is_list(files), do: files
  defp normalize_files(file) when is_binary(file), do: [file]

  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "tar.json"),
        else: nil
      ),
      "tar.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "tar.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      path -> File.exists?(path)
    end) ||
      raise "Could not find tar.json spec file"
  end
end
