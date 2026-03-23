defmodule UnixTools.Sort do
  @moduledoc """
  sort -- sort lines of text files.

  ## What This Program Does

  This is a reimplementation of the GNU `sort` utility in Elixir. It reads
  lines from files (or stdin) and writes them in sorted order to stdout.

  ## How sort Works

  At its simplest:

      sort file.txt    =>   lines of file.txt in alphabetical order
      sort             =>   reads stdin, sorts, outputs

  The default sort is lexicographic (dictionary order by Unicode code points).

  ## Sort Modes

  GNU sort supports many comparison modes:

  | Flag | Mode               | Example                          |
  |------|--------------------|----------------------------------|
  | (none)| Lexicographic     | "apple" < "banana" < "cherry"    |
  | -n   | Numeric            | "2" < "10" < "100"               |
  | -h   | Human-numeric      | "1K" < "2M" < "3G"              |
  | -M   | Month              | "JAN" < "FEB" < "MAR"           |
  | -g   | General numeric    | "1e2" < "1e3" (scientific)       |
  | -V   | Version            | "1.2" < "1.10" < "2.0"          |

  ## Modifier Flags

  - `-r` (--reverse):         Reverse sort order.
  - `-u` (--unique):          Remove duplicate lines (keep first occurrence).
  - `-f` (--ignore-case):     Fold lowercase to uppercase for comparison.
  - `-d` (--dictionary-order): Only consider blanks and alphanumeric chars.
  - `-b` (--ignore-leading-blanks): Ignore leading whitespace.
  - `-s` (--stable):          Preserve original order for equal elements.

  ## Implementation Approach

  We implement sort as a pipeline of pure functions:

  1. `sort_lines/2` takes a list of strings and an options map.
  2. A key-extraction function is built from the options.
  3. Lines are sorted using `Enum.sort_by/3` with the key function.
  4. Post-processing (unique, reverse) is applied.

  This keeps the business logic testable without file I/O.
  """

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Business Logic
  # ---------------------------------------------------------------------------

  @doc """
  Sort a list of lines according to the given options.

  ## Options

  - `:reverse` (boolean) - Reverse the sort order.
  - `:numeric` (boolean) - Sort by numeric value.
  - `:human_numeric` (boolean) - Sort by human-readable sizes (1K, 2M, 3G).
  - `:month` (boolean) - Sort by month abbreviation.
  - `:general_numeric` (boolean) - Sort by general numeric value (floats, sci).
  - `:version` (boolean) - Version-number sort.
  - `:unique` (boolean) - Remove duplicates after sorting.
  - `:ignore_case` (boolean) - Case-insensitive comparison.
  - `:dictionary_order` (boolean) - Only consider blanks and alphanumeric.
  - `:ignore_leading_blanks` (boolean) - Strip leading whitespace before comparing.

  ## How the Key Function Works

  We build a "sort key" for each line. The key depends on the sort mode:

  - **Lexicographic**: the line itself (possibly downcased, stripped).
  - **Numeric**: parse the line as an integer/float.
  - **Human-numeric**: parse "2K" as 2048, "3M" as 3145728, etc.
  - **Month**: map "JAN" -> 1, "FEB" -> 2, ..., "DEC" -> 12.

  Then `Enum.sort_by/3` sorts by these keys.

  ## Examples

      iex> UnixTools.Sort.sort_lines(["banana", "apple", "cherry"])
      ["apple", "banana", "cherry"]

      iex> UnixTools.Sort.sort_lines(["10", "2", "1"], %{numeric: true})
      ["1", "2", "10"]
  """
  def sort_lines(lines, opts \\ %{}) do
    key_fn = build_key_function(opts)

    sorted =
      if opts[:stable] do
        # Stable sort preserves original order for equal elements.
        # Elixir's Enum.sort_by is already stable, so this is the default.
        Enum.sort_by(lines, key_fn)
      else
        Enum.sort_by(lines, key_fn)
      end

    sorted = if opts[:reverse], do: Enum.reverse(sorted), else: sorted
    sorted = if opts[:unique], do: Enum.uniq_by(sorted, key_fn), else: sorted

    sorted
  end

  @doc """
  Check whether lines are already sorted according to the given options.

  Returns `:ok` if sorted, or `{:error, line_number, line}` for the first
  out-of-order line.

  ## How Sorted Checking Works

  We compare adjacent pairs using the same key function used for sorting.
  The first pair where `key(line_n) > key(line_n+1)` is reported.
  """
  def check_sorted(lines, opts \\ %{}) do
    key_fn = build_key_function(opts)

    lines
    |> Enum.with_index(1)
    |> Enum.reduce_while(:ok, fn {line, idx}, _acc ->
      if idx == 1 do
        {:cont, {:prev, key_fn.(line), idx}}
      else
        {:cont, {:prev, key_fn.(line), idx}}
      end
    end)

    # Simpler approach: check pairs
    result =
      lines
      |> Enum.with_index(1)
      |> Enum.chunk_every(2, 1, :discard)
      |> Enum.find(fn [{line_a, _idx_a}, {line_b, _idx_b}] ->
        key_a = key_fn.(line_a)
        key_b = key_fn.(line_b)

        if opts[:reverse] do
          key_a < key_b
        else
          key_a > key_b
        end
      end)

    case result do
      nil -> :ok
      [{_line_a, _idx_a}, {line_b, idx_b}] -> {:error, idx_b, line_b}
    end
  end

  # ---------------------------------------------------------------------------
  # Key Function Builder
  # ---------------------------------------------------------------------------

  @doc false
  defp build_key_function(opts) do
    fn line ->
      # Step 1: Optionally strip leading blanks.
      key = if opts[:ignore_leading_blanks], do: String.trim_leading(line), else: line

      # Step 2: Optionally restrict to dictionary characters.
      key =
        if opts[:dictionary_order] do
          String.replace(key, ~r/[^[:alnum:]\s]/, "")
        else
          key
        end

      # Step 3: Optionally fold case.
      key = if opts[:ignore_case], do: String.downcase(key), else: key

      # Step 4: Apply sort mode.
      cond do
        opts[:numeric] -> parse_numeric(key)
        opts[:human_numeric] -> parse_human_numeric(key)
        opts[:month] -> parse_month(key)
        opts[:general_numeric] -> parse_general_numeric(key)
        opts[:version] -> parse_version(key)
        true -> key
      end
    end
  end

  # ---------------------------------------------------------------------------
  # Numeric Parsers
  # ---------------------------------------------------------------------------

  @doc """
  Parse a string as a numeric value for -n (numeric sort).

  Leading whitespace is ignored. The first numeric portion is extracted.
  Non-numeric strings sort as 0 (matching GNU sort behavior).

  ## Examples

      iex> UnixTools.Sort.parse_numeric("42")
      42.0

      iex> UnixTools.Sort.parse_numeric("  -3.14")
      -3.14

      iex> UnixTools.Sort.parse_numeric("abc")
      0.0
  """
  def parse_numeric(str) do
    trimmed = String.trim_leading(str)

    case Float.parse(trimmed) do
      {num, _rest} -> num
      :error ->
        case Integer.parse(trimmed) do
          {num, _rest} -> num * 1.0
          :error -> 0.0
        end
    end
  end

  @doc """
  Parse human-readable sizes like "1K", "2M", "3G" for -h sort.

  ## Size Suffixes

  | Suffix | Multiplier |
  |--------|-----------|
  | K      | 1024      |
  | M      | 1024^2    |
  | G      | 1024^3    |
  | T      | 1024^4    |
  | P      | 1024^5    |
  | E      | 1024^6    |

  ## Examples

      iex> UnixTools.Sort.parse_human_numeric("2K")
      2048.0

      iex> UnixTools.Sort.parse_human_numeric("1M")
      1048576.0
  """
  def parse_human_numeric(str) do
    trimmed = String.trim(str)

    suffixes = %{
      "K" => 1024,
      "M" => 1024 * 1024,
      "G" => 1024 * 1024 * 1024,
      "T" => 1024 * 1024 * 1024 * 1024,
      "P" => 1024 * 1024 * 1024 * 1024 * 1024,
      "E" => 1024 * 1024 * 1024 * 1024 * 1024 * 1024
    }

    # Try to match a number followed by an optional suffix.
    case Regex.run(~r/^([+-]?\d+\.?\d*)\s*([KMGTPE])?$/i, trimmed) do
      [_full, num_str, suffix] ->
        base = parse_numeric(num_str)
        multiplier = Map.get(suffixes, String.upcase(suffix), 1)
        base * multiplier

      [_full, num_str] ->
        parse_numeric(num_str)

      nil ->
        0.0
    end
  end

  @month_map %{
    "JAN" => 1, "FEB" => 2, "MAR" => 3, "APR" => 4,
    "MAY" => 5, "JUN" => 6, "JUL" => 7, "AUG" => 8,
    "SEP" => 9, "OCT" => 10, "NOV" => 11, "DEC" => 12
  }

  @doc """
  Parse a month abbreviation for -M sort.

  Maps three-letter month abbreviations to their ordinal position.
  Unknown months sort before January (as 0).

  ## Examples

      iex> UnixTools.Sort.parse_month("JAN")
      1

      iex> UnixTools.Sort.parse_month("unknown")
      0
  """
  def parse_month(str) do
    key = str |> String.trim() |> String.upcase() |> String.slice(0, 3)
    Map.get(@month_map, key, 0)
  end

  @doc """
  Parse a general numeric value for -g sort.

  Handles scientific notation like "1e3", "2.5E-4", infinity, and NaN.
  Non-numeric strings sort as negative infinity (before all numbers).
  """
  def parse_general_numeric(str) do
    trimmed = String.trim(str)

    case Float.parse(trimmed) do
      {num, _rest} -> num
      :error ->
        case Integer.parse(trimmed) do
          {num, _rest} -> num * 1.0
          :error -> -1.0e308
        end
    end
  end

  @doc """
  Parse a version string for -V sort.

  Splits the string into segments of digits and non-digits.
  Digit segments are compared numerically; non-digit segments lexicographically.
  This produces "natural" ordering: "file2" < "file10".

  ## Examples

      iex> UnixTools.Sort.parse_version("file10")
      ["file", 10]

      iex> UnixTools.Sort.parse_version("1.2.3")
      [1, ".", 2, ".", 3]
  """
  def parse_version(str) do
    # Split into alternating digit/non-digit segments.
    Regex.scan(~r/\d+|[^\d]+/, str)
    |> List.flatten()
    |> Enum.map(fn segment ->
      case Integer.parse(segment) do
        {num, ""} -> num
        _ -> segment
      end
    end)
  end

  # ---------------------------------------------------------------------------
  # Entry Point
  # ---------------------------------------------------------------------------

  @doc """
  Entry point. Receives `argv` as a list of strings.
  """
  def main(argv) do
    spec_path = resolve_spec_path()

    case Parser.parse(spec_path, ["sort" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        opts = %{
          reverse: !!flags["reverse"],
          numeric: !!flags["numeric_sort"],
          human_numeric: !!flags["human_numeric_sort"],
          month: !!flags["month_sort"],
          general_numeric: !!flags["general_numeric_sort"],
          version: !!flags["version_sort"],
          unique: !!flags["unique"],
          ignore_case: !!flags["ignore_case"],
          dictionary_order: !!flags["dictionary_order"],
          ignore_leading_blanks: !!flags["ignore_leading_blanks"],
          stable: !!flags["stable"],
          check: !!flags["check"]
        }

        file_list = normalize_files(arguments["files"])
        lines = read_all_lines(file_list)

        if opts[:check] do
          case check_sorted(lines, opts) do
            :ok -> :ok
            {:error, _line_num, line} ->
              IO.puts(:stderr, "sort: -:#{line}: disorder: #{line}")
              System.halt(1)
          end
        else
          sorted = sort_lines(lines, opts)
          Enum.each(sorted, &IO.puts/1)
        end

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "sort: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  @doc false
  defp read_all_lines(file_list) do
    Enum.flat_map(file_list, fn
      "-" ->
        case IO.read(:stdio, :eof) do
          {:error, _} -> []
          :eof -> []
          data -> String.split(data, "\n", trim: true)
        end

      file_path ->
        case File.read(file_path) do
          {:ok, content} -> String.split(content, "\n", trim: true)
          {:error, reason} ->
            IO.puts(:stderr, "sort: #{file_path}: #{:file.format_error(reason)}")
            []
        end
    end)
  end

  @doc false
  defp normalize_files(nil), do: ["-"]
  defp normalize_files(files) when is_list(files), do: files
  defp normalize_files(file) when is_binary(file), do: [file]

  @doc false
  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "sort.json"),
        else: nil
      ),
      "sort.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "sort.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find sort.json spec file"
  end
end
