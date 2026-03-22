defmodule UnixTools.DirnameTool do
  @moduledoc """
  dirname -- strip last component from file name.

  ## What This Program Does

  This is a reimplementation of the GNU `dirname` utility in Elixir. It
  outputs each NAME with its last non-slash component and trailing slashes
  removed. If NAME contains no slashes, it outputs "." (the current
  directory).

  ## How dirname Works

  dirname extracts the directory portion of a path:

      dirname /usr/bin/sort    =>   /usr/bin
      dirname stdio.h          =>   .
      dirname /usr/             =>   /
      dirname /                 =>   /

  ## The Algorithm (POSIX Specification)

  1. If the string is "//", some systems treat it specially. We treat
     it as "/".
  2. Remove trailing slashes.
  3. If there are no slashes remaining, return ".".
  4. Remove everything after the last slash.
  5. Remove trailing slashes (again).
  6. If the string is empty, return "/".
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

    case Parser.parse(spec_path, ["dirname" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        zero = !!flags["zero"]
        terminator = if zero, do: <<0>>, else: "\n"

        names = normalize_names(arguments["names"])

        Enum.each(names, fn name ->
          IO.write(compute_dirname(name) <> terminator)
        end)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "dirname: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Extract directory portion of a path.
  # ---------------------------------------------------------------------------

  @doc """
  Compute the dirname of a path according to the POSIX specification.

  ## Examples and Edge Cases

      Input          Output     Reason
      -----          ------     ------
      /usr/bin       /usr       Normal case
      /usr/          /          Trailing slash removed, then dirname
      usr            .          No slashes => current directory
      /              /          Root directory
      .              .          Current directory
      ..             .          Parent directory (dirname is cwd)
      (empty)        .          Empty string => current directory
  """
  def compute_dirname(pathname) do
    # Handle empty string.
    if pathname == "" do
      "."
    else
      # Step 1: Remove trailing slashes.
      stripped = String.trim_trailing(pathname, "/")

      # If removing trailing slashes left us empty, path was all slashes.
      if stripped == "" do
        "/"
      else
        # Step 2: Check if there's a slash in the string.
        if not String.contains?(stripped, "/") do
          # No slash found: entire path is a filename in current directory.
          "."
        else
          # Step 3: Remove everything after the last slash.
          # We split on "/" and rejoin all but the last segment.
          parts = String.split(stripped, "/")
          dir_parts = Enum.slice(parts, 0..(length(parts) - 2)//1)
          dir = Enum.join(dir_parts, "/")

          # Step 4: Remove trailing slashes from result.
          dir = String.trim_trailing(dir, "/")

          # Step 5: If empty, we were at root.
          if dir == "", do: "/", else: dir
        end
      end
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
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "dirname.json"),
        else: nil
      ),
      "dirname.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "dirname.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find dirname.json spec file"
  end
end
