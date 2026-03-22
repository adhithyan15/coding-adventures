defmodule UnixTools.Nl do
  @moduledoc """
  nl -- number lines of files.

  ## What This Program Does

  This is a reimplementation of the GNU `nl` utility in Elixir. It reads
  files and writes them to standard output with line numbers added.

  ## How nl Works

  nl adds line numbers to the left of each line:

      nl file.txt                =>   number non-empty lines
      nl -ba file.txt            =>   number ALL lines (including blank)
      nl -w 3 file.txt           =>   use 3-digit line numbers
      nl -s '. ' file.txt        =>   use ". " as the separator

  ## Numbering Styles

  - `a` (all): Number every line, including blank lines.
  - `t` (text): Number only non-empty lines (default for body).
  - `n` (none): Don't number any lines (default for header/footer).
  - `pBRE`: Number only lines matching a basic regular expression.

  ## Number Formats

  - `ln`: Left-justified, no leading zeros.
  - `rn`: Right-justified, no leading zeros (default).
  - `rz`: Right-justified, leading zeros.

  ## Logical Pages

  nl supports header/body/footer sections delimited by special lines using
  the section delimiter (default `\\:`):
  - `\\:\\:\\:` starts a header section.
  - `\\:\\:` starts a body section.
  - `\\:` starts a footer section.
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

    case Parser.parse(spec_path, ["nl" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        body_numbering = flags["body_numbering"] || "t"
        header_numbering = flags["header_numbering"] || "n"
        footer_numbering = flags["footer_numbering"] || "n"
        line_increment = flags["line_increment"] || 1
        number_format = flags["number_format"] || "rn"
        number_width = flags["number_width"] || 6
        number_separator = flags["number_separator"] || "\t"
        starting_line_number = flags["starting_line_number"] || 1
        section_delimiter = flags["section_delimiter"] || "\\:"
        no_renumber = !!flags["no_renumber"]

        file_list = normalize_files(arguments["files"])

        # Build section delimiters.
        header_delim = String.duplicate(section_delimiter, 3)
        body_delim = String.duplicate(section_delimiter, 2)
        footer_delim = section_delimiter

        # Process files with state tracking.
        _final_state =
          Enum.reduce(file_list, {starting_line_number, body_numbering}, fn file, {line_num, current_style} ->
            case read_file(file) do
              {:ok, content} ->
                process_content(
                  content,
                  line_num,
                  current_style,
                  header_delim,
                  body_delim,
                  footer_delim,
                  header_numbering,
                  body_numbering,
                  footer_numbering,
                  line_increment,
                  number_format,
                  number_width,
                  number_separator,
                  starting_line_number,
                  no_renumber
                )

              {:error, reason} ->
                IO.puts(:stderr, "nl: #{file}: #{:file.format_error(reason)}")
                {line_num, current_style}
            end
          end)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "nl: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Line numbering.
  # ---------------------------------------------------------------------------

  @doc """
  Determine if a line should be numbered based on the numbering style.
  """
  def should_number(line, style) do
    case style do
      "a" -> true
      "t" -> String.length(line) > 0
      "n" -> false
      "p" <> pattern ->
        case Regex.compile(pattern) do
          {:ok, regex} -> Regex.match?(regex, line)
          {:error, _} -> false
        end
      _ -> false
    end
  end

  @doc """
  Format a line number according to the specified format and width.

  - `ln`: Left-justified, space-padded.
  - `rn`: Right-justified, space-padded.
  - `rz`: Right-justified, zero-padded.
  """
  def format_number(num, number_format, width) do
    num_str = Integer.to_string(num)

    case number_format do
      "ln" -> String.pad_trailing(num_str, width)
      "rz" -> String.pad_leading(num_str, width, "0")
      _ -> String.pad_leading(num_str, width)
    end
  end

  defp process_content(
         content,
         line_num,
         current_style,
         header_delim,
         body_delim,
         footer_delim,
         header_numbering,
         body_numbering,
         footer_numbering,
         line_increment,
         number_format,
         number_width,
         number_separator,
         starting_line_number,
         no_renumber
       ) do
    lines = String.split(content, "\n")
    has_trailing = String.ends_with?(content, "\n")
    limit = if has_trailing, do: length(lines) - 1, else: length(lines)

    effective_lines = Enum.take(lines, limit)

    Enum.reduce(effective_lines, {line_num, current_style}, fn line, {num, style} ->
      cond do
        line == header_delim ->
          new_num = if no_renumber, do: num, else: starting_line_number
          IO.write("\n")
          {new_num, header_numbering}

        line == body_delim ->
          new_num = if no_renumber, do: num, else: starting_line_number
          IO.write("\n")
          {new_num, body_numbering}

        line == footer_delim ->
          IO.write("\n")
          {num, footer_numbering}

        true ->
          if should_number(line, style) do
            formatted = format_number(num, number_format, number_width)
            IO.write(formatted <> number_separator <> line <> "\n")
            {num + line_increment, style}
          else
            # Non-numbered line: pad with spaces.
            IO.write(String.duplicate(" ", number_width) <> "  " <> line <> "\n")
            {num, style}
          end
      end
    end)
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp read_file("-") do
    case IO.read(:stdio, :eof) do
      {:error, reason} -> {:error, reason}
      :eof -> {:ok, ""}
      data -> {:ok, data}
    end
  end

  defp read_file(file), do: File.read(file)

  defp normalize_files(nil), do: ["-"]
  defp normalize_files(files) when is_list(files), do: files
  defp normalize_files(file) when is_binary(file), do: [file]

  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "nl.json"),
        else: nil
      ),
      "nl.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "nl.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find nl.json spec file"
  end
end
