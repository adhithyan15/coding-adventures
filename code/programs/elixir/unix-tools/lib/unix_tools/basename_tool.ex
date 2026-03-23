defmodule UnixTools.BasenameTool do
  @moduledoc """
  basename -- strip directory and suffix from filenames.

  ## What This Program Does

  This is a reimplementation of the GNU `basename` utility in Elixir. It
  prints NAME with any leading directory components removed. If a SUFFIX
  is specified, it is also removed from the end.

  ## How basename Works

  basename extracts the filename from a path:

      basename /usr/bin/sort         =>   sort
      basename include/stdio.h .h   =>   stdio
      basename -s .h include/stdio.h  =>   stdio

  ## The Algorithm

  1. Remove all trailing slashes from the path.
  2. If the entire path was slashes, return "/".
  3. Remove the directory prefix (everything up to the last slash).
  4. If a suffix is specified and the name ends with it (and the name
     is not equal to the suffix), remove the suffix.

  ## Multiple Mode (-a)

  By default, basename processes only one NAME argument (with an optional
  second argument as the suffix). With `-a` or `-s SUFFIX`, it processes
  all arguments as names, applying the same suffix removal to each.
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

    case Parser.parse(spec_path, ["basename" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        multiple = !!flags["multiple"]
        suffix_flag = flags["suffix"]
        zero = !!flags["zero"]
        terminator = if zero, do: <<0>>, else: "\n"

        # Get the name arguments.
        names = normalize_names(arguments["name"])

        # Determine suffix and names to process.
        # Two calling conventions:
        #   basename NAME [SUFFIX]         -- single name, optional suffix
        #   basename -a [-s SUFFIX] NAME...  -- multiple names
        {names_to_process, suffix_to_use} =
          if multiple or suffix_flag != nil do
            {names, suffix_flag}
          else
            if length(names) == 2 do
              {[List.first(names)], List.last(names)}
            else
              {[List.first(names)], nil}
            end
          end

        Enum.each(names_to_process, fn name ->
          IO.write(compute_basename(name, suffix_to_use) <> terminator)
        end)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "basename: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Strip directory and suffix.
  # ---------------------------------------------------------------------------

  @doc """
  Compute the basename of a path, optionally stripping a suffix.

  ## Step-by-Step

  1. Strip trailing slashes: "/usr/bin/" -> "/usr/bin"
  2. Handle all-slashes case: "///" -> "/"
  3. Take everything after the last slash: "/usr/bin" -> "bin"
  4. Strip suffix if it matches and doesn't consume entire name.

  ## Examples

      compute_basename("/usr/bin/sort", nil)     => "sort"
      compute_basename("stdio.h", ".h")          => "stdio"
      compute_basename("/", nil)                 => "/"
      compute_basename(".h", ".h")               => ".h"
  """
  def compute_basename(pathname, suffix \\ nil) do
    # Step 1: Remove trailing slashes.
    stripped = String.trim_trailing(pathname, "/")

    # Step 2: If empty after stripping, the path was all slashes.
    if stripped == "" do
      "/"
    else
      # Step 3: Remove directory prefix.
      name =
        if not String.contains?(stripped, "/") do
          stripped
        else
          parts = String.split(stripped, "/")
          List.last(parts)
        end

      do_strip_suffix(name, suffix)
    end
  end

  defp do_strip_suffix(name, suffix) do

    # Step 4: Remove suffix if specified.
    if suffix != nil and suffix != "" and name != suffix and String.ends_with?(name, suffix) do
      String.slice(name, 0, String.length(name) - String.length(suffix))
    else
      name
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp normalize_names(nil), do: []
  defp normalize_names(names) when is_list(names), do: names
  defp normalize_names(name) when is_binary(name), do: [name]

  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "basename.json"),
        else: nil
      ),
      "basename.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "basename.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find basename.json spec file"
  end
end
