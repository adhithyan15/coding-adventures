defmodule UnixTools.Tr do
  @moduledoc """
  tr -- translate or delete characters.

  ## What This Program Does

  This is a reimplementation of the GNU `tr` utility in Elixir. It reads
  from standard input, translates, squeezes, or deletes characters, and
  writes the result to standard output.

  ## How tr Works

  tr operates on individual characters:

      echo "hello" | tr 'l' 'r'         =>   "herro"
      echo "hello" | tr 'a-z' 'A-Z'     =>   "HELLO"
      echo "hello" | tr -d 'l'           =>   "heo"
      echo "aabbcc" | tr -s 'a-c'        =>   "abc"

  ## Character Sets

  SET1 and SET2 are strings of characters. Special notations:
  - `a-z`: Character range from 'a' to 'z' (inclusive).
  - `\\n`, `\\t`, `\\\\`: Escape sequences for newline, tab, backslash.

  ## Operation Modes

  1. **Translate** (default): Replace SET1 chars with corresponding SET2 chars.
  2. **Delete** (`-d`): Remove SET1 characters from input.
  3. **Squeeze** (`-s`): Compress consecutive duplicates from the last SET.
  4. **Complement** (`-c`): Use the complement of SET1.
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

    case Parser.parse(spec_path, ["tr" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        use_complement = !!flags["complement"]
        delete_mode = !!flags["delete"]
        squeeze = !!flags["squeeze_repeats"]
        truncate_set1 = !!flags["truncate_set1"]

        set1_spec = arguments["set1"]
        set2_spec = arguments["set2"]

        # Expand character sets.
        set1_raw = expand_set(set1_spec)
        set2 = if set2_spec, do: expand_set(set2_spec), else: []

        # Apply complement if requested.
        set1 = if use_complement, do: complement_set(set1_raw), else: set1_raw

        # Truncate set1 to length of set2 if requested.
        set1 =
          if truncate_set1 and length(set2) > 0 do
            Enum.take(set1, length(set2))
          else
            set1
          end

        # Read stdin.
        input = read_stdin()

        # Transform and output.
        output =
          cond do
            delete_mode ->
              delete_chars(input, set1, squeeze, set2)

            squeeze and set2 == [] ->
              squeeze_chars(input, set1)

            set2 != [] ->
              translate_chars(input, set1, set2, squeeze)

            true ->
              input
          end

        IO.write(output)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "tr: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Character set expansion.
  # ---------------------------------------------------------------------------

  @doc """
  Expand a character set specification into a list of characters.

  Supports literal characters, ranges (a-z), and escape sequences.
  """
  def expand_set(set_spec) do
    set_spec
    |> String.graphemes()
    |> do_expand([])
    |> Enum.reverse()
  end

  defp do_expand([], acc), do: acc

  defp do_expand(["\\" | [next | rest]], acc) do
    char =
      case next do
        "n" -> "\n"
        "t" -> "\t"
        "r" -> "\r"
        "\\" -> "\\"
        "a" -> <<7>>
        "b" -> "\b"
        "f" -> "\f"
        "v" -> <<11>>
        other -> other
      end

    do_expand(rest, [char | acc])
  end

  defp do_expand([ch, "-", end_ch | rest], acc) do
    start_code = :binary.first(ch)
    end_code = :binary.first(end_ch)

    if start_code <= end_code do
      # Expand the range from start_code to end_code inclusive.
      # The range characters replace the "ch-end_ch" pattern entirely.
      range_chars =
        start_code..end_code
        |> Enum.map(fn code -> <<code>> end)

      do_expand(rest, Enum.reverse(range_chars) ++ acc)
    else
      # Not a valid range -- treat "-" as a literal character.
      do_expand(["-", end_ch | rest], [ch | acc])
    end
  end

  defp do_expand([ch | rest], acc) do
    do_expand(rest, [ch | acc])
  end

  @doc """
  Compute the complement of a character set (all ASCII chars NOT in the set).
  """
  def complement_set(set_chars) do
    set = MapSet.new(set_chars)

    0..127
    |> Enum.map(fn code -> <<code>> end)
    |> Enum.reject(fn ch -> MapSet.member?(set, ch) end)
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Translation, deletion, squeezing.
  # ---------------------------------------------------------------------------

  @doc """
  Translate characters: replace each character in set1 with the corresponding
  character in set2.
  """
  def translate_chars(input, set1, set2, squeeze) do
    # Build a translation map.
    last_set2 = List.last(set2)

    translation_map =
      set1
      |> Enum.with_index()
      |> Enum.into(%{}, fn {ch, i} ->
        replacement = Enum.at(set2, i, last_set2)
        {ch, replacement}
      end)

    squeeze_set = MapSet.new(set2)

    input
    |> String.graphemes()
    |> Enum.reduce({"", nil}, fn ch, {output, last_char} ->
      translated = Map.get(translation_map, ch, ch)

      if squeeze and translated == last_char and MapSet.member?(squeeze_set, translated) do
        {output, last_char}
      else
        {output <> translated, translated}
      end
    end)
    |> elem(0)
  end

  @doc """
  Delete characters in set1 from the input.
  """
  def delete_chars(input, set1, squeeze, squeeze_set_chars) do
    delete_set = MapSet.new(set1)
    squeeze_lookup = MapSet.new(squeeze_set_chars)

    input
    |> String.graphemes()
    |> Enum.reduce({"", nil}, fn ch, {output, last_char} ->
      cond do
        MapSet.member?(delete_set, ch) ->
          {output, last_char}

        squeeze and ch == last_char and MapSet.member?(squeeze_lookup, ch) ->
          {output, last_char}

        true ->
          {output <> ch, ch}
      end
    end)
    |> elem(0)
  end

  @doc """
  Squeeze characters: replace consecutive duplicates in the set with a
  single occurrence.
  """
  def squeeze_chars(input, set_chars) do
    squeeze_set = MapSet.new(set_chars)

    input
    |> String.graphemes()
    |> Enum.reduce({"", nil}, fn ch, {output, last_char} ->
      if ch == last_char and MapSet.member?(squeeze_set, ch) do
        {output, last_char}
      else
        {output <> ch, ch}
      end
    end)
    |> elem(0)
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp read_stdin do
    case IO.read(:stdio, :eof) do
      {:error, _} -> ""
      :eof -> ""
      data -> data
    end
  end

  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "tr.json"),
        else: nil
      ),
      "tr.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "tr.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find tr.json spec file"
  end
end
