defmodule UnixTools.Cat do
  @moduledoc """
  cat -- concatenate files and print on standard output.

  ## What This Program Does

  This is a reimplementation of the GNU `cat` utility in Elixir. It reads
  files sequentially and writes their contents to standard output. When no
  files are given, or when a file is `-`, it reads from stdin.

  ## How cat Works

  At its simplest, cat just copies bytes from input to output:

      cat file1.txt file2.txt    =>   contents of file1 followed by file2
      cat                        =>   copies stdin to stdout
      cat -                      =>   same as above

  The name "cat" comes from "concatenate" -- it joins files together
  end-to-end.

  ## Display Flags

  - `-n` (--number):         Number ALL output lines starting from 1.
  - `-b` (--number-nonblank): Number only non-blank lines (overrides -n).
  - `-s` (--squeeze-blank):  Collapse consecutive blank lines into one.
  - `-T` (--show-tabs):      Display TAB characters as `^I`.
  - `-E` (--show-ends):      Display `$` at the end of each line.
  - `-v` (--show-nonprinting): Use `^` and `M-` notation for control chars.
  - `-A` (--show-all):       Equivalent to `-vET` (show everything).

  ## Line Numbering Semantics

  When numbering lines (`-n` or `-b`), the line counter is global across
  all files. The number is right-justified in a field of width 6, followed
  by a tab.
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
  3. Extract flags into an options map.
  4. Process each file in order, maintaining a global line counter.
  """
  def main(argv) do
    spec_path = resolve_spec_path()

    case Parser.parse(spec_path, ["cat" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        # -----------------------------------------------------------------------
        # Extract flags into an options map.
        # Expand -A (show-all) into its component flags: -v, -E, -T.
        # -----------------------------------------------------------------------

        show_all = !!flags["show_all"]

        opts = %{
          number: !!flags["number"],
          number_nonblank: !!flags["number_nonblank"],
          squeeze_blank: !!flags["squeeze_blank"],
          show_tabs: show_all or !!flags["show_tabs"],
          show_ends: show_all or !!flags["show_ends"],
          show_nonprinting: show_all or !!flags["show_nonprinting"]
        }

        # Get the list of files. Default to stdin ("-").
        file_list = normalize_files(arguments["files"])

        # Process each file, threading the line number through.
        Enum.reduce(file_list, 1, fn file, line_num ->
          process_file(file, opts, line_num)
        end)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "cat: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # File reading
  # ---------------------------------------------------------------------------

  @doc false
  defp process_file("-", opts, line_num) do
    # Read from stdin.
    content = read_stdin()
    process_content(content, opts, line_num)
  end

  defp process_file(file, opts, line_num) do
    case File.read(file) do
      {:ok, content} ->
        process_content(content, opts, line_num)

      {:error, reason} ->
        IO.puts(:stderr, "cat: #{file}: #{:file.format_error(reason)}")
        line_num
    end
  end

  @doc false
  defp read_stdin do
    case IO.read(:stdio, :eof) do
      {:error, _} -> ""
      :eof -> ""
      data -> data
    end
  end

  # ---------------------------------------------------------------------------
  # Content processing
  # ---------------------------------------------------------------------------

  @doc """
  Process a string through cat's transformation pipeline.

  Splits into lines, applies squeeze/numbering/display transforms, and
  writes each line to stdout. Returns the updated line number counter.
  """
  def process_content(content, opts, line_num) do
    lines = String.split(content, "\n")

    # The split creates an empty string after the final \\n.
    # We drop that trailing empty element to avoid printing an extra line.
    lines =
      if List.last(lines) == "" do
        List.delete_at(lines, -1)
      else
        lines
      end

    {_blank_count, final_line_num} =
      Enum.reduce(lines, {0, line_num}, fn line, {consecutive_blanks, num} ->
        is_blank = String.trim(line) == ""

        # --- Squeeze blank lines (-s) ---
        new_blanks = if is_blank, do: consecutive_blanks + 1, else: 0

        if opts.squeeze_blank and is_blank and consecutive_blanks >= 1 do
          # Skip this blank line (already printed one).
          {new_blanks, num}
        else
          # Apply transformations and print.
          transformed = line

          # --- Show non-printing characters (-v) ---
          transformed =
            if opts.show_nonprinting do
              show_nonprinting(transformed)
            else
              transformed
            end

          # --- Show tabs (-T) ---
          transformed =
            if opts.show_tabs do
              String.replace(transformed, "\t", "^I")
            else
              transformed
            end

          # --- Show ends (-E) ---
          transformed =
            if opts.show_ends do
              transformed <> "$"
            else
              transformed
            end

          # --- Number lines (-n or -b) ---
          {transformed, new_num} =
            if opts.number_nonblank do
              if not is_blank do
                {String.pad_leading(Integer.to_string(num), 6) <> "\t" <> transformed, num + 1}
              else
                {transformed, num}
              end
            else
              if opts.number do
                {String.pad_leading(Integer.to_string(num), 6) <> "\t" <> transformed, num + 1}
              else
                {transformed, num}
              end
            end

          IO.puts(transformed)
          {new_blanks, new_num}
        end
      end)

    final_line_num
  end

  # ---------------------------------------------------------------------------
  # Non-printing character display
  # ---------------------------------------------------------------------------

  @doc """
  Convert non-printing characters to visible notation.

  Implements the same notation as GNU cat -v:
  - Control characters (0x00-0x1F) become ^@ through ^_ (except TAB and LF)
  - DEL (0x7F) becomes ^?
  - High-bit characters (0x80-0xFF) become M- prefixed
  """
  def show_nonprinting(line) do
    line
    |> :binary.bin_to_list()
    |> Enum.map(fn
      # Tab -- leave as-is (handled by -T flag separately).
      9 -> "\t"
      # Newline -- leave as-is.
      10 -> "\n"
      # Control characters: ^@ through ^_
      code when code < 32 -> "^" <> <<code + 64>>
      # DEL character
      127 -> "^?"
      # High-bit control characters: M-^@ through M-^_
      code when code >= 128 and code < 160 -> "M-^" <> <<code - 128 + 64>>
      # High-bit printable characters
      code when code >= 160 and code < 255 -> "M-" <> <<code - 128>>
      # M-^?
      255 -> "M-^?"
      # Regular printable character
      code -> <<code>>
    end)
    |> IO.iodata_to_binary()
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
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "cat.json"),
        else: nil
      ),
      "cat.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "cat.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find cat.json spec file"
  end
end
