defmodule UnixTools.Sleep do
  @moduledoc """
  sleep -- delay for a specified amount of time.

  ## What This Program Does

  This is a reimplementation of the GNU `sleep` utility in Elixir. It pauses
  execution for the specified duration, then exits.

  ## How sleep Works

      sleep 5       =>   pause for 5 seconds
      sleep 1.5     =>   pause for 1.5 seconds
      sleep 2m      =>   pause for 2 minutes
      sleep 1h 30m  =>   pause for 1 hour and 30 minutes

  ## Duration Suffixes

  Each duration argument can have an optional suffix:

      Suffix    Meaning     Multiplier
      ------    -------     ----------
      s         seconds     1          (default)
      m         minutes     60
      h         hours       3600
      d         days        86400

  If no suffix is given, seconds is assumed.

  ## Multiple Arguments Are Summed

  When multiple duration arguments are provided, their values are summed:

      sleep 1h 30m    =>   sleep for 5400 seconds (3600 + 1800)
      sleep 1 2 3     =>   sleep for 6 seconds (1 + 2 + 3)

  ## Floating Point Support

  Durations can be floating-point numbers:

      sleep 0.5     =>   sleep for 500 milliseconds
      sleep 1.5m    =>   sleep for 90 seconds

  ## Implementation Note

  We use `Process.sleep/1` which takes milliseconds as an integer. We
  convert the summed seconds to milliseconds and round to the nearest
  integer. Sub-millisecond precision is not supported (nor is it by the
  BEAM runtime).
  """

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Entry point
  # ---------------------------------------------------------------------------

  @doc """
  Entry point. Receives `argv` as a list of strings.

  ## How It Works

  1. Parse arguments with CLI Builder.
  2. Handle --help and --version.
  3. Parse each duration argument into seconds.
  4. Sum all durations.
  5. Convert to milliseconds and sleep.
  """
  def main(argv) do
    spec_path = resolve_spec_path()

    case Parser.parse(spec_path, ["sleep" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{arguments: arguments}} ->
        # -----------------------------------------------------------------------
        # Business logic: parse durations, sum, and sleep.
        # -----------------------------------------------------------------------

        durations = normalize_durations(arguments["duration"])

        # Parse each duration string and sum the results.
        total_seconds =
          Enum.reduce(durations, {:ok, 0.0}, fn duration_str, acc ->
            case acc do
              {:ok, total} ->
                case parse_duration(duration_str) do
                  {:ok, seconds} -> {:ok, total + seconds}
                  {:error, reason} -> {:error, reason}
                end

              error ->
                error
            end
          end)

        case total_seconds do
          {:ok, seconds} ->
            # Convert to milliseconds and sleep.
            milliseconds = round(seconds * 1000)
            Process.sleep(milliseconds)

          {:error, reason} ->
            IO.puts(:stderr, "sleep: #{reason}")
            System.halt(1)
        end

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "sleep: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Duration Parsing
  # ---------------------------------------------------------------------------

  @doc """
  Parse a duration string into seconds.

  A duration string is a number optionally followed by a suffix:
  - `s` or no suffix: seconds
  - `m`: minutes (multiply by 60)
  - `h`: hours (multiply by 3600)
  - `d`: days (multiply by 86400)

  Returns `{:ok, seconds}` on success or `{:error, message}` on failure.

  ## Algorithm

  1. Strip any trailing suffix character (s, m, h, d).
  2. Parse the remaining string as a float.
  3. Multiply by the appropriate factor.

  ## Examples

      iex> UnixTools.Sleep.parse_duration("5")
      {:ok, 5.0}

      iex> UnixTools.Sleep.parse_duration("5s")
      {:ok, 5.0}

      iex> UnixTools.Sleep.parse_duration("2m")
      {:ok, 120.0}

      iex> UnixTools.Sleep.parse_duration("1.5h")
      {:ok, 5400.0}

      iex> UnixTools.Sleep.parse_duration("1d")
      {:ok, 86400.0}

      iex> UnixTools.Sleep.parse_duration("abc")
      {:error, "invalid time interval 'abc'"}
  """
  def parse_duration(duration_str) do
    # -------------------------------------------------------------------------
    # Step 1: Identify the suffix and numeric part.
    # -------------------------------------------------------------------------
    # We check the last character. If it's a known suffix letter, we
    # strip it and use the corresponding multiplier. Otherwise, the
    # entire string is the number and we default to seconds.

    {numeric_str, multiplier} = extract_suffix(duration_str)

    # -------------------------------------------------------------------------
    # Step 2: Parse the numeric part as a float.
    # -------------------------------------------------------------------------

    case Float.parse(numeric_str) do
      {value, ""} when value >= 0 ->
        {:ok, value * multiplier}

      {value, _remainder} when value >= 0 ->
        # There was trailing garbage after the number.
        {:error, "invalid time interval '#{duration_str}'"}

      _ ->
        {:error, "invalid time interval '#{duration_str}'"}
    end
  end

  # ---------------------------------------------------------------------------
  # Suffix extraction
  # ---------------------------------------------------------------------------

  @doc """
  Extract the suffix from a duration string and return the numeric part
  and the corresponding multiplier.

  ## Suffix Table

      Suffix    Multiplier    Meaning
      ------    ----------    -------
      s         1.0           seconds
      m         60.0          minutes
      h         3600.0        hours
      d         86400.0       days
      (none)    1.0           seconds (default)
  """
  def extract_suffix(duration_str) do
    case String.last(duration_str) do
      "s" -> {String.slice(duration_str, 0..-2//1), 1.0}
      "m" -> {String.slice(duration_str, 0..-2//1), 60.0}
      "h" -> {String.slice(duration_str, 0..-2//1), 3600.0}
      "d" -> {String.slice(duration_str, 0..-2//1), 86400.0}
      _ -> {duration_str, 1.0}
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp normalize_durations(nil), do: []
  defp normalize_durations(durations) when is_list(durations), do: durations
  defp normalize_durations(duration) when is_binary(duration), do: [duration]

  @doc false
  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "sleep.json"),
        else: nil
      ),
      "sleep.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "sleep.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find sleep.json spec file"
  end
end
