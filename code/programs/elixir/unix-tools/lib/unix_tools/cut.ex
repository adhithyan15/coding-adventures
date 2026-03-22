defmodule UnixTools.Cut do
  @moduledoc """
  cut -- remove sections from each line of files.

  ## What This Program Does

  This is a reimplementation of the GNU `cut` utility in Elixir. It selects
  portions of each line from files and writes them to stdout.

  ## How cut Works

  `cut` has three selection modes, exactly one of which must be specified:

  | Flag | Mode       | Description                         |
  |------|-----------|-------------------------------------|
  | -b   | Bytes     | Select specific byte positions       |
  | -c   | Characters| Select specific character positions  |
  | -f   | Fields    | Select specific fields (delimited)   |

  ## Range List Syntax

  The LIST argument specifies which bytes/characters/fields to select:

  | Pattern | Meaning                              |
  |---------|--------------------------------------|
  | N       | The Nth element (1-based)            |
  | N-M     | From N through M (inclusive)         |
  | N-      | From N through end of line           |
  | -M      | From start through M                 |
  | N,M     | Multiple selections (comma-separated)|

  ## Examples

      echo "hello:world:foo" | cut -d: -f2       =>   world
      echo "hello:world:foo" | cut -d: -f1,3     =>   hello:foo
      echo "abcdef" | cut -c1-3                   =>   abc
      echo "abcdef" | cut -c3-                    =>   cdef

  ## Field Mode Details

  In field mode (-f), `cut` splits each line by a delimiter (default: TAB).
  The `-s` flag suppresses lines that don't contain the delimiter at all.
  """

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Business Logic
  # ---------------------------------------------------------------------------

  @doc """
  Parse a range list string into a list of `{start, stop}` tuples.

  The range list syntax is: `N`, `N-M`, `N-`, `-M`, comma-separated.
  All positions are 1-based. An open end is represented by `:infinity`.

  ## How Parsing Works

  We split on commas, then parse each segment:

  - "5"    -> {5, 5}       (single position)
  - "2-7"  -> {2, 7}       (closed range)
  - "3-"   -> {3, :infinity} (open-ended)
  - "-4"   -> {1, 4}       (from start)

  ## Examples

      iex> UnixTools.Cut.parse_ranges("1-3,5,7-")
      [{1, 3}, {5, 5}, {7, :infinity}]
  """
  def parse_ranges(range_str) do
    range_str
    |> String.split(",")
    |> Enum.map(&parse_single_range/1)
    |> Enum.sort_by(fn {start, _stop} -> start end)
  end

  @doc false
  defp parse_single_range(str) do
    trimmed = String.trim(str)

    cond do
      # Open-ended: "N-"
      String.ends_with?(trimmed, "-") and String.length(trimmed) > 1 ->
        start = trimmed |> String.trim_trailing("-") |> String.to_integer()
        {start, :infinity}

      # From start: "-M"
      String.starts_with?(trimmed, "-") ->
        stop = trimmed |> String.trim_leading("-") |> String.to_integer()
        {1, stop}

      # Closed range: "N-M"
      String.contains?(trimmed, "-") ->
        [start_str, stop_str] = String.split(trimmed, "-", parts: 2)
        {String.to_integer(start_str), String.to_integer(stop_str)}

      # Single position: "N"
      true ->
        n = String.to_integer(trimmed)
        {n, n}
    end
  end

  @doc """
  Check if a 1-based position is included in the given ranges.

  ## How Position Checking Works

  We iterate through each range and check if the position falls within
  any of them. Since ranges can include `:infinity`, we handle that
  case specially.

  ## Examples

      iex> UnixTools.Cut.position_included?(3, [{1, 5}])
      true

      iex> UnixTools.Cut.position_included?(6, [{1, 3}, {7, :infinity}])
      false
  """
  def position_included?(pos, ranges) do
    Enum.any?(ranges, fn
      {start, :infinity} -> pos >= start
      {start, stop} -> pos >= start and pos <= stop
    end)
  end

  @doc """
  Cut a single line by byte positions.

  Selects the bytes at the positions specified by ranges.

  ## Examples

      iex> UnixTools.Cut.cut_bytes("abcdef", [{1, 3}])
      "abc"
  """
  def cut_bytes(line, ranges, opts \\ %{}) do
    complement = !!opts[:complement]
    bytes = :binary.bin_to_list(line)

    selected =
      bytes
      |> Enum.with_index(1)
      |> Enum.filter(fn {_byte, idx} ->
        included = position_included?(idx, ranges)
        if complement, do: not included, else: included
      end)
      |> Enum.map(fn {byte, _idx} -> byte end)

    output_delim = opts[:output_delimiter]

    if output_delim do
      # Group consecutive selected positions and join with output delimiter.
      group_and_join(selected, output_delim)
    else
      :binary.list_to_bin(selected)
    end
  end

  @doc """
  Cut a single line by character positions.

  In Elixir, strings are UTF-8, so characters and bytes may differ.
  We use `String.graphemes/1` to split into user-perceived characters.

  ## Examples

      iex> UnixTools.Cut.cut_characters("abcdef", [{2, 4}])
      "bcd"
  """
  def cut_characters(line, ranges, opts \\ %{}) do
    complement = !!opts[:complement]
    chars = String.graphemes(line)

    selected =
      chars
      |> Enum.with_index(1)
      |> Enum.filter(fn {_char, idx} ->
        included = position_included?(idx, ranges)
        if complement, do: not included, else: included
      end)
      |> Enum.map(fn {char, _idx} -> char end)

    Enum.join(selected)
  end

  @doc """
  Cut a single line by field positions.

  Splits the line by the delimiter and selects the fields at the
  given positions.

  ## Options

  - `:delimiter` - Field delimiter (default: "\\t").
  - `:only_delimited` - If true, suppress lines without the delimiter.
  - `:output_delimiter` - Use this string between output fields.
  - `:complement` - Select fields NOT in the ranges.

  ## Examples

      iex> UnixTools.Cut.cut_fields("a:b:c:d", [{2, 3}], %{delimiter: ":"})
      "b:c"
  """
  def cut_fields(line, ranges, opts \\ %{}) do
    delimiter = opts[:delimiter] || "\t"
    output_delim = opts[:output_delimiter] || delimiter
    only_delimited = !!opts[:only_delimited]
    complement = !!opts[:complement]

    # If the line doesn't contain the delimiter, handle -s flag.
    unless String.contains?(line, delimiter) do
      if only_delimited do
        return_nil()
      else
        return_val(line)
      end
    else
      fields = String.split(line, delimiter)

      selected =
        fields
        |> Enum.with_index(1)
        |> Enum.filter(fn {_field, idx} ->
          included = position_included?(idx, ranges)
          if complement, do: not included, else: included
        end)
        |> Enum.map(fn {field, _idx} -> field end)

      {:ok, Enum.join(selected, output_delim)}
    end
  end

  # Helper to return nil for suppressed lines
  defp return_nil, do: :suppress
  defp return_val(val), do: {:ok, val}

  @doc false
  defp group_and_join(bytes, _delim) do
    :binary.list_to_bin(bytes)
  end

  # ---------------------------------------------------------------------------
  # Entry Point
  # ---------------------------------------------------------------------------

  @doc """
  Entry point. Receives `argv` as a list of strings.
  """
  def main(argv) do
    spec_path = resolve_spec_path()

    case Parser.parse(spec_path, ["cut" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        opts = %{
          delimiter: flags["delimiter"],
          only_delimited: !!flags["only_delimited"],
          output_delimiter: flags["output_delimiter"],
          complement: !!flags["complement"]
        }

        # Determine mode and ranges.
        {mode, ranges} =
          cond do
            flags["bytes"] -> {:bytes, parse_ranges(flags["bytes"])}
            flags["characters"] -> {:characters, parse_ranges(flags["characters"])}
            flags["fields"] -> {:fields, parse_ranges(flags["fields"])}
            true ->
              IO.puts(:stderr, "cut: you must specify a list of bytes, characters, or fields")
              System.halt(1)
          end

        file_list = normalize_files(arguments["files"])

        Enum.each(file_list, fn file_path ->
          lines = read_lines(file_path)

          Enum.each(lines, fn line ->
            result =
              case mode do
                :bytes -> {:ok, cut_bytes(line, ranges, opts)}
                :characters -> {:ok, cut_characters(line, ranges, opts)}
                :fields -> cut_fields(line, ranges, opts)
              end

            case result do
              {:ok, output} -> IO.puts(output)
              :suppress -> :ok
            end
          end)
        end)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "cut: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  @doc false
  defp read_lines("-") do
    case IO.read(:stdio, :eof) do
      {:error, _} -> []
      :eof -> []
      data -> String.split(data, "\n", trim: true)
    end
  end

  defp read_lines(file_path) do
    case File.read(file_path) do
      {:ok, content} -> String.split(content, "\n", trim: true)
      {:error, reason} ->
        IO.puts(:stderr, "cut: #{file_path}: #{:file.format_error(reason)}")
        []
    end
  end

  @doc false
  defp normalize_files(nil), do: ["-"]
  defp normalize_files(files) when is_list(files), do: files
  defp normalize_files(file) when is_binary(file), do: [file]

  @doc false
  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "cut.json"),
        else: nil
      ),
      "cut.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "cut.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find cut.json spec file"
  end
end
