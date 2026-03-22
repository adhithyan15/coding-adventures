defmodule UnixTools.Seq do
  @moduledoc """
  seq -- print a sequence of numbers.

  ## What This Program Does

  This is a reimplementation of the GNU `seq` utility in Elixir. It prints
  a sequence of numbers from FIRST to LAST, in steps of INCREMENT.

  ## How seq Works

  seq accepts one, two, or three positional arguments:

      seq LAST              =>   1, 2, 3, ..., LAST
      seq FIRST LAST        =>   FIRST, FIRST+1, ..., LAST
      seq FIRST INCR LAST   =>   FIRST, FIRST+INCR, FIRST+2*INCR, ..., LAST

  ## Floating Point Support

  seq handles both integers and floating-point numbers:

      seq 0.5 0.5 2.5   =>   0.5, 1.0, 1.5, 2.0, 2.5

  ## Equal Width Mode (-w)

  With `-w`, all numbers are padded with leading zeroes to the same width:

      seq -w 8 12   =>   08, 09, 10, 11, 12

  ## Custom Separator (-s)

  By default, numbers are separated by newlines. With `-s STRING`:

      seq -s ', ' 5   =>   1, 2, 3, 4, 5
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

    case Parser.parse(spec_path, ["seq" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        separator = flags["separator"] || "\n"
        equal_width = !!flags["equal_width"]

        # Parse positional arguments.
        numbers = normalize_numbers(arguments["numbers"])

        {first_str, incr_str, last_str} =
          case length(numbers) do
            1 -> {"1", "1", Enum.at(numbers, 0)}
            2 -> {Enum.at(numbers, 0), "1", Enum.at(numbers, 1)}
            _ -> {Enum.at(numbers, 0), Enum.at(numbers, 1), Enum.at(numbers, 2)}
          end

        first = parse_number(first_str)
        increment = parse_number(incr_str)
        last = parse_number(last_str)

        # Determine output precision.
        precision =
          Enum.max([
            decimal_places(first_str),
            decimal_places(incr_str),
            decimal_places(last_str)
          ])

        # Generate the sequence.
        sequence = generate_sequence(first, increment, last, precision)

        # Apply equal-width padding.
        sequence =
          if equal_width and length(sequence) > 0 do
            max_width = sequence |> Enum.map(&String.length/1) |> Enum.max()
            Enum.map(sequence, &String.pad_leading(&1, max_width, "0"))
          else
            sequence
          end

        # Output.
        if length(sequence) > 0 do
          IO.write(Enum.join(sequence, separator) <> "\n")
        end

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "seq: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Generate the number sequence.
  # ---------------------------------------------------------------------------

  @doc """
  Determine how many decimal places a number string has.

  ## Examples

      "3"    => 0
      "1.5"  => 1
      "0.25" => 2
  """
  def decimal_places(num_str) do
    case String.split(num_str, ".") do
      [_, decimal] -> String.length(decimal)
      _ -> 0
    end
  end

  @doc """
  Format a number with a specific number of decimal places.

  Elixir's `:erlang.float_to_binary/2` with the `decimals` option
  handles this, but we need to handle integers specially.
  """
  def format_number(value, 0) do
    # For zero precision, round and format as integer.
    Integer.to_string(round(value))
  end

  def format_number(value, precision) do
    :erlang.float_to_binary(value / 1, [{:decimals, precision}])
  end

  @doc """
  Generate the sequence of numbers.

  We compute each value as `first + i * increment` to avoid accumulating
  floating-point errors from repeated addition.
  """
  def generate_sequence(first, increment, last, precision) do
    if increment > 0 do
      do_generate_ascending(first, increment, last, precision, 0, [])
    else
      if increment < 0 do
        do_generate_descending(first, increment, last, precision, 0, [])
      else
        # Zero increment: empty sequence.
        []
      end
    end
  end

  defp do_generate_ascending(first, increment, last, precision, i, acc) do
    value = first + i * increment

    if value <= last + 1.0e-10 do
      formatted = format_number(value, precision)
      do_generate_ascending(first, increment, last, precision, i + 1, [formatted | acc])
    else
      Enum.reverse(acc)
    end
  end

  defp do_generate_descending(first, increment, last, precision, i, acc) do
    value = first + i * increment

    if value >= last - 1.0e-10 do
      formatted = format_number(value, precision)
      do_generate_descending(first, increment, last, precision, i + 1, [formatted | acc])
    else
      Enum.reverse(acc)
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp parse_number(str) do
    case Float.parse(str) do
      {value, _} -> value
      :error -> 0.0
    end
  end

  defp normalize_numbers(nil), do: []
  defp normalize_numbers(numbers) when is_list(numbers), do: numbers
  defp normalize_numbers(number) when is_binary(number), do: [number]

  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "seq.json"),
        else: nil
      ),
      "seq.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "seq.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find seq.json spec file"
  end
end
