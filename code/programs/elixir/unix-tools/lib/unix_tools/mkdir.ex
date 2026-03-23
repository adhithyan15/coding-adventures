defmodule UnixTools.Mkdir do
  @moduledoc """
  mkdir -- make directories.

  ## What This Program Does

  This is a reimplementation of the GNU `mkdir` utility in Elixir. It creates
  one or more directories. With `-p`, it creates parent directories as needed
  and does not error if the directory already exists.

  ## How mkdir Works

  mkdir creates the named directories in the order given:

      mkdir dir1 dir2          =>   creates dir1 and dir2
      mkdir -p a/b/c           =>   creates a, a/b, and a/b/c
      mkdir -m 755 mydir       =>   creates mydir with mode 755

  ## The -p Flag (Parents)

  Without `-p`, mkdir fails if the directory already exists or if a parent
  directory doesn't exist. With `-p`, mkdir creates all necessary parent
  directories and does not complain if the directory already exists.

  ## Mode (-m)

  The `-m` flag sets the permission mode for the created directory. The mode
  is specified as an octal string (e.g., "755").
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

    case Parser.parse(spec_path, ["mkdir" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        parents = !!flags["parents"]
        verbose = !!flags["verbose"]
        mode_str = flags["mode"]

        dirs = normalize_dirs(arguments["directories"])

        Enum.each(dirs, fn dir ->
          create_directory(dir, parents, verbose, mode_str)
        end)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "mkdir: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Create directories.
  # ---------------------------------------------------------------------------

  @doc """
  Create a single directory, optionally with parents.

  With `parents` true, we use `File.mkdir_p/1` which creates all intermediate
  directories and doesn't error on existing directories. Without it, we use
  `File.mkdir/1` which fails if the directory exists or parents are missing.
  """
  def create_directory(dir, parents, verbose, mode_str) do
    result =
      if parents do
        File.mkdir_p(dir)
      else
        File.mkdir(dir)
      end

    case result do
      :ok ->
        # Set mode if specified.
        if mode_str do
          case parse_mode(mode_str) do
            {:ok, mode} -> File.chmod(dir, mode)
            {:error, reason} -> IO.puts(:stderr, "mkdir: #{reason}")
          end
        end

        if verbose do
          IO.puts("mkdir: created directory '#{dir}'")
        end

      {:error, reason} ->
        IO.puts(:stderr, "mkdir: cannot create directory '#{dir}': #{:file.format_error(reason)}")
    end
  end

  @doc """
  Parse an octal mode string like "755" into an integer.

  Octal is the traditional way Unix permissions are expressed:
    - 7 = rwx (read + write + execute)
    - 5 = r-x (read + execute)
    - 0 = --- (no permissions)
  """
  def parse_mode(mode_str) do
    case Integer.parse(mode_str, 8) do
      {mode, ""} -> {:ok, mode}
      _ -> {:error, "invalid mode: '#{mode_str}'"}
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp normalize_dirs(dirs) when is_list(dirs), do: dirs
  defp normalize_dirs(dir) when is_binary(dir), do: [dir]

  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "mkdir.json"),
        else: nil
      ),
      "mkdir.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "mkdir.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find mkdir.json spec file"
  end
end
