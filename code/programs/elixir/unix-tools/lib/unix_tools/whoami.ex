defmodule UnixTools.Whoami do
  @moduledoc """
  whoami -- print the current user name.

  ## What This Program Does

  This is a reimplementation of the GNU `whoami` utility in Elixir. It prints
  the user name associated with the current effective user ID.

  ## How whoami Works

  At its simplest:

      whoami    =>   adhithya

  The program reads the `USER` environment variable to determine the
  current user name. This is the standard approach on Unix-like systems.

  ## whoami vs id

  Both commands can tell you who you are, but they differ:

  - `whoami` prints just the user name of the effective user ID
  - `id` prints user ID, group ID, and all group memberships

  ## whoami vs logname

  - `whoami` prints the *effective* user (which changes with `su` or `sudo`)
  - `logname` prints the *login* user (the original user who logged in)

  For example:

      $ whoami          =>   adhithya
      $ sudo whoami     =>   root       (effective user changed)
      $ sudo logname    =>   adhithya   (login user unchanged)

  ## Implementation Note

  We use `System.get_env("USER")` which reads the `$USER` environment
  variable. On most Unix systems, this is set by the login process and
  accurately reflects the effective user. An alternative would be to use
  `:os.cmd(~c"id -un")` to call the system `id` command, but reading
  the environment variable is simpler and sufficient for our purposes.
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
  3. Look up the current user name and print it.
  """
  def main(argv) do
    spec_path = resolve_spec_path()

    case Parser.parse(spec_path, ["whoami" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{}} ->
        # -----------------------------------------------------------------------
        # Business logic: print the current user name.
        # -----------------------------------------------------------------------

        case get_username() do
          {:ok, username} ->
            IO.puts(username)

          :error ->
            IO.puts(:stderr, "whoami: cannot find name for user ID")
            System.halt(1)
        end

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "whoami: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic
  # ---------------------------------------------------------------------------

  @doc """
  Get the current user name.

  Reads the `$USER` environment variable. Returns `{:ok, username}` if
  the variable is set, or `:error` if it is not.

  ## Why $USER?

  The `$USER` environment variable is set by the login process on all
  major Unix-like systems (Linux, macOS, BSDs). It reflects the effective
  user — the user whose permissions the current process is running with.

  ## Examples

      iex> System.put_env("USER", "testuser")
      iex> UnixTools.Whoami.get_username()
      {:ok, "testuser"}
  """
  def get_username do
    case System.get_env("USER") do
      nil -> :error
      username -> {:ok, username}
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  @doc false
  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "whoami.json"),
        else: nil
      ),
      "whoami.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "whoami.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find whoami.json spec file"
  end
end
