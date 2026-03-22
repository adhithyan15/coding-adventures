defmodule UnixTools.Cmp do
  @moduledoc """
  cmp -- compare two files byte by byte.

  ## What This Program Does

  This is a reimplementation of the GNU `cmp` utility in Elixir. It compares
  two files byte by byte and reports the first position where they differ.

  ## How cmp Works

  At its simplest:

      cmp file1.txt file2.txt   =>   reports first difference (or nothing if identical)

  ## Output Modes

  cmp has three output modes:

  | Mode        | Flag | Output                                              |
  |-------------|------|------------------------------------------------------|
  | Default     | -    | "file1 file2 differ: byte N, line L"                |
  | Verbose     | -l   | Lists every differing byte: "offset byte1 byte2"    |
  | Silent      | -s   | No output at all, just exit code                    |

  ## Exit Codes

  | Code | Meaning                  |
  |------|--------------------------|
  | 0    | Files are identical       |
  | 1    | Files differ              |
  | 2    | Error (file not found)    |

  ## The -b Flag

  When -b (print-bytes) is combined with default output, the differing bytes
  are shown as octal values alongside their character representations.

  ## The -i Flag (ignore-initial)

  Skip the first N bytes of both files before comparing. The format is a
  single number (skip same amount in both) or SKIP1:SKIP2 (different amounts).

  ## The -n Flag (max-bytes)

  Compare at most N bytes, then stop. Useful for comparing only the header
  of large files.

  ## Implementation Approach

  The core comparison is implemented as a pure function `compare_bytes/2`
  that takes two binaries and options, returning a list of differences.
  This keeps business logic separate from I/O.
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

    case Parser.parse(spec_path, ["cmp" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        opts = %{
          verbose: !!flags["list"],
          silent: !!flags["silent"],
          print_bytes: !!flags["print_bytes"],
          skip: parse_skip(flags["ignore_initial"]),
          max_bytes: flags["max_bytes"]
        }

        file1 = arguments["file1"]
        file2 = arguments["file2"] || "-"

        run(file1, file2, opts)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn err ->
          IO.puts(:stderr, "cmp: #{err.message}")
        end)

        System.halt(2)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic
  # ---------------------------------------------------------------------------

  @doc """
  Compare two binaries byte by byte, returning a comparison result.

  ## Return Values

  - `:equal` — the binaries are identical (after skip/limit)
  - `{:differ, differences}` — list of `{offset, byte1, byte2}` tuples
  - `{:eof, which_file}` — one file ended before the other

  ## How It Works

  1. Apply skip offsets to both binaries.
  2. Limit both to max_bytes if specified.
  3. Walk through byte by byte, collecting differences.

  ## Examples

      iex> UnixTools.Cmp.compare_bytes("hello", "hello", %{})
      :equal

      iex> UnixTools.Cmp.compare_bytes("hello", "hxllo", %{})
      {:differ, [{2, ?e, ?x}]}
  """
  def compare_bytes(bin1, bin2, opts \\ %{}) do
    {skip1, skip2} = Map.get(opts, :skip, {0, 0})
    max_bytes = Map.get(opts, :max_bytes, nil)

    # -------------------------------------------------------------------------
    # Step 1: Apply skip offsets.
    #
    # binary_part/3 extracts a substring starting at the skip offset.
    # If the skip exceeds the binary length, we get an empty binary.
    # -------------------------------------------------------------------------

    bytes1 = safe_binary_slice(bin1, skip1)
    bytes2 = safe_binary_slice(bin2, skip2)

    # -------------------------------------------------------------------------
    # Step 2: Apply max_bytes limit.
    # -------------------------------------------------------------------------

    bytes1 = if max_bytes, do: binary_part(bytes1, 0, min(max_bytes, byte_size(bytes1))), else: bytes1
    bytes2 = if max_bytes, do: binary_part(bytes2, 0, min(max_bytes, byte_size(bytes2))), else: bytes2

    # -------------------------------------------------------------------------
    # Step 3: Compare byte by byte.
    #
    # We convert to charlists (lists of integers) for easy zip-based comparison.
    # -------------------------------------------------------------------------

    list1 = :binary.bin_to_list(bytes1)
    list2 = :binary.bin_to_list(bytes2)

    min_len = min(length(list1), length(list2))

    differences =
      Enum.zip(list1, list2)
      |> Enum.with_index(1)
      |> Enum.filter(fn {{b1, b2}, _offset} -> b1 != b2 end)
      |> Enum.map(fn {{b1, b2}, offset} -> {offset, b1, b2} end)

    cond do
      # If one file is shorter, report EOF on the shorter file.
      length(list1) != length(list2) and differences == [] ->
        which = if length(list1) < length(list2), do: :file1, else: :file2
        {:eof, which, min_len}

      length(list1) != length(list2) and differences != [] ->
        {:differ, differences}

      differences == [] ->
        :equal

      true ->
        {:differ, differences}
    end
  end

  @doc """
  Format the comparison result for output.

  In default mode, only the first difference is reported:

      file1 file2 differ: byte 5, line 2

  In verbose mode (-l), every difference is listed:

      5  145  170

  In silent mode (-s), nothing is printed.
  """
  def format_result(result, file1, file2, bin1, opts) do
    cond do
      opts[:silent] ->
        case result do
          :equal -> {:exit, 0, ""}
          {:eof, _, _} -> {:exit, 1, ""}
          {:differ, _} -> {:exit, 1, ""}
        end

      true ->
        case result do
          :equal ->
            {:exit, 0, ""}

          {:eof, which, byte_count} ->
            eof_file = if which == :file1, do: file1, else: file2
            msg = "cmp: EOF on #{eof_file} after byte #{byte_count}"
            {:exit, 1, msg}

          {:differ, differences} when opts.verbose ->
            lines =
              Enum.map(differences, fn {offset, b1, b2} ->
                if opts[:print_bytes] do
                  "#{offset} #{format_octal(b1)} #{format_char(b1)} #{format_octal(b2)} #{format_char(b2)}"
                else
                  "#{offset} #{format_octal(b1)} #{format_octal(b2)}"
                end
              end)
            {:exit, 1, Enum.join(lines, "\n")}

          {:differ, [{offset, b1, b2} | _rest]} ->
            {skip1, _skip2} = Map.get(opts, :skip, {0, 0})
            line_num = count_lines(bin1, skip1, offset)

            msg =
              if opts[:print_bytes] do
                "#{file1} #{file2} differ: byte #{offset}, line #{line_num} is #{format_octal(b1)} #{format_char(b1)} #{format_octal(b2)} #{format_char(b2)}"
              else
                "#{file1} #{file2} differ: byte #{offset}, line #{line_num}"
              end
            {:exit, 1, msg}
        end
    end
  end

  @doc """
  Count the number of newlines in the first `count` bytes of a binary,
  starting after `skip` bytes. Returns 1-based line number.

  ## Example

      iex> UnixTools.Cmp.count_lines("ab\\ncd\\nef", 0, 5)
      2
  """
  def count_lines(bin, skip, count) do
    slice = safe_binary_slice(bin, skip)
    actual_count = min(count, byte_size(slice))
    region = binary_part(slice, 0, actual_count)

    # Count newlines in the region, add 1 for 1-based line numbering
    newlines = region |> :binary.bin_to_list() |> Enum.count(fn b -> b == ?\n end)
    newlines + 1
  end

  @doc """
  Parse the -i (ignore-initial) flag value.

  Formats:
  - `nil` — no skip: `{0, 0}`
  - `"N"` — skip N bytes in both files: `{N, N}`
  - `"N1:N2"` — skip N1 in file1, N2 in file2: `{N1, N2}`
  """
  def parse_skip(nil), do: {0, 0}

  def parse_skip(value) when is_binary(value) do
    case String.split(value, ":") do
      [n] ->
        {String.to_integer(n), String.to_integer(n)}

      [n1, n2] ->
        {String.to_integer(n1), String.to_integer(n2)}
    end
  end

  # ---------------------------------------------------------------------------
  # Run
  # ---------------------------------------------------------------------------

  defp run(file1, file2, opts) do
    with {:ok, bin1} <- read_file(file1),
         {:ok, bin2} <- read_file(file2) do
      result = compare_bytes(bin1, bin2, opts)
      {exit_code, output} = format_exit(result, file1, file2, bin1, opts)

      if output != "", do: IO.puts(output)
      if exit_code != 0, do: System.halt(exit_code)
    else
      {:error, reason} ->
        IO.puts(:stderr, "cmp: #{reason}")
        System.halt(2)
    end
  end

  defp format_exit(result, file1, file2, bin1, opts) do
    {:exit, code, msg} = format_result(result, file1, file2, bin1, opts)
    {code, msg}
  end

  defp read_file("-") do
    case IO.read(:stdio, :eof) do
      {:error, reason} -> {:error, "stdin: #{inspect(reason)}"}
      :eof -> {:ok, ""}
      data -> {:ok, data}
    end
  end

  defp read_file(path) do
    case File.read(path) do
      {:ok, data} -> {:ok, data}
      {:error, reason} -> {:error, "#{path}: #{:file.format_error(reason)}"}
    end
  end

  # ---------------------------------------------------------------------------
  # Formatting helpers
  # ---------------------------------------------------------------------------

  @doc """
  Format a byte as an octal string (3 digits, zero-padded).
  """
  def format_octal(byte) do
    Integer.to_string(byte, 8) |> String.pad_leading(3, "0")
  end

  @doc """
  Format a byte as a printable character representation.

  Printable ASCII (32-126) is shown as the character itself.
  Non-printable bytes are shown as their octal escape.
  """
  def format_char(byte) when byte >= 32 and byte <= 126 do
    <<byte>>
  end

  def format_char(byte) do
    "\\#{Integer.to_string(byte, 8)}"
  end

  # ---------------------------------------------------------------------------
  # Utility
  # ---------------------------------------------------------------------------

  defp safe_binary_slice(bin, skip) when skip >= byte_size(bin), do: <<>>
  defp safe_binary_slice(bin, skip), do: binary_part(bin, skip, byte_size(bin) - skip)

  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "cmp.json"),
        else: nil
      ),
      "cmp.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "cmp.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      path -> File.exists?(path)
    end) ||
      raise "Could not find cmp.json spec file"
  end
end
