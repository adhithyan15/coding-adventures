defmodule UnixTools.IdTool do
  @moduledoc """
  id -- print real and effective user and group IDs.

  ## What This Program Does

  This is a reimplementation of the GNU `id` utility in Elixir. It prints
  information about the current user's identity: user ID (UID), group ID
  (GID), and all group memberships.

  ## How id Works

  With no flags, `id` prints the full identity string:

      id    =>   uid=501(adhithya) gid=20(staff) groups=20(staff),12(everyone),...

  With flags, specific parts can be selected:

      id -u          =>   501
      id -u -n       =>   adhithya
      id -g          =>   20
      id -g -n       =>   staff
      id -G          =>   20 12 61 ...
      id -G -n       =>   staff everyone localaccounts ...

  ## Module Name

  We use `IdTool` instead of `Id` to avoid conflicts with Elixir's
  built-in `id` function (the identity function). This is a common
  pattern when wrapping Unix utilities whose names clash with language
  keywords or built-ins.

  ## Implementation Approach

  We delegate to the system `id` command via `:os.cmd/1` and parse its
  output. This is pragmatic -- the underlying UID/GID information is
  deeply OS-specific and not exposed by Erlang/OTP in a structured way.

  The business logic (parsing id output) is kept as pure functions for
  testability.
  """

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult, ParseErrors}

  # ---------------------------------------------------------------------------
  # Business Logic
  # ---------------------------------------------------------------------------

  @doc """
  Get the current user's identity information.

  Returns a map with keys:
  - `:uid` - User ID (integer).
  - `:uid_name` - User name (string).
  - `:gid` - Primary group ID (integer).
  - `:gid_name` - Primary group name (string).
  - `:group_list` - List of `{gid, name}` tuples for all groups.

  ## How We Get the Information

  We call the system `id` command and parse its output. The output format
  on most Unix systems is:

      uid=501(username) gid=20(groupname) groups=20(groupname),12(other),...

  ## Implementation Note

  On macOS, the `id` command output is slightly different from Linux,
  but the uid=N(name) gid=N(name) groups=N(name),... format is universal
  across POSIX systems.
  """
  def get_user_info do
    raw = :os.cmd(~c"id") |> to_string() |> String.trim()
    parse_id_output(raw)
  end

  @doc """
  Get identity information for a specific user.

  Calls `id <username>` and parses the output.
  """
  def get_user_info(username) do
    raw = :os.cmd(~c"id #{username}") |> to_string() |> String.trim()
    parse_id_output(raw)
  end

  @doc """
  Parse the output of the `id` command into a structured map.

  ## Parsing Strategy

  The id output follows a predictable pattern:

      uid=NUMBER(NAME) gid=NUMBER(NAME) groups=NUMBER(NAME),NUMBER(NAME),...

  We use regex to extract each piece.

  ## Examples

      iex> UnixTools.IdTool.parse_id_output("uid=501(user) gid=20(staff) groups=20(staff),12(everyone)")
      %{uid: 501, uid_name: "user", gid: 20, gid_name: "staff",
        group_list: [{20, "staff"}, {12, "everyone"}]}
  """
  def parse_id_output(output) do
    uid_match = Regex.run(~r/uid=(\d+)\(([^)]+)\)/, output)
    gid_match = Regex.run(~r/gid=(\d+)\(([^)]+)\)/, output)
    groups_match = Regex.run(~r/groups=(.+)$/, output)

    {uid, uid_name} =
      case uid_match do
        [_, id_str, name] -> {String.to_integer(id_str), name}
        _ -> {0, "unknown"}
      end

    {gid, gid_name} =
      case gid_match do
        [_, id_str, name] -> {String.to_integer(id_str), name}
        _ -> {0, "unknown"}
      end

    group_list =
      case groups_match do
        [_, groups_str] -> parse_group_list(groups_str)
        _ -> []
      end

    %{
      uid: uid,
      uid_name: uid_name,
      gid: gid,
      gid_name: gid_name,
      group_list: group_list
    }
  end

  @doc """
  Parse the groups portion of id output.

  The groups string looks like: "20(staff),12(everyone),61(localaccounts)"

  ## Examples

      iex> UnixTools.IdTool.parse_group_list("20(staff),12(everyone)")
      [{20, "staff"}, {12, "everyone"}]
  """
  def parse_group_list(groups_str) do
    Regex.scan(~r/(\d+)\(([^)]+)\)/, groups_str)
    |> Enum.map(fn [_full, id_str, name] ->
      {String.to_integer(id_str), name}
    end)
  end

  @doc """
  Format the full identity string (default id output).

  Reconstructs the uid=N(name) gid=N(name) groups=N(name),... format.

  ## Examples

      iex> info = %{uid: 501, uid_name: "user", gid: 20, gid_name: "staff",
      ...>   group_list: [{20, "staff"}, {12, "everyone"}]}
      iex> UnixTools.IdTool.format_full(info)
      "uid=501(user) gid=20(staff) groups=20(staff),12(everyone)"
  """
  def format_full(info) do
    groups_str =
      info.group_list
      |> Enum.map(fn {gid, name} -> "#{gid}(#{name})" end)
      |> Enum.join(",")

    "uid=#{info.uid}(#{info.uid_name}) gid=#{info.gid}(#{info.gid_name}) groups=#{groups_str}"
  end

  # ---------------------------------------------------------------------------
  # Entry Point
  # ---------------------------------------------------------------------------

  @doc """
  Entry point. Receives `argv` as a list of strings.
  """
  def main(argv) do
    spec_path = resolve_spec_path()

    case Parser.parse(spec_path, ["id" | argv]) do
      {:ok, %HelpResult{text: text}} ->
        IO.puts(text)

      {:ok, %VersionResult{version: version}} ->
        IO.puts(version)

      {:ok, %ParseResult{flags: flags, arguments: arguments}} ->
        show_user = !!flags["user"]
        show_group = !!flags["group"]
        show_groups = !!flags["groups"]
        show_name = !!flags["name"]
        _show_real = !!flags["real"]

        info =
          case arguments["user_name"] do
            nil -> get_user_info()
            username -> get_user_info(username)
          end

        output =
          cond do
            show_user and show_name -> info.uid_name
            show_user -> Integer.to_string(info.uid)
            show_group and show_name -> info.gid_name
            show_group -> Integer.to_string(info.gid)
            show_groups and show_name ->
              info.group_list |> Enum.map(fn {_gid, name} -> name end) |> Enum.join(" ")
            show_groups ->
              info.group_list |> Enum.map(fn {gid, _name} -> Integer.to_string(gid) end) |> Enum.join(" ")
            true -> format_full(info)
          end

        IO.puts(output)

      {:error, %ParseErrors{errors: errors}} ->
        Enum.each(errors, fn e ->
          IO.puts(:stderr, "id: #{e.message}")
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
        do: Path.join(Mix.Project.config()[:lockfile] |> Path.dirname(), "id.json"),
        else: nil
      ),
      "id.json",
      Path.join(:code.priv_dir(:unix_tools) |> to_string(), "id.json")
    ]

    Enum.find(candidates, fn
      nil -> false
      candidate_path -> File.exists?(candidate_path)
    end) ||
      raise "Could not find id.json spec file"
  end
end
