defmodule UnixTools.Groups do
  @moduledoc """
  groups -- print the groups a user is in.

  ## What This Program Does

  This is a reimplementation of the GNU `groups` utility in Elixir. It prints
  the list of groups that a user belongs to.

  ## How groups Works

  With no arguments, it prints the groups of the current user:

      groups    =>   staff everyone localaccounts ...

  With one or more usernames, it prints each user's groups:

      groups alice bob  =>   alice : staff everyone
                              bob : staff admin

  ## groups vs id -Gn

  Both commands show group memberships, but the output format differs:

  - `groups` prints "username : group1 group2 ..." (when given a username)
  - `id -Gn` prints "group1 group2 ..." (no username prefix)

  When no username is given, `groups` omits the "username :" prefix,
  making its output identical to `id -Gn`.

  ## Implementation Approach

  We delegate to the system `groups` command via `:os.cmd/1`. This is
  the most portable approach since group membership lookup requires
  OS-specific system calls (getgrouplist, etc.) not directly exposed
  by Erlang/OTP.
  """

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Business Logic
  # ---------------------------------------------------------------------------

  @doc """
  Get the groups for the current user.

  Returns a list of group name strings.

  ## How It Works

  Calls the system `groups` command and splits the output on whitespace.

  ## Examples

      iex> groups = UnixTools.Groups.get_groups()
      iex> is_list(groups)
      true
  """
  def get_groups do
    :os.cmd(~c"groups")
    |> to_string()
    |> String.trim()
    |> String.split(~r/\s+/)
  end

  @doc """
  Get the groups for a specific user.

  Returns `{:ok, groups}` if the user exists, or `{:error, message}`
  if the user is not found.

  ## How It Works

  Calls `groups <username>` and parses the output. On macOS, the output
  format is "username : group1 group2 ...". On Linux it may just be
  "group1 group2 ...".
  """
  def get_groups(username) do
    raw = :os.cmd(~c"groups #{username}") |> to_string() |> String.trim()

    cond do
      String.contains?(raw, "no such user") ->
        {:error, "groups: '#{username}': no such user"}

      # macOS format: "username : group1 group2 ..."
      String.contains?(raw, " : ") ->
        [_user_part, groups_part] = String.split(raw, " : ", parts: 2)
        {:ok, String.split(groups_part, ~r/\s+/)}

      true ->
        {:ok, String.split(raw, ~r/\s+/)}
    end
  end

  @doc """
  Parse the raw output of the `groups` command.

  Handles both the "username : group1 group2" format (macOS) and
  the plain "group1 group2" format (Linux).

  ## Examples

      iex> UnixTools.Groups.parse_groups_output("staff everyone admin")
      ["staff", "everyone", "admin"]

      iex> UnixTools.Groups.parse_groups_output("alice : staff admin")
      ["staff", "admin"]
  """
  def parse_groups_output(output) do
    trimmed = String.trim(output)

    if String.contains?(trimmed, " : ") do
      [_user, groups_part] = String.split(trimmed, " : ", parts: 2)
      String.split(groups_part, ~r/\s+/)
    else
      String.split(trimmed, ~r/\s+/)
    end
  end

  # ---------------------------------------------------------------------------
  # Entry Point
  # ---------------------------------------------------------------------------

  @doc """
  Entry point. Receives `argv` as a list of strings.
  """
  def main(argv) do
    spec_path = resolve_spec_path()

    case Parser.parse(spec_path, ["groups" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{arguments: arguments}} ->
        users = arguments["users"]

        case users do
          nil ->
            # No username given: print current user's groups.
            group_names = get_groups()
            IO.puts(Enum.join(group_names, " "))

          usernames when is_list(usernames) ->
            Enum.each(usernames, fn username ->
              case get_groups(username) do
                {:ok, group_names} ->
                  IO.puts("#{username} : #{Enum.join(group_names, " ")}")

                {:error, msg} ->
                  IO.puts(:stderr, msg)
              end
            end)

          username when is_binary(username) ->
            case get_groups(username) do
              {:ok, group_names} ->
                IO.puts("#{username} : #{Enum.join(group_names, " ")}")

              {:error, msg} ->
                IO.puts(:stderr, msg)
            end
        end

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "groups: #{e.message}")
        end)

        System.halt(1)
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  @doc false
  defp resolve_spec_path do
    candidates = [
      if(function_exported?(Mix, :Project, 0),
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "groups.json"),
        else: nil
      ),
      "groups.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "groups.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find groups.json spec file"
  end
end
