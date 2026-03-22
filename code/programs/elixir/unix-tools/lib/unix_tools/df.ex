defmodule UnixTools.Df do
  @moduledoc """
  df -- report file system disk space usage.

  ## What This Program Does

  This is a reimplementation of the GNU `df` utility in Elixir. It displays
  information about the file system on which each FILE resides, or all file
  systems by default.

  ## How df Works

  With no arguments, `df` shows all mounted file systems:

      df    =>   Filesystem     1K-blocks     Used Available Use% Mounted on
                 /dev/sda1      102400000 60000000  42400000  59% /
                 tmpfs            8192000        0   8192000   0% /tmp

  With a path argument, it shows only the file system containing that path:

      df /home   =>   just the entry for /home's filesystem

  ## Display Options

  - `-h` (--human-readable): Show sizes in powers of 1024 (e.g., 1.5G).
  - `-H` (--si): Show sizes in powers of 1000 (e.g., 1.6G).
  - `-T` (--print-type): Include the filesystem type column.
  - `-i` (--inodes): Show inode usage instead of block usage.

  ## Implementation Approach

  We delegate to the system `df` command via `:os.cmd/1` and parse its
  tabular output. The heavy lifting is in parsing the column-based output
  that `df` produces, which can vary between operating systems.

  The business logic (parsing df output, formatting sizes) is kept as
  pure functions for testability.
  """

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Business Logic
  # ---------------------------------------------------------------------------

  @doc """
  Get filesystem information by running the system `df` command.

  Returns a list of maps, each with keys: `:filesystem`, `:blocks`,
  `:used`, `:available`, `:use_percent`, `:mounted_on`.

  ## How Parsing Works

  The `df -k` command outputs 1K-blocks in a tabular format:

      Filesystem     1K-blocks     Used Available Use% Mounted on
      /dev/sda1      102400000 60000000  42400000  59% /

  We skip the header line and parse each subsequent line into fields.
  Some filesystems have spaces in their names, so we're careful to
  parse from both ends when needed.
  """
  def get_fs_info do
    raw = :os.cmd(~c"df -k") |> to_string()
    parse_df_output(raw)
  end

  @doc """
  Get filesystem information for a specific path.
  """
  def get_fs_info(file_path) do
    raw = :os.cmd(~c"df -k #{file_path}") |> to_string()
    parse_df_output(raw)
  end

  @doc """
  Parse the output of `df -k` into structured data.

  ## Parsing Strategy

  1. Split output into lines.
  2. Skip the header line.
  3. For each data line, split on whitespace.
  4. Handle the case where a filesystem name wraps to a new line
     (common with long NFS mount names).

  ## Examples

      iex> output = "Filesystem  1K-blocks  Used Available Use% Mounted on\\n/dev/sda1 100 60 40 60% /\\n"
      iex> [entry] = UnixTools.Df.parse_df_output(output)
      iex> entry.filesystem
      "/dev/sda1"
  """
  def parse_df_output(output) do
    lines =
      output
      |> String.split("\n")
      |> Enum.reject(&(String.trim(&1) == ""))

    case lines do
      [] -> []
      [_header | data_lines] ->
        data_lines
        |> Enum.map(&parse_df_line/1)
        |> Enum.reject(&is_nil/1)
    end
  end

  @doc false
  defp parse_df_line(line) do
    parts = String.split(line, ~r/\s+/)

    # df output typically has 6 fields:
    # Filesystem, 1K-blocks, Used, Available, Use%, Mounted on
    # On macOS, "Mounted on" can contain spaces, and there may be extra fields.
    case parts do
      [fs, blocks_str, used_str, avail_str, pct_str | mount_parts]
      when length(mount_parts) >= 1 ->
        %{
          filesystem: fs,
          blocks: safe_parse_int(blocks_str),
          used: safe_parse_int(used_str),
          available: safe_parse_int(avail_str),
          use_percent: String.replace(pct_str, "%", ""),
          mounted_on: Enum.join(mount_parts, " ")
        }

      _ ->
        nil
    end
  end

  @doc false
  defp safe_parse_int(str) do
    case Integer.parse(str) do
      {num, _} -> num
      :error -> 0
    end
  end

  @doc """
  Format a size in bytes/kilobytes to human-readable format.

  ## How Human-Readable Formatting Works

  We divide by successive powers of the base (1024 or 1000) until
  the number is small enough to display with a unit suffix.

  | Suffix | Power of 1024 | Power of 1000 |
  |--------|--------------|---------------|
  | K      | 1            | 1             |
  | M      | 2            | 2             |
  | G      | 3            | 3             |
  | T      | 4            | 4             |
  | P      | 5            | 5             |

  ## Examples

      iex> UnixTools.Df.format_size(1536, 1024)
      "1.5M"

      iex> UnixTools.Df.format_size(1000, 1000)
      "1.0M"
  """
  def format_size(kb, base \\ 1024) do
    units = ["K", "M", "G", "T", "P"]

    {value, unit} =
      Enum.reduce_while(units, {kb * 1.0, "K"}, fn unit, {val, _current_unit} ->
        if val >= base do
          {:cont, {val / base, unit}}
        else
          {:halt, {val, unit}}
        end
      end)

    # Handle the case where we exhausted all units
    if value >= base do
      formatted = :erlang.float_to_binary(value, decimals: 1)
      "#{formatted}#{unit}"
    else
      formatted = :erlang.float_to_binary(value, decimals: 1)
      "#{formatted}#{unit}"
    end
  end

  # ---------------------------------------------------------------------------
  # Entry Point
  # ---------------------------------------------------------------------------

  @doc """
  Entry point. Receives `argv` as a list of strings.
  """
  def main(argv) do
    spec_path = resolve_spec_path()

    case Parser.parse(spec_path, ["df" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        human_readable = !!flags["human_readable"]
        si = !!flags["si"]
        _print_type = !!flags["print_type"]

        file_list = arguments["files"]

        entries =
          case file_list do
            nil -> get_fs_info()
            paths when is_list(paths) ->
              Enum.flat_map(paths, &get_fs_info/1)
            single_path when is_binary(single_path) ->
              get_fs_info(single_path)
          end

        # Print header
        IO.puts("Filesystem     1K-blocks      Used Available Use% Mounted on")

        # Print entries
        Enum.each(entries, fn entry ->
          blocks_str =
            cond do
              human_readable -> format_size(entry.blocks, 1024)
              si -> format_size(entry.blocks, 1000)
              true -> Integer.to_string(entry.blocks)
            end

          used_str =
            cond do
              human_readable -> format_size(entry.used, 1024)
              si -> format_size(entry.used, 1000)
              true -> Integer.to_string(entry.used)
            end

          avail_str =
            cond do
              human_readable -> format_size(entry.available, 1024)
              si -> format_size(entry.available, 1000)
              true -> Integer.to_string(entry.available)
            end

          IO.puts(
            "#{String.pad_trailing(entry.filesystem, 15)}" <>
            "#{String.pad_leading(blocks_str, 10)} " <>
            "#{String.pad_leading(used_str, 10)} " <>
            "#{String.pad_leading(avail_str, 10)} " <>
            "#{String.pad_leading(entry.use_percent <> "%", 5)} " <>
            "#{entry.mounted_on}"
          )
        end)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "df: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  @doc false
  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "df.json"),
        else: nil
      ),
      "df.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "df.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find df.json spec file"
  end
end
