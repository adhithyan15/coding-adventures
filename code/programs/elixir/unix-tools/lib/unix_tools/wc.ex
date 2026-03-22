defmodule UnixTools.Wc do
  @moduledoc """
  wc -- word, line, and byte count.

  ## What This Program Does

  This is a reimplementation of the GNU `wc` utility in Elixir. It counts
  newlines, words, and bytes in files, printing the results in right-aligned
  columns.

  ## How wc Works

  By default (no flags), wc prints three counts for each file:

      $ wc file.txt
         10   42  256 file.txt

  That's: 10 lines, 42 words, 256 bytes. When multiple files are given,
  wc also prints a "total" line summing all counts.

  ## Counting Rules

  - **Lines** (`-l`): Count of newline characters (`\\n`).
  - **Words** (`-w`): Count of whitespace-delimited sequences.
  - **Bytes** (`-c`): Total bytes in the file.
  - **Characters** (`-m`): Total Unicode code points.
  - **Max line length** (`-L`): Display width of the longest line.

  ## Flag Selection

  Individual flags select which counts to display. If no flags are given,
  lines + words + bytes are shown (the default). `-c` and `-m` are mutually
  exclusive.
  """

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Entry point
  # ---------------------------------------------------------------------------

  @doc """
  Entry point. Receives `argv` as a list of strings.

  1. Parse arguments with CLI Builder.
  2. Handle --help and --version.
  3. Determine which columns to display from flags.
  4. Read each file and compute counts.
  5. If multiple files, add a "total" line.
  6. Format and print all results.
  """
  def main(argv) do
    spec_path = resolve_spec_path()

    case Parser.parse(spec_path, ["wc" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        # -----------------------------------------------------------------------
        # Determine which columns to display.
        # If no specific flags are given, show lines + words + bytes.
        # -----------------------------------------------------------------------

        any_flag_set =
          !!flags["lines"] or !!flags["words"] or !!flags["bytes"] or
            !!flags["chars"] or !!flags["max_line_length"]

        display = %{
          lines: if(any_flag_set, do: !!flags["lines"], else: true),
          words: if(any_flag_set, do: !!flags["words"], else: true),
          bytes: if(any_flag_set, do: !!flags["bytes"], else: true),
          chars: !!flags["chars"],
          max_line_length: !!flags["max_line_length"]
        }

        # If no flags set, default shows lines+words+bytes but NOT chars.
        display =
          if not any_flag_set do
            Map.put(display, :chars, false)
          else
            display
          end

        # Get files list.
        file_list = normalize_files(arguments["files"])

        # Process each file and collect counts.
        all_counts =
          Enum.reduce(file_list, [], fn file, acc ->
            case read_file(file) do
              {:ok, content, filename} ->
                acc ++ [count_content(content, filename)]

              {:error, reason, filename} ->
                IO.puts(:stderr, "wc: #{filename}: #{reason}")
                acc
            end
          end)

        # Add totals if multiple files.
        all_counts =
          if length(all_counts) > 1 do
            total = compute_total(all_counts)
            all_counts ++ [total]
          else
            all_counts
          end

        # Determine column width and print.
        col_width = column_width(all_counts)

        Enum.each(all_counts, fn counts ->
          IO.puts(format_line(counts, display, col_width))
        end)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "wc: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # File reading
  # ---------------------------------------------------------------------------

  @doc false
  defp read_file("-") do
    case IO.read(:stdio, :eof) do
      {:error, reason} -> {:error, inspect(reason), ""}
      :eof -> {:ok, "", ""}
      data -> {:ok, data, ""}
    end
  end

  defp read_file(file) do
    case File.read(file) do
      {:ok, content} -> {:ok, content, file}
      {:error, reason} -> {:error, :file.format_error(reason) |> to_string(), file}
    end
  end

  # ---------------------------------------------------------------------------
  # Counting
  # ---------------------------------------------------------------------------

  @doc """
  Count lines, words, bytes, characters, and max line length in a string.

  ## Word Counting Algorithm

  We use a state machine with one boolean: `in_word`. We start outside a
  word. Each time we transition from whitespace to non-whitespace, we
  increment the word count.

      "  hello   world  "
       ^^     ^^^     ^^
       |       |       |
       outside inside  outside

      Two transitions into a word => 2 words.
  """
  def count_content(content, filename) do
    bytes = byte_size(content)
    chars = String.length(content)
    lines = count_newlines(content)
    word_count = count_words(content)
    max_len = max_line_length(content)

    %{
      lines: lines,
      words: word_count,
      bytes: bytes,
      chars: chars,
      max_line_length: max_len,
      filename: filename
    }
  end

  @doc false
  defp count_newlines(content) do
    content
    |> String.graphemes()
    |> Enum.count(&(&1 == "\n"))
  end

  @doc false
  defp count_words(content) do
    content
    |> String.split(~r/\s+/, trim: true)
    |> length()
  end

  @doc false
  defp max_line_length(content) do
    content
    |> String.split("\n")
    |> Enum.map(&String.length/1)
    |> Enum.max(fn -> 0 end)
  end

  # ---------------------------------------------------------------------------
  # Totals
  # ---------------------------------------------------------------------------

  @doc false
  defp compute_total(all_counts) do
    %{
      lines: Enum.sum(Enum.map(all_counts, & &1.lines)),
      words: Enum.sum(Enum.map(all_counts, & &1.words)),
      bytes: Enum.sum(Enum.map(all_counts, & &1.bytes)),
      chars: Enum.sum(Enum.map(all_counts, & &1.chars)),
      max_line_length: Enum.max(Enum.map(all_counts, & &1.max_line_length)),
      filename: "total"
    }
  end

  # ---------------------------------------------------------------------------
  # Formatting
  # ---------------------------------------------------------------------------

  @doc """
  Format a single line of output with right-aligned columns.
  """
  def format_line(counts, display, col_width) do
    parts = []

    parts = if display.lines, do: parts ++ [pad(counts.lines, col_width)], else: parts
    parts = if display.words, do: parts ++ [pad(counts.words, col_width)], else: parts
    parts = if display.bytes, do: parts ++ [pad(counts.bytes, col_width)], else: parts
    parts = if display.chars, do: parts ++ [pad(counts.chars, col_width)], else: parts

    parts =
      if display.max_line_length,
        do: parts ++ [pad(counts.max_line_length, col_width)],
        else: parts

    # Add filename if present.
    parts =
      if counts.filename != "" do
        parts ++ [counts.filename]
      else
        parts
      end

    Enum.join(parts, " ")
  end

  @doc false
  defp pad(number, width) do
    number
    |> Integer.to_string()
    |> String.pad_leading(width)
  end

  @doc false
  defp column_width(all_counts) do
    max_val =
      all_counts
      |> Enum.flat_map(fn c ->
        [c.lines, c.words, c.bytes, c.chars, c.max_line_length]
      end)
      |> Enum.max(fn -> 0 end)

    max(1, String.length(Integer.to_string(max_val)))
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  @doc false
  defp normalize_files(nil), do: ["-"]
  defp normalize_files(files) when is_list(files), do: files
  defp normalize_files(file) when is_binary(file), do: [file]

  @doc false
  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "wc.json"),
        else: nil
      ),
      "wc.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "wc.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find wc.json spec file"
  end
end
