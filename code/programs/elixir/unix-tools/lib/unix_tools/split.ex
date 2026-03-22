defmodule UnixTools.Split do
  @moduledoc """
  split -- split a file into pieces.

  ## What This Program Does

  This is a reimplementation of the GNU `split` utility in Elixir. It reads
  a file and splits it into smaller output files, each containing a specified
  number of lines or bytes.

  ## How split Works

  At its simplest:

      split largefile.txt              =>   creates xaa, xab, xac, ...
      split -l 100 largefile.txt       =>   100 lines per file
      split -b 1M largefile.txt        =>   1 megabyte per file

  ## Output Filenames

  split generates output filenames by appending a suffix to a prefix:

  | Prefix | Suffix Type  | Length | Files Generated            |
  |--------|-------------|--------|----------------------------|
  | x      | alphabetic  | 2      | xaa, xab, ..., xzz        |
  | x      | numeric (-d)| 2      | x00, x01, ..., x99        |
  | out    | alphabetic  | 3      | outaaa, outaab, ..., outzzz|

  The default prefix is "x" and the default suffix length is 2.

  ## Alphabetic Suffix Generation

  Alphabetic suffixes work like a base-26 counter using a-z:

      0  => "aa"    (suffix length 2)
      1  => "ab"
      25 => "az"
      26 => "ba"
      ...
      675 => "zz"   (26^2 - 1 = 675)

  This is essentially counting in base 26 where a=0, b=1, ..., z=25.

  ## Numeric Suffix Generation

  Numeric suffixes are simpler — just zero-padded decimal numbers:

      0  => "00"
      1  => "01"
      99 => "99"

  ## Hex Suffix Generation

  Hex suffixes use hexadecimal digits:

      0  => "00"
      15 => "0f"
      255 => "ff"

  ## Split by Bytes

  The `-b` flag accepts a size with optional suffix:

  | Suffix | Multiplier  | Example  |
  |--------|-------------|----------|
  | (none) | 1           | 1000     |
  | K / k  | 1024        | 1K       |
  | M / m  | 1048576     | 1M       |
  | G / g  | 1073741824  | 1G       |

  ## Implementation Approach

  Pure functions implement each piece:

  1. `generate_suffix/3` creates the suffix for a given file index.
  2. `split_by_lines/3` splits content into chunks of N lines.
  3. `split_by_bytes/3` splits content into chunks of N bytes.
  4. `parse_size/1` parses a size string like "1M" into bytes.
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

    case Parser.parse(spec_path, ["split" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        suffix_len = flags["suffix_length"] || 2
        use_numeric = !!flags["numeric_suffixes"]
        use_hex = !!flags["hex_suffixes"]
        verbose = !!flags["verbose"]
        additional_suffix = flags["additional_suffix"] || ""
        lines_per_file = flags["lines"] || 1000
        bytes_spec = flags["bytes"]

        suffix_type =
          cond do
            use_numeric -> :numeric
            use_hex -> :hex
            true -> :alpha
          end

        file_path = arguments["file"] || "-"
        prefix = arguments["prefix"] || "x"

        content =
          if file_path == "-" do
            IO.read(:stdio, :eof)
          else
            File.read!(file_path)
          end

        chunks =
          if bytes_spec do
            byte_count = parse_size(bytes_spec)
            split_by_bytes(content, byte_count)
          else
            split_by_lines(content, lines_per_file)
          end

        write_chunks(chunks, prefix, suffix_type, suffix_len, additional_suffix, verbose)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "split: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Suffix Generation
  # ---------------------------------------------------------------------------

  @doc """
  Generate a suffix string for a given file index.

  ## Alphabetic Suffixes

  Alphabetic suffixes count in base 26 using a-z. The suffix length
  determines how many characters are used:

      generate_suffix(0, :alpha, 2)   => "aa"
      generate_suffix(1, :alpha, 2)   => "ab"
      generate_suffix(25, :alpha, 2)  => "az"
      generate_suffix(26, :alpha, 2)  => "ba"
      generate_suffix(27, :alpha, 2)  => "bb"

  ## Numeric Suffixes

  Numeric suffixes are zero-padded decimal:

      generate_suffix(0, :numeric, 2)  => "00"
      generate_suffix(5, :numeric, 2)  => "05"
      generate_suffix(42, :numeric, 3) => "042"

  ## Hex Suffixes

  Hex suffixes are zero-padded hexadecimal:

      generate_suffix(0, :hex, 2)   => "00"
      generate_suffix(15, :hex, 2)  => "0f"
      generate_suffix(255, :hex, 2) => "ff"

  ## Examples

      iex> UnixTools.Split.generate_suffix(0, :alpha, 2)
      "aa"

      iex> UnixTools.Split.generate_suffix(27, :alpha, 2)
      "bb"

      iex> UnixTools.Split.generate_suffix(5, :numeric, 3)
      "005"

      iex> UnixTools.Split.generate_suffix(255, :hex, 2)
      "ff"
  """
  def generate_suffix(index, :alpha, suffix_len) do
    # -------------------------------------------------------------------------
    # Convert index to base-26 digits, then map each digit to a-z.
    #
    # This is like converting a number to base 26:
    #   index=0   => [0, 0]     => "aa"
    #   index=1   => [0, 1]     => "ab"
    #   index=26  => [1, 0]     => "ba"
    #   index=27  => [1, 1]     => "bb"
    # -------------------------------------------------------------------------

    digits = base_convert(index, 26, suffix_len)
    Enum.map(digits, fn d -> <<(?a + d)>> end) |> Enum.join()
  end

  def generate_suffix(index, :numeric, suffix_len) do
    Integer.to_string(index) |> String.pad_leading(suffix_len, "0")
  end

  def generate_suffix(index, :hex, suffix_len) do
    Integer.to_string(index, 16)
    |> String.downcase()
    |> String.pad_leading(suffix_len, "0")
  end

  @doc """
  Convert an integer to a list of digits in the given base, with a fixed
  number of positions.

  ## How It Works

  We repeatedly divide by the base and collect remainders, then pad
  to the desired length. This is standard base conversion.

  ## Examples

      iex> UnixTools.Split.base_convert(0, 26, 2)
      [0, 0]

      iex> UnixTools.Split.base_convert(27, 26, 2)
      [1, 1]

      iex> UnixTools.Split.base_convert(26, 26, 2)
      [1, 0]
  """
  def base_convert(number, base, num_positions) do
    digits =
      if number == 0 do
        [0]
      else
        do_base_convert(number, base, [])
      end

    # Pad with leading zeros to reach the desired number of positions.
    pad_count = max(0, num_positions - length(digits))
    List.duplicate(0, pad_count) ++ digits
  end

  defp do_base_convert(0, _base, acc), do: acc

  defp do_base_convert(number, base, acc) do
    do_base_convert(div(number, base), base, [rem(number, base) | acc])
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Splitting Content
  # ---------------------------------------------------------------------------

  @doc """
  Split content into chunks of N lines each.

  The last chunk may have fewer than N lines.

  ## Examples

      iex> UnixTools.Split.split_by_lines("a\\nb\\nc\\nd\\ne\\n", 2)
      ["a\\nb\\n", "c\\nd\\n", "e\\n"]

      iex> UnixTools.Split.split_by_lines("one\\ntwo\\nthree\\n", 5)
      ["one\\ntwo\\nthree\\n"]
  """
  def split_by_lines(content, lines_per_chunk) do
    lines = String.split(content, "\n")

    # If the content ends with a newline, String.split produces a trailing
    # empty string. We keep it to preserve the newline at the end.

    lines
    |> Enum.chunk_every(lines_per_chunk)
    |> Enum.map(fn chunk -> Enum.join(chunk, "\n") end)
    |> Enum.reject(fn chunk -> chunk == "" end)
  end

  @doc """
  Split content into chunks of N bytes each.

  Binary content is sliced into fixed-size pieces. The last chunk may
  be smaller than N bytes.

  ## Examples

      iex> UnixTools.Split.split_by_bytes("abcdefghij", 3)
      ["abc", "def", "ghi", "j"]

      iex> UnixTools.Split.split_by_bytes("hello", 10)
      ["hello"]
  """
  def split_by_bytes(content, bytes_per_chunk) do
    do_split_bytes(content, bytes_per_chunk, [])
    |> Enum.reverse()
  end

  defp do_split_bytes(<<>>, _chunk_size, acc), do: acc

  defp do_split_bytes(data, chunk_size, acc) do
    size = min(byte_size(data), chunk_size)
    <<chunk::binary-size(size), remaining::binary>> = data
    do_split_bytes(remaining, chunk_size, [chunk | acc])
  end

  @doc """
  Parse a size string like "1K", "2M", "100" into a number of bytes.

  ## Supported Suffixes

  | Suffix   | Multiplier | Example | Bytes     |
  |----------|------------|---------|-----------|
  | (none)   | 1          | "100"   | 100       |
  | K or k   | 1024       | "1K"    | 1024      |
  | M or m   | 1048576    | "2M"    | 2097152   |
  | G or g   | 2^30       | "1G"    | 1073741824|

  ## Examples

      iex> UnixTools.Split.parse_size("100")
      100

      iex> UnixTools.Split.parse_size("1K")
      1024

      iex> UnixTools.Split.parse_size("2M")
      2097152

      iex> UnixTools.Split.parse_size("1G")
      1073741824
  """
  def parse_size(size_str) when is_binary(size_str) do
    size_str = String.trim(size_str)

    cond do
      String.ends_with?(size_str, ["G", "g"]) ->
        {num, _} = Integer.parse(String.slice(size_str, 0..-2//1))
        num * 1024 * 1024 * 1024

      String.ends_with?(size_str, ["M", "m"]) ->
        {num, _} = Integer.parse(String.slice(size_str, 0..-2//1))
        num * 1024 * 1024

      String.ends_with?(size_str, ["K", "k"]) ->
        {num, _} = Integer.parse(String.slice(size_str, 0..-2//1))
        num * 1024

      true ->
        {num, _} = Integer.parse(size_str)
        num
    end
  end

  def parse_size(size_val) when is_integer(size_val), do: size_val

  # ---------------------------------------------------------------------------
  # File Output
  # ---------------------------------------------------------------------------

  defp write_chunks(chunks, prefix, suffix_type, suffix_len, additional_suffix, verbose) do
    chunks
    |> Enum.with_index()
    |> Enum.each(fn {chunk, idx} ->
      suffix = generate_suffix(idx, suffix_type, suffix_len)
      filename = "#{prefix}#{suffix}#{additional_suffix}"

      if verbose do
        IO.puts(:stderr, "creating file '#{filename}'")
      end

      File.write!(filename, chunk)
    end)
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "split.json"),
        else: nil
      ),
      "split.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "split.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find split.json spec file"
  end
end
