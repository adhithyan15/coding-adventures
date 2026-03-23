defmodule UnixTools.Du do
  @moduledoc """
  du -- estimate file space usage.

  ## What This Program Does

  This is a reimplementation of the GNU `du` utility in Elixir. It estimates
  and reports the disk space used by files and directories.

  ## How du Works

  By default, `du` reports the disk usage of each directory recursively:

      du /home/user     =>   4096    /home/user/docs
                              8192    /home/user/photos
                              16384   /home/user

  The numbers are in 1K blocks by default.

  ## Key Flags

  - `-a` (--all): Show sizes for files too, not just directories.
  - `-s` (--summarize): Only show the total for each argument.
  - `-h` (--human-readable): Print sizes in human-readable format (1K, 2M, 3G).
  - `-c` (--total): Print a grand total at the end.
  - `-d N` (--max-depth=N): Only show directories N levels deep.

  ## How Recursive Measurement Works

  1. Start at the given path.
  2. If it's a regular file: its size is `File.stat!/1 |> .size`.
  3. If it's a directory:
     a. List all entries with `File.ls!/1`.
     b. Recursively measure each entry.
     c. The directory's total = sum of all entries' sizes.
  4. Convert bytes to 1K blocks (divide by 1024, round up).

  ## Implementation Approach

  We use `File.stat!/1` and `File.ls!/1` from Elixir's standard library.
  This gives us file sizes in bytes, which we convert to 1K blocks to
  match the default `du` output format.

  The core logic is a recursive function `disk_usage/2` that returns
  a list of `{path, size_in_bytes}` tuples.
  """

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Business Logic
  # ---------------------------------------------------------------------------

  @doc """
  Calculate disk usage for a path.

  Returns a list of `{path, size_in_bytes}` tuples. By default, only
  directory entries are included. With `show_all: true`, file entries
  are included too.

  ## Options

  - `:show_all` (boolean) - Include files in output, not just directories.
  - `:max_depth` (integer or nil) - Maximum depth to report.

  ## How the Recursion Works

  For a directory tree like:

      project/
        README.md    (1000 bytes)
        src/
          main.ex    (2000 bytes)

  The function produces:

      [{"project/src", 2000}, {"project", 3000}]

  With `show_all: true`:

      [{"project/src/main.ex", 2000}, {"project/src", 2000},
       {"project/README.md", 1000}, {"project", 3000}]

  ## Examples

      iex> [{path, size}] = UnixTools.Du.disk_usage("/tmp/testfile")
      iex> is_binary(path) and is_integer(size)
      true
  """
  def disk_usage(file_path, opts \\ %{}) do
    show_all = !!opts[:show_all]
    max_depth = opts[:max_depth]

    do_disk_usage(file_path, show_all, max_depth, 0)
  end

  @doc false
  defp do_disk_usage(file_path, show_all, max_depth, current_depth) do
    case File.stat(file_path) do
      {:ok, %File.Stat{type: :regular, size: size}} ->
        if show_all do
          [{file_path, size}]
        else
          # Files are counted in parent's total but not reported individually.
          [{file_path, size}]
        end

      {:ok, %File.Stat{type: :directory}} ->
        case File.ls(file_path) do
          {:ok, entries} ->
            # Recursively measure each entry.
            child_results =
              Enum.flat_map(entries, fn entry ->
                child_path = Path.join(file_path, entry)
                do_disk_usage(child_path, show_all, max_depth, current_depth + 1)
              end)

            # Calculate total size for this directory.
            total_size =
              child_results
              |> Enum.map(fn {_path, size} -> size end)
              |> Enum.sum()

            # Filter which entries to report based on depth and show_all.
            reported =
              child_results
              |> Enum.filter(fn {path, _size} ->
                child_stat = File.stat(path)

                case child_stat do
                  {:ok, %File.Stat{type: :directory}} ->
                    within_depth?(current_depth + 1, max_depth)

                  {:ok, %File.Stat{type: :regular}} ->
                    show_all and within_depth?(current_depth + 1, max_depth)

                  _ ->
                    false
                end
              end)

            # Add this directory itself if within depth.
            if within_depth?(current_depth, max_depth) do
              reported ++ [{file_path, total_size}]
            else
              reported
            end

          {:error, reason} ->
            IO.puts(:stderr, "du: cannot read directory '#{file_path}': #{:file.format_error(reason)}")
            []
        end

      {:ok, %File.Stat{type: :symlink, size: size}} ->
        [{file_path, size}]

      {:error, reason} ->
        IO.puts(:stderr, "du: cannot access '#{file_path}': #{:file.format_error(reason)}")
        []
    end
  end

  @doc false
  defp within_depth?(_current, nil), do: true
  defp within_depth?(current, max), do: current <= max

  @doc """
  Convert bytes to 1K blocks (ceiling division).

  This matches how `du` reports sizes -- a 1-byte file uses 1 block,
  a 1025-byte file uses 2 blocks.

  ## Examples

      iex> UnixTools.Du.bytes_to_blocks(0)
      0

      iex> UnixTools.Du.bytes_to_blocks(1024)
      1

      iex> UnixTools.Du.bytes_to_blocks(1025)
      2
  """
  def bytes_to_blocks(bytes) when bytes <= 0, do: 0
  def bytes_to_blocks(bytes), do: div(bytes + 1023, 1024)

  @doc """
  Format a size in 1K blocks to human-readable format.

  ## Examples

      iex> UnixTools.Du.format_human(1024)
      "1.0M"

      iex> UnixTools.Du.format_human(4)
      "4.0K"
  """
  def format_human(kb, base \\ 1024) do
    units = ["K", "M", "G", "T", "P"]

    {value, unit} =
      Enum.reduce_while(units, {kb * 1.0, "K"}, fn unit, {val, _current_unit} ->
        if val >= base do
          {:cont, {val / base, unit}}
        else
          {:halt, {val, unit}}
        end
      end)

    formatted = :erlang.float_to_binary(value, decimals: 1)
    "#{formatted}#{unit}"
  end

  # ---------------------------------------------------------------------------
  # Entry Point
  # ---------------------------------------------------------------------------

  @doc """
  Entry point. Receives `argv` as a list of strings.
  """
  def main(argv) do
    spec_path = resolve_spec_path()

    case Parser.parse(spec_path, ["du" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        opts = %{
          show_all: !!flags["all"],
          max_depth: flags["max_depth"],
          summarize: !!flags["summarize"],
          human_readable: !!flags["human_readable"],
          si: !!flags["si"],
          total: !!flags["total"]
        }

        # If summarize, set max_depth to 0.
        opts =
          if opts[:summarize] do
            Map.put(opts, :max_depth, 0)
          else
            opts
          end

        file_list = normalize_files(arguments["files"])

        all_results =
          Enum.flat_map(file_list, fn file_path ->
            disk_usage(file_path, opts)
          end)

        # Print results.
        Enum.each(all_results, fn {entry_path, size_bytes} ->
          kb = bytes_to_blocks(size_bytes)

          size_str =
            cond do
              opts[:human_readable] -> format_human(kb, 1024)
              opts[:si] -> format_human(kb, 1000)
              true -> Integer.to_string(kb)
            end

          IO.puts("#{size_str}\t#{entry_path}")
        end)

        # Grand total.
        if opts[:total] do
          grand_total =
            file_list
            |> Enum.map(fn file_path ->
              results = disk_usage(file_path, opts)
              case List.last(results) do
                {_p, s} -> s
                nil -> 0
              end
            end)
            |> Enum.sum()

          kb = bytes_to_blocks(grand_total)

          size_str =
            cond do
              opts[:human_readable] -> format_human(kb, 1024)
              opts[:si] -> format_human(kb, 1000)
              true -> Integer.to_string(kb)
            end

          IO.puts("#{size_str}\ttotal")
        end

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "du: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  @doc false
  defp normalize_files(nil), do: ["."]
  defp normalize_files(files) when is_list(files), do: files
  defp normalize_files(file) when is_binary(file), do: [file]

  @doc false
  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "du.json"),
        else: nil
      ),
      "du.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "du.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find du.json spec file"
  end
end
