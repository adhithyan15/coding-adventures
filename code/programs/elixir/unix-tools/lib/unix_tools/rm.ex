defmodule UnixTools.Rm do
  @moduledoc """
  rm -- remove files or directories.

  ## What This Program Does

  This is a reimplementation of the GNU `rm` utility in Elixir. It removes
  each specified file. By default, it does not remove directories.

  ## How rm Works

  rm permanently deletes files with no recycle bin or undo:

      rm file.txt               =>   removes file.txt
      rm -r directory/          =>   removes directory and everything in it
      rm -f nonexistent.txt     =>   no error even if file doesn't exist

  ## Safety Features

  Without `-r`, rm refuses to remove directories.
  Without `-f`, rm reports errors for nonexistent files.
  The `-d` flag allows removing empty directories (like rmdir).
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

    case Parser.parse(spec_path, ["rm" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        force = !!flags["force"]
        recursive = !!flags["recursive"]
        remove_empty_dirs = !!flags["dir"]
        verbose = !!flags["verbose"]

        file_list = normalize_files(arguments["files"])

        Enum.each(file_list, fn file ->
          remove_file(file, force, recursive, remove_empty_dirs, verbose)
        end)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "rm: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic: Remove files and directories.
  # ---------------------------------------------------------------------------

  @doc """
  Remove a single file or directory based on the flags.

  - Regular files are removed with `File.rm/1`.
  - Directories require `-r` (recursive) or `-d` (empty only).
  - With `-f`, missing files are silently ignored.
  """
  def remove_file(file, force, recursive, remove_empty_dirs, verbose) do
    case File.lstat(file) do
      {:ok, %{type: :directory}} ->
        cond do
          recursive ->
            File.rm_rf(file)

            if verbose do
              IO.puts("removed directory '#{file}'")
            end

          remove_empty_dirs ->
            case File.rmdir(file) do
              :ok ->
                if verbose, do: IO.puts("removed directory '#{file}'")

              {:error, reason} ->
                IO.puts(:stderr, "rm: cannot remove '#{file}': #{:file.format_error(reason)}")
            end

          true ->
            IO.puts(:stderr, "rm: cannot remove '#{file}': Is a directory")
        end

      {:ok, _stat} ->
        # Regular file, symlink, etc.
        case File.rm(file) do
          :ok ->
            if verbose, do: IO.puts("removed '#{file}'")

          {:error, reason} ->
            IO.puts(:stderr, "rm: cannot remove '#{file}': #{:file.format_error(reason)}")
        end

      {:error, :enoent} when force ->
        # With -f, ignore missing files.
        :ok

      {:error, reason} ->
        IO.puts(:stderr, "rm: cannot remove '#{file}': #{:file.format_error(reason)}")
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
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "rm.json"),
        else: nil
      ),
      "rm.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "rm.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find rm.json spec file"
  end
end
