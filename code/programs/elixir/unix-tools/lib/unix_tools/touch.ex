defmodule UnixTools.Touch do
  @moduledoc """
  touch -- change file timestamps.

  ## What This Program Does

  This is a reimplementation of the GNU `touch` utility in Elixir. It updates
  the access and modification times of each FILE to the current time. If the
  file does not exist, it is created (unless -c is given).

  ## How touch Works

  touch is most commonly used to create empty files:

      touch newfile.txt         =>   creates newfile.txt (empty)
      touch existing.txt        =>   updates timestamps
      touch -c noexist.txt      =>   does nothing (file doesn't exist)

  ## Time Selection

  By default, touch updates both access time and modification time. You can
  select just one with `-a` (access only) or `-m` (modification only).

  ## Custom Timestamps

  Instead of the current time, you can specify a time with:
  - `-d STRING`: Parse a date string.
  - `-r FILE`: Use another file's timestamps.
  - `-t STAMP`: Use [[CC]YY]MMDDhhmm[.ss] format.
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

    case Parser.parse(spec_path, ["touch" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        access_only = !!flags["access_only"]
        modification_only = !!flags["modification_only"]
        no_create = !!flags["no_create"]
        date_str = flags["date"]
        reference_file = flags["reference"]
        timestamp_str = flags["timestamp"]

        file_list = normalize_files(arguments["files"])

        # Determine the target time.
        target_time = determine_time(date_str, reference_file, timestamp_str)

        Enum.each(file_list, fn file ->
          touch_file(file, target_time, access_only, modification_only, no_create)
        end)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "touch: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Determine timestamp and touch files.
  # ---------------------------------------------------------------------------

  @doc """
  Determine the timestamp to use based on flags.

  Returns a DateTime or NaiveDateTime representing the desired time.
  If no time-source flag is given, uses the current time.
  """
  def determine_time(nil, nil, nil), do: NaiveDateTime.utc_now()

  def determine_time(date_str, nil, nil) when is_binary(date_str) do
    case NaiveDateTime.from_iso8601(date_str) do
      {:ok, dt} -> dt
      {:error, _} ->
        # Try parsing as a date only.
        case Date.from_iso8601(date_str) do
          {:ok, d} -> NaiveDateTime.new!(d, ~T[00:00:00])
          {:error, _} ->
            IO.puts(:stderr, "touch: invalid date format '#{date_str}'")
            System.halt(1)
        end
    end
  end

  def determine_time(nil, reference_file, nil) when is_binary(reference_file) do
    case File.stat(reference_file, time: :posix) do
      {:ok, stat} ->
        DateTime.from_unix!(stat.mtime) |> DateTime.to_naive()

      {:error, reason} ->
        IO.puts(:stderr, "touch: #{reference_file}: #{:file.format_error(reason)}")
        System.halt(1)
    end
  end

  def determine_time(nil, nil, timestamp_str) when is_binary(timestamp_str) do
    parse_timestamp(timestamp_str)
  end

  @doc """
  Parse a -t timestamp in [[CC]YY]MMDDhhmm[.ss] format.

  The format is positional and the century is inferred if omitted.
  """
  def parse_timestamp(stamp) do
    # Separate seconds if present.
    {main_part, seconds} =
      case String.split(stamp, ".") do
        [main, secs] -> {main, String.to_integer(secs)}
        [main] -> {main, 0}
      end

    {year, month, day, hour, minute} =
      case String.length(main_part) do
        8 ->
          # MMDDhhmm -- use current year.
          {DateTime.utc_now().year,
           String.slice(main_part, 0, 2) |> String.to_integer(),
           String.slice(main_part, 2, 2) |> String.to_integer(),
           String.slice(main_part, 4, 2) |> String.to_integer(),
           String.slice(main_part, 6, 2) |> String.to_integer()}

        10 ->
          # YYMMDDhhmm -- two-digit year.
          yy = String.slice(main_part, 0, 2) |> String.to_integer()
          full_year = if yy >= 69, do: 1900 + yy, else: 2000 + yy

          {full_year,
           String.slice(main_part, 2, 2) |> String.to_integer(),
           String.slice(main_part, 4, 2) |> String.to_integer(),
           String.slice(main_part, 6, 2) |> String.to_integer(),
           String.slice(main_part, 8, 2) |> String.to_integer()}

        12 ->
          # CCYYMMDDhhmm -- four-digit year.
          {String.slice(main_part, 0, 4) |> String.to_integer(),
           String.slice(main_part, 4, 2) |> String.to_integer(),
           String.slice(main_part, 6, 2) |> String.to_integer(),
           String.slice(main_part, 8, 2) |> String.to_integer(),
           String.slice(main_part, 10, 2) |> String.to_integer()}

        _ ->
          IO.puts(:stderr, "touch: invalid timestamp format '#{stamp}'")
          System.halt(1)
      end

    NaiveDateTime.new!(year, month, day, hour, minute, seconds)
  end

  @doc """
  Touch a single file: create if needed, then update timestamps.
  """
  def touch_file(file, target_time, access_only, modification_only, no_create) do
    file_exists = File.exists?(file)

    if not file_exists do
      if no_create do
        # -c: don't create, just skip.
        :ok
      else
        case File.write(file, "") do
          :ok -> update_timestamps(file, target_time, access_only, modification_only)
          {:error, reason} ->
            IO.puts(:stderr, "touch: #{file}: #{:file.format_error(reason)}")
        end
      end
    else
      update_timestamps(file, target_time, access_only, modification_only)
    end
  end

  defp update_timestamps(file, target_time, access_only, modification_only) do
    # Convert NaiveDateTime to posix seconds for File.touch.
    epoch = NaiveDateTime.diff(target_time, ~N[1970-01-01 00:00:00])

    # Determine which times to update.
    update_atime = access_only or (not access_only and not modification_only)
    update_mtime = modification_only or (not access_only and not modification_only)

    case File.stat(file, time: :posix) do
      {:ok, stat} ->
        new_atime = if update_atime, do: epoch, else: stat.atime
        new_mtime = if update_mtime, do: epoch, else: stat.mtime

        # Use :file.change_time to set both timestamps.
        :file.change_time(to_charlist(file), new_atime, new_mtime)

      {:error, reason} ->
        IO.puts(:stderr, "touch: #{file}: #{:file.format_error(reason)}")
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp normalize_files(files) when is_list(files), do: files
  defp normalize_files(file) when is_binary(file), do: [file]

  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "touch.json"),
        else: nil
      ),
      "touch.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "touch.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find touch.json spec file"
  end
end
