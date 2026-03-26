defmodule UnixTools.Logname do
  @moduledoc """
  logname -- print the user's login name.

  ## What This Program Does

  This is a reimplementation of the POSIX `logname` utility in Elixir. It
  prints the login name of the current user — the name used to log into
  the system, as opposed to the effective user name (which can change
  with `su` or `sudo`).

  ## How logname Works

      logname    =>   adhithya

  ## logname vs whoami

  The key difference is what each command reports:

  - `logname` prints the *login* user (who originally logged in)
  - `whoami` prints the *effective* user (who the process is running as)

  This matters when you use `su` or `sudo`:

      $ logname         =>   adhithya   (login user)
      $ whoami          =>   adhithya   (effective user)
      $ sudo logname    =>   adhithya   (login user unchanged!)
      $ sudo whoami     =>   root       (effective user is now root)

  ## Implementation

  We check the `$LOGNAME` environment variable first, then fall back to
  `$USER`. On most Unix systems, `$LOGNAME` is set by the login process
  and preserved across `su`/`sudo` invocations, while `$USER` reflects
  the effective user.

  POSIX specifies that `logname` should use the `getlogin()` C function,
  which reads from the utmp database. In Elixir, we approximate this by
  reading `$LOGNAME`, which is the standard way the login name is
  communicated to processes.
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
  3. Look up the login name and print it.
  """
  def main(argv) do
    spec_path = resolve_spec_path()

    case Parser.parse(spec_path, ["logname" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{}} ->
        # -----------------------------------------------------------------------
        # Business logic: print the login name.
        # -----------------------------------------------------------------------

        case get_login_name() do
          {:ok, login_name} ->
            IO.puts(login_name)

          :error ->
            IO.puts(:stderr, "logname: no login name")
            System.halt(1)
        end

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "logname: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Business Logic
  # ---------------------------------------------------------------------------

  @doc """
  Get the login name of the current user.

  Checks `$LOGNAME` first (the POSIX-standard variable set at login),
  then falls back to `$USER`. Returns `{:ok, name}` if found, or
  `:error` if neither variable is set.

  ## Why LOGNAME First?

  `$LOGNAME` is specifically designated by POSIX as the variable that
  holds the login name. It is set once at login and not changed by
  `su` or `sudo` (in most configurations). `$USER`, on the other hand,
  may reflect the effective user after a `su` or `sudo`.

  ## Examples

      iex> System.put_env("LOGNAME", "testuser")
      iex> UnixTools.Logname.get_login_name()
      {:ok, "testuser"}
  """
  def get_login_name do
    case System.get_env("LOGNAME") do
      nil ->
        # Fall back to $USER if $LOGNAME is not set.
        case System.get_env("USER") do
          nil -> :error
          user -> {:ok, user}
        end

      login_name ->
        {:ok, login_name}
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  @doc false
  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "logname.json"),
        else: nil
      ),
      "logname.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "logname.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find logname.json spec file"
  end
end
