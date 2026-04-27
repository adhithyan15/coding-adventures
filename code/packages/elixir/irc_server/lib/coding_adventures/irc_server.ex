defmodule CodingAdventures.IrcServer do
  @moduledoc """
  Pure IRC server state machine (channels, nicks, command dispatch).

  This module is the "brain" of an IRC server. It knows nothing about sockets,
  threads, or I/O — it is a *pure state machine* that consumes `Message` values
  and produces lists of `{conn_id, Message}` pairs that the transport layer
  should forward to the appropriate connections.

  ## Architecture

  State is a plain map containing:

  - `:server_name` — hostname shown in server-generated messages (e.g. "irc.local").
  - `:version`     — server version string.
  - `:motd`        — list of MOTD lines.
  - `:oper_password` — password for the OPER command.
  - `:clients`     — map from `conn_id` to client map.
  - `:channels`    — map from lowercase channel name to channel map.
  - `:nicks`       — map from lowercase nick to `conn_id` (for O(1) lookup).

  A **client map** has fields:
  `id`, `nick`, `username`, `realname`, `hostname`, `registered`,
  `channels` (MapSet of channel names), `away_message`, `is_oper`.

  A **channel map** has fields:
  `name` (canonical), `topic`, `members` (map conn_id -> member_map),
  `modes` (MapSet), `ban_list` (list).

  A **member map** has: `conn_id`, `is_op`, `is_voice`.

  ## Pure functional design

  Every public function takes state and returns `{new_state, responses}`.
  `responses` is a list of `{conn_id, Message.t()}` pairs. No GenServer
  required — wrap in one only if concurrent access is needed.

  ## RFC 1459 references

  Commands: https://www.rfc-editor.org/rfc/rfc1459#section-4
  Numerics: https://www.rfc-editor.org/rfc/rfc1459#section-6
  """

  alias CodingAdventures.IrcProto.Message

  # ---------------------------------------------------------------------------
  # Numeric reply constants (RFC 1459)
  # ---------------------------------------------------------------------------

  @rpl_welcome "001"
  @rpl_yourhost "002"
  @rpl_created "003"
  @rpl_myinfo "004"
  @rpl_luserclient "251"
  @rpl_away "301"
  @rpl_unaway "305"
  @rpl_nowaway "306"
  @rpl_whoisuser "311"
  @rpl_whoisserver "312"
  @rpl_endofwhois "318"
  @rpl_whoischannels "319"
  @rpl_liststart "321"
  @rpl_list "322"
  @rpl_listend "323"
  @rpl_channelmodeis "324"
  @rpl_notopic "331"
  @rpl_topic "332"
  @rpl_inviting "341"
  @rpl_endofwho "315"
  @rpl_whoreply "352"
  @rpl_namreply "353"
  @rpl_endofnames "366"
  @rpl_motdstart "375"
  @rpl_motd "372"
  @rpl_endofmotd "376"
  @rpl_youreoper "381"

  @err_nosuchnick "401"
  @err_nosuchchannel "403"
  @err_unknowncommand "421"
  @err_nonicknamegiven "431"
  @err_erroneusnickname "432"
  @err_nicknameinuse "433"
  @err_usernotinchannel "441"
  @err_notonchannel "442"
  @err_notregistered "451"
  @err_needmoreparams "461"
  @err_passwdmismatch "464"
  @err_chanoprivsneeded "482"

  # ---------------------------------------------------------------------------
  # State constructors
  # ---------------------------------------------------------------------------

  @doc """
  Create a new IRC server state map.

  ## Parameters

  - `server_name`    — hostname used as prefix in server-generated messages.
  - `version`        — server version string (default `"0.1.0"`).
  - `motd`           — list of MOTD line strings.
  - `oper_password`  — password for OPER command; empty string disables.

  ## Example

      iex> state = IrcServer.new("irc.local")
      iex> state.server_name
      "irc.local"
  """
  @spec new(String.t(), String.t(), [String.t()], String.t()) :: map()
  def new(server_name, version \\ "0.1.0", motd \\ ["Welcome."], oper_password \\ "") do
    %{
      server_name: server_name,
      version: version,
      motd: motd,
      oper_password: oper_password,
      clients: %{},
      channels: %{},
      nicks: %{}
    }
  end

  # ---------------------------------------------------------------------------
  # Lifecycle callbacks
  # ---------------------------------------------------------------------------

  @doc """
  Handle a new TCP connection.

  Creates a new unregistered client record for `conn_id`. Returns an empty
  response list (no server messages are sent until the client speaks).

  ## Returns

  `{new_state, []}`
  """
  @spec on_connect(map(), any(), String.t()) :: {map(), []}
  def on_connect(state, conn_id, host) do
    client = %{
      id: conn_id,
      nick: nil,
      username: nil,
      realname: nil,
      hostname: host,
      registered: false,
      channels: MapSet.new(),
      away_message: nil,
      is_oper: false
    }

    new_state = put_in(state, [:clients, conn_id], client)
    {new_state, []}
  end

  @doc """
  Handle a parsed IRC message from `conn_id`.

  Dispatches to the appropriate command handler. Returns `{new_state, responses}`.
  """
  @spec on_message(map(), any(), Message.t()) :: {map(), list()}
  def on_message(state, conn_id, %Message{command: cmd} = msg) do
    case cmd do
      "CAP"     -> {state, []}
      "NICK"    -> handle_nick(state, conn_id, msg)
      "USER"    -> handle_user(state, conn_id, msg)
      "PASS"    -> {state, []}
      "QUIT"    -> handle_quit(state, conn_id, msg)
      "JOIN"    -> handle_join(state, conn_id, msg)
      "PART"    -> handle_part(state, conn_id, msg)
      "PRIVMSG" -> handle_privmsg(state, conn_id, msg)
      "NOTICE"  -> handle_notice(state, conn_id, msg)
      "NAMES"   -> handle_names(state, conn_id, msg)
      "LIST"    -> handle_list(state, conn_id, msg)
      "TOPIC"   -> handle_topic(state, conn_id, msg)
      "KICK"    -> handle_kick(state, conn_id, msg)
      "INVITE"  -> handle_invite(state, conn_id, msg)
      "MODE"    -> handle_mode(state, conn_id, msg)
      "PING"    -> handle_ping(state, conn_id, msg)
      "PONG"    -> {state, []}
      "AWAY"    -> handle_away(state, conn_id, msg)
      "WHOIS"   -> handle_whois(state, conn_id, msg)
      "WHO"     -> handle_who(state, conn_id, msg)
      "OPER"    -> handle_oper(state, conn_id, msg)
      _         -> handle_unknown(state, conn_id, msg)
    end
  end

  @doc """
  Handle a disconnected connection (peer closed).

  Broadcasts QUIT to all channel members and cleans up state.

  ## Returns

  `{new_state, responses}`
  """
  @spec on_disconnect(map(), any()) :: {map(), list()}
  def on_disconnect(state, conn_id) do
    case Map.get(state.clients, conn_id) do
      nil -> {state, []}
      _client -> cleanup_client(state, conn_id, "Connection closed")
    end
  end

  # ---------------------------------------------------------------------------
  # Command handlers
  # ---------------------------------------------------------------------------

  defp handle_nick(state, conn_id, %Message{params: params}) do
    client = Map.get(state.clients, conn_id)
    if client == nil, do: throw({:unknown_conn, conn_id})

    case params do
      [] ->
        resp = smsg(state, @err_nonicknamegiven, ["No nickname given"])
        {state, [{conn_id, resp}]}

      [new_nick | _] ->
        if not valid_nick?(new_nick) do
          resp = smsg(state, @err_erroneusnickname, [new_nick, "Erroneous nickname"])
          {state, [{conn_id, resp}]}
        else
          lower_nick = String.downcase(new_nick)

          # Check if nick is already in use by someone else.
          existing = Map.get(state.nicks, lower_nick)
          if existing != nil and existing != conn_id do
            resp = smsg(state, @err_nicknameinuse, [new_nick, "Nickname is already in use"])
            {state, [{conn_id, resp}]}
          else
            old_nick = client.nick
            old_lower = if old_nick, do: String.downcase(old_nick), else: nil

            # Update nicks index.
            nicks2 = if old_lower, do: Map.delete(state.nicks, old_lower), else: state.nicks
            nicks3 = Map.put(nicks2, lower_nick, conn_id)

            client2 = %{client | nick: new_nick}
            state2 = %{state | nicks: nicks3, clients: Map.put(state.clients, conn_id, client2)}

            # Notify peers if this is a nick change (not initial NICK).
            responses =
              if old_nick != nil do
                nick_msg = %Message{prefix: mask(client), command: "NICK", params: [new_nick]}
                peers = unique_peers(state2, conn_id)
                Enum.map([conn_id | peers], fn pid -> {pid, nick_msg} end)
              else
                []
              end

            # Attempt registration.
            {state3, reg_responses} = maybe_register(state2, conn_id)
            {state3, responses ++ reg_responses}
          end
        end
    end
  end

  defp handle_user(state, conn_id, %Message{params: params}) do
    client = Map.get(state.clients, conn_id)
    if client == nil, do: throw({:unknown_conn, conn_id})

    if client.registered do
      {state, []}
    else
      case params do
        [username, _mode, _unused, realname | _] ->
          client2 = %{client | username: username, realname: realname}
          state2 = put_in(state, [:clients, conn_id], client2)
          maybe_register(state2, conn_id)

        _ ->
          resp = smsg(state, @err_needmoreparams, ["USER", "Not enough parameters"])
          {state, [{conn_id, resp}]}
      end
    end
  end

  defp handle_quit(state, conn_id, %Message{params: params}) do
    reason = List.first(params, "Quit")
    cleanup_client(state, conn_id, reason)
  end

  defp handle_join(state, conn_id, %Message{params: params}) do
    client = Map.get(state.clients, conn_id)
    if client == nil, do: throw({:unknown_conn, conn_id})

    if not client.registered do
      resp = smsg(state, @err_notregistered, ["You have not registered"])
      {state, [{conn_id, resp}]}
    else
      case params do
        [] ->
          resp = smsg(state, @err_needmoreparams, ["JOIN", "Not enough parameters"])
          {state, [{conn_id, resp}]}

        [channels_str | _] ->
          channels_str
          |> String.split(",")
          |> Enum.reduce({state, []}, fn chan_name, {s, resps} ->
            {s2, new_resps} = join_channel_safe(s, conn_id, chan_name)
            {s2, resps ++ new_resps}
          end)
      end
    end
  end

  defp join_channel(state, conn_id, chan_name) do
    client = Map.get(state.clients, conn_id)

    # Validate channel name starts with #
    unless String.starts_with?(chan_name, "#") do
      resp = smsg(state, @err_nosuchchannel, [chan_name, "No such channel"])
      return_early({state, [{conn_id, resp}]})
    end

    lower = String.downcase(chan_name)

    # Already in channel?
    if MapSet.member?(client.channels, lower) do
      return_early({state, []})
    end

    # Get or create channel.
    channel =
      Map.get(state.channels, lower, %{
        name: chan_name,
        topic: nil,
        members: %{},
        modes: MapSet.new(),
        ban_list: []
      })

    is_first = map_size(channel.members) == 0

    member = %{conn_id: conn_id, is_op: is_first, is_voice: false}
    channel2 = %{channel | members: Map.put(channel.members, conn_id, member)}
    client2 = %{client | channels: MapSet.put(client.channels, lower)}

    state2 =
      state
      |> put_in([:channels, lower], channel2)
      |> put_in([:clients, conn_id], client2)

    # Broadcast JOIN to all members (including the joining client).
    join_msg = %Message{prefix: mask(client2), command: "JOIN", params: [chan_name]}
    join_responses = Enum.map(channel2.members, fn {pid, _} -> {pid, join_msg} end)

    # Send topic (or RPL_NOTOPIC) to the joining client.
    topic_resp =
      if channel2.topic do
        [{conn_id, smsg(state2, @rpl_topic, [chan_name, channel2.topic])}]
      else
        [{conn_id, smsg(state2, @rpl_notopic, [chan_name, "No topic is set"])}]
      end

    # Send NAMES list.
    names_resps = names_responses(state2, conn_id, lower)

    {state2, join_responses ++ topic_resp ++ names_resps}
  end

  # Return from a function early (avoids deep nesting with a throw/catch pattern).
  defp return_early(value), do: throw({:early_return, value})

  defp with_early_return(fun) do
    try do
      fun.()
    catch
      {:early_return, value} -> value
    end
  end

  defp join_channel_safe(state, conn_id, chan_name) do
    with_early_return(fn -> join_channel(state, conn_id, chan_name) end)
  end

  defp handle_part(state, conn_id, %Message{params: params}) do
    client = Map.get(state.clients, conn_id)
    if client == nil, do: throw({:unknown_conn, conn_id})

    if not client.registered do
      resp = smsg(state, @err_notregistered, ["You have not registered"])
      {state, [{conn_id, resp}]}
    else
      case params do
        [] ->
          resp = smsg(state, @err_needmoreparams, ["PART", "Not enough parameters"])
          {state, [{conn_id, resp}]}

        [channels_str | rest] ->
          reason = List.first(rest, client.nick || "leaving")

          channels_str
          |> String.split(",")
          |> Enum.reduce({state, []}, fn chan_name, {s, resps} ->
            lower = String.downcase(chan_name)
            chan = Map.get(s.channels, lower)
            cli = Map.get(s.clients, conn_id)

            if chan == nil or not Map.has_key?(chan.members, conn_id) do
              resp = smsg(s, @err_notonchannel, [chan_name, "You're not on that channel"])
              {s, resps ++ [{conn_id, resp}]}
            else
              part_msg = %Message{prefix: mask(cli), command: "PART", params: [chan_name, reason]}
              part_resps = Enum.map(chan.members, fn {pid, _} -> {pid, part_msg} end)

              chan2 = %{chan | members: Map.delete(chan.members, conn_id)}
              cli2 = %{cli | channels: MapSet.delete(cli.channels, lower)}

              s2 =
                s
                |> put_in([:channels, lower], chan2)
                |> put_in([:clients, conn_id], cli2)

              # Remove empty channels.
              s3 = if map_size(chan2.members) == 0, do: %{s2 | channels: Map.delete(s2.channels, lower)}, else: s2

              {s3, resps ++ part_resps}
            end
          end)
      end
    end
  end

  defp handle_privmsg(state, conn_id, %Message{params: params}) do
    client = Map.get(state.clients, conn_id)
    if client == nil, do: throw({:unknown_conn, conn_id})

    if not client.registered do
      resp = smsg(state, @err_notregistered, ["You have not registered"])
      {state, [{conn_id, resp}]}
    else
      case params do
        [target, text] ->
          deliver_message(state, conn_id, "PRIVMSG", target, text)

        _ ->
          resp = smsg(state, @err_needmoreparams, ["PRIVMSG", "Not enough parameters"])
          {state, [{conn_id, resp}]}
      end
    end
  end

  defp handle_notice(state, conn_id, %Message{params: params}) do
    client = Map.get(state.clients, conn_id)
    if client == nil, do: throw({:unknown_conn, conn_id})

    if not client.registered do
      {state, []}
    else
      case params do
        [target, text] ->
          # NOTICE never generates automatic replies.
          {_state2, responses} = deliver_message(state, conn_id, "NOTICE", target, text)
          # Filter out any away replies.
          filtered = Enum.filter(responses, fn {_pid, msg} -> msg.command != @rpl_away end)
          {state, filtered}

        _ ->
          {state, []}
      end
    end
  end

  defp deliver_message(state, conn_id, cmd, target, text) do
    client = Map.get(state.clients, conn_id)
    msg = %Message{prefix: mask(client), command: cmd, params: [target, text]}

    if String.starts_with?(target, "#") do
      lower = String.downcase(target)
      chan = Map.get(state.channels, lower)

      if chan == nil do
        resp = smsg(state, @err_nosuchchannel, [target, "No such channel"])
        {state, [{conn_id, resp}]}
      else
        # Send to all channel members except sender.
        responses = for {pid, _} <- chan.members, pid != conn_id, do: {pid, msg}
        {state, responses}
      end
    else
      # Direct message to a nick.
      lower_target = String.downcase(target)
      target_id = Map.get(state.nicks, lower_target)

      if target_id == nil do
        resp = smsg(state, @err_nosuchnick, [target, "No such nick"])
        {state, [{conn_id, resp}]}
      else
        target_client = Map.get(state.clients, target_id)
        responses = [{target_id, msg}]

        # Notify sender if target is away (only for PRIVMSG, not NOTICE).
        away_responses =
          if target_client.away_message != nil and cmd == "PRIVMSG" do
            away_msg = smsg(state, @rpl_away, [target_client.nick, target_client.away_message])
            [{conn_id, away_msg}]
          else
            []
          end

        {state, responses ++ away_responses}
      end
    end
  end

  defp handle_names(state, conn_id, %Message{params: params}) do
    client = Map.get(state.clients, conn_id)
    if client == nil, do: throw({:unknown_conn, conn_id})

    case params do
      [] ->
        # NAMES with no args: list all channels.
        resps =
          Enum.flat_map(state.channels, fn {lower, _} ->
            names_responses(state, conn_id, lower)
          end)

        {state, resps}

      [channels_str | _] ->
        resps =
          channels_str
          |> String.split(",")
          |> Enum.flat_map(fn chan_name ->
            lower = String.downcase(chan_name)

            if Map.has_key?(state.channels, lower) do
              names_responses(state, conn_id, lower)
            else
              end_msg = smsg(state, @rpl_endofnames, [chan_name, "End of NAMES list"])
              [{conn_id, end_msg}]
            end
          end)

        {state, resps}
    end
  end

  defp names_responses(state, conn_id, lower_chan) do
    chan = Map.get(state.channels, lower_chan)

    if chan == nil do
      []
    else
      names =
        Enum.map(chan.members, fn {pid, member} ->
          c = Map.get(state.clients, pid)
          prefix = if member.is_op, do: "@", else: ""
          "#{prefix}#{c.nick}"
        end)
        |> Enum.join(" ")

      names_msg = smsg(state, @rpl_namreply, ["=", chan.name, names])
      end_msg = smsg(state, @rpl_endofnames, [chan.name, "End of NAMES list"])
      [{conn_id, names_msg}, {conn_id, end_msg}]
    end
  end

  defp handle_list(state, conn_id, _msg) do
    client = Map.get(state.clients, conn_id)
    if client == nil, do: throw({:unknown_conn, conn_id})

    start_resp = smsg(state, @rpl_liststart, ["Channel", "Users Name"])

    list_resps =
      Enum.map(state.channels, fn {_lower, chan} ->
        count = map_size(chan.members)
        topic = chan.topic || ""
        {conn_id, smsg(state, @rpl_list, [chan.name, Integer.to_string(count), topic])}
      end)

    end_resp = smsg(state, @rpl_listend, ["End of LIST"])

    {state, [{conn_id, start_resp}] ++ list_resps ++ [{conn_id, end_resp}]}
  end

  defp handle_topic(state, conn_id, %Message{params: params}) do
    client = Map.get(state.clients, conn_id)
    if client == nil, do: throw({:unknown_conn, conn_id})

    if not client.registered do
      resp = smsg(state, @err_notregistered, ["You have not registered"])
      {state, [{conn_id, resp}]}
    else
      case params do
        [] ->
          resp = smsg(state, @err_needmoreparams, ["TOPIC", "Not enough parameters"])
          {state, [{conn_id, resp}]}

        [chan_name] ->
          # Query topic.
          lower = String.downcase(chan_name)
          chan = Map.get(state.channels, lower)

          if chan == nil do
            resp = smsg(state, @err_nosuchchannel, [chan_name, "No such channel"])
            {state, [{conn_id, resp}]}
          else
            resp =
              if chan.topic,
                do: smsg(state, @rpl_topic, [chan_name, chan.topic]),
                else: smsg(state, @rpl_notopic, [chan_name, "No topic is set"])

            {state, [{conn_id, resp}]}
          end

        [chan_name, new_topic | _] ->
          lower = String.downcase(chan_name)
          chan = Map.get(state.channels, lower)

          cond do
            chan == nil ->
              resp = smsg(state, @err_nosuchchannel, [chan_name, "No such channel"])
              {state, [{conn_id, resp}]}

            not Map.has_key?(chan.members, conn_id) ->
              resp = smsg(state, @err_notonchannel, [chan_name, "You're not on that channel"])
              {state, [{conn_id, resp}]}

            true ->
              chan2 = %{chan | topic: new_topic}
              state2 = put_in(state, [:channels, lower], chan2)

              topic_msg = %Message{
                prefix: mask(client),
                command: "TOPIC",
                params: [chan_name, new_topic]
              }

              resps = Enum.map(chan2.members, fn {pid, _} -> {pid, topic_msg} end)
              {state2, resps}
          end
      end
    end
  end

  defp handle_kick(state, conn_id, %Message{params: params}) do
    client = Map.get(state.clients, conn_id)
    if client == nil, do: throw({:unknown_conn, conn_id})

    if not client.registered do
      resp = smsg(state, @err_notregistered, ["You have not registered"])
      {state, [{conn_id, resp}]}
    else
      case params do
        [chan_name, target_nick | rest] ->
          reason = List.first(rest, client.nick || "kicked")
          lower = String.downcase(chan_name)
          chan = Map.get(state.channels, lower)

          cond do
            chan == nil ->
              resp = smsg(state, @err_nosuchchannel, [chan_name, "No such channel"])
              {state, [{conn_id, resp}]}

            not Map.has_key?(chan.members, conn_id) ->
              resp = smsg(state, @err_notonchannel, [chan_name, "You're not on that channel"])
              {state, [{conn_id, resp}]}

            not Map.get(chan.members, conn_id, %{}).is_op ->
              resp = smsg(state, @err_chanoprivsneeded, [chan_name, "You're not channel operator"])
              {state, [{conn_id, resp}]}

            true ->
              lower_target = String.downcase(target_nick)
              target_id = Map.get(state.nicks, lower_target)
              target_member = if target_id, do: Map.get(chan.members, target_id), else: nil

              if target_member == nil do
                resp = smsg(state, @err_usernotinchannel, [target_nick, chan_name, "They aren't on that channel"])
                {state, [{conn_id, resp}]}
              else
                kick_msg = %Message{
                  prefix: mask(client),
                  command: "KICK",
                  params: [chan_name, target_nick, reason]
                }

                kick_resps = Enum.map(chan.members, fn {pid, _} -> {pid, kick_msg} end)

                # Remove target from channel.
                target_client = Map.get(state.clients, target_id)
                chan2 = %{chan | members: Map.delete(chan.members, target_id)}
                target_client2 = %{target_client | channels: MapSet.delete(target_client.channels, lower)}

                state2 =
                  state
                  |> put_in([:channels, lower], chan2)
                  |> put_in([:clients, target_id], target_client2)

                state3 = if map_size(chan2.members) == 0, do: %{state2 | channels: Map.delete(state2.channels, lower)}, else: state2

                {state3, kick_resps}
              end
          end

        _ ->
          resp = smsg(state, @err_needmoreparams, ["KICK", "Not enough parameters"])
          {state, [{conn_id, resp}]}
      end
    end
  end

  defp handle_invite(state, conn_id, %Message{params: params}) do
    client = Map.get(state.clients, conn_id)
    if client == nil, do: throw({:unknown_conn, conn_id})

    if not client.registered do
      resp = smsg(state, @err_notregistered, ["You have not registered"])
      {state, [{conn_id, resp}]}
    else
      case params do
        [target_nick, chan_name] ->
          _lower = String.downcase(chan_name)
          lower_target = String.downcase(target_nick)
          target_id = Map.get(state.nicks, lower_target)

          cond do
            target_id == nil ->
              resp = smsg(state, @err_nosuchnick, [target_nick, "No such nick"])
              {state, [{conn_id, resp}]}

            true ->
              invite_msg = %Message{
                prefix: mask(client),
                command: "INVITE",
                params: [target_nick, chan_name]
              }

              inviting_resp = smsg(state, @rpl_inviting, [target_nick, chan_name])
              {state, [{conn_id, inviting_resp}, {target_id, invite_msg}]}
          end

        _ ->
          resp = smsg(state, @err_needmoreparams, ["INVITE", "Not enough parameters"])
          {state, [{conn_id, resp}]}
      end
    end
  end

  defp handle_mode(state, conn_id, %Message{params: params}) do
    client = Map.get(state.clients, conn_id)
    if client == nil, do: throw({:unknown_conn, conn_id})

    if not client.registered do
      resp = smsg(state, @err_notregistered, ["You have not registered"])
      {state, [{conn_id, resp}]}
    else
      case params do
        [] ->
          resp = smsg(state, @err_needmoreparams, ["MODE", "Not enough parameters"])
          {state, [{conn_id, resp}]}

        [target | rest] ->
          if String.starts_with?(target, "#") do
            handle_channel_mode(state, conn_id, target, rest)
          else
            handle_user_mode(state, conn_id, target, rest)
          end
      end
    end
  end

  defp handle_channel_mode(state, conn_id, chan_name, rest) do
    lower = String.downcase(chan_name)
    chan = Map.get(state.channels, lower)

    if chan == nil do
      resp = smsg(state, @err_nosuchchannel, [chan_name, "No such channel"])
      {state, [{conn_id, resp}]}
    else
      case rest do
        [] ->
          # Query modes.
          mode_str = "+" <> (MapSet.to_list(chan.modes) |> Enum.sort() |> Enum.join(""))
          resp = smsg(state, @rpl_channelmodeis, [chan_name, mode_str])
          {state, [{conn_id, resp}]}

        [mode_str | _mode_args] ->
          # Set/unset modes.
          {action, modes} =
            case mode_str do
              "+" <> m -> {:add, String.graphemes(m)}
              "-" <> m -> {:remove, String.graphemes(m)}
              m -> {:add, String.graphemes(m)}
            end

          new_modes =
            Enum.reduce(modes, chan.modes, fn m, acc ->
              case action do
                :add -> MapSet.put(acc, m)
                :remove -> MapSet.delete(acc, m)
              end
            end)

          chan2 = %{chan | modes: new_modes}
          state2 = put_in(state, [:channels, lower], chan2)
          {state2, []}
      end
    end
  end

  defp handle_user_mode(state, conn_id, _target, _rest) do
    client = Map.get(state.clients, conn_id)
    if client == nil, do: throw({:unknown_conn, conn_id})
    resp = smsg(state, @rpl_channelmodeis, [client.nick || "*", "+"])
    {state, [{conn_id, resp}]}
  end

  defp handle_ping(state, conn_id, %Message{params: params}) do
    token = List.first(params, state.server_name)
    resp = smsg(state, "PONG", [state.server_name, token])
    {state, [{conn_id, resp}]}
  end

  defp handle_away(state, conn_id, %Message{params: params}) do
    client = Map.get(state.clients, conn_id)
    if client == nil, do: throw({:unknown_conn, conn_id})

    if not client.registered do
      resp = smsg(state, @err_notregistered, ["You have not registered"])
      {state, [{conn_id, resp}]}
    else
      case params do
        [] ->
          client2 = %{client | away_message: nil}
          state2 = put_in(state, [:clients, conn_id], client2)
          resp = smsg(state2, @rpl_unaway, ["You are no longer marked as being away"])
          {state2, [{conn_id, resp}]}

        [message | _] ->
          client2 = %{client | away_message: message}
          state2 = put_in(state, [:clients, conn_id], client2)
          resp = smsg(state2, @rpl_nowaway, ["You have been marked as being away"])
          {state2, [{conn_id, resp}]}
      end
    end
  end

  defp handle_whois(state, conn_id, %Message{params: params}) do
    client = Map.get(state.clients, conn_id)
    if client == nil, do: throw({:unknown_conn, conn_id})

    if not client.registered do
      resp = smsg(state, @err_notregistered, ["You have not registered"])
      {state, [{conn_id, resp}]}
    else
      target_nick = List.first(params, "")
      lower = String.downcase(target_nick)
      target_id = Map.get(state.nicks, lower)

      if target_id == nil do
        resp = smsg(state, @err_nosuchnick, [target_nick, "No such nick"])
        {state, [{conn_id, resp}]}
      else
        target = Map.get(state.clients, target_id)

        user_resp = smsg(state, @rpl_whoisuser, [
          target.nick,
          target.username || "~u",
          target.hostname,
          "*",
          target.realname || target.nick
        ])

        server_resp = smsg(state, @rpl_whoisserver, [target.nick, state.server_name, "IRC server"])

        chans_str =
          MapSet.to_list(target.channels)
          |> Enum.map(fn lower_chan ->
            chan = Map.get(state.channels, lower_chan)
            if chan do
              member = Map.get(chan.members, target_id, %{})
              prefix = if Map.get(member, :is_op, false), do: "@", else: ""
              "#{prefix}#{chan.name}"
            end
          end)
          |> Enum.filter(& &1)
          |> Enum.join(" ")

        chan_resp = smsg(state, @rpl_whoischannels, [target.nick, chans_str])
        end_resp = smsg(state, @rpl_endofwhois, [target.nick, "End of WHOIS list"])

        {state, [{conn_id, user_resp}, {conn_id, server_resp}, {conn_id, chan_resp}, {conn_id, end_resp}]}
      end
    end
  end

  defp handle_who(state, conn_id, %Message{params: params}) do
    client = Map.get(state.clients, conn_id)
    if client == nil, do: throw({:unknown_conn, conn_id})

    if not client.registered do
      resp = smsg(state, @err_notregistered, ["You have not registered"])
      {state, [{conn_id, resp}]}
    else
      mask_str = List.first(params, "*")

      targets =
        if String.starts_with?(mask_str, "#") do
          lower = String.downcase(mask_str)
          chan = Map.get(state.channels, lower)
          if chan, do: Map.keys(chan.members), else: []
        else
          # Wildcard match on nicks — return all for simplicity.
          Map.keys(state.clients)
        end

      resps =
        Enum.map(targets, fn pid ->
          c = Map.get(state.clients, pid)
          {conn_id, smsg(state, @rpl_whoreply, [
            mask_str,
            c.username || "~u",
            c.hostname,
            state.server_name,
            c.nick || "*",
            "H",
            "0 #{c.realname || c.nick || "*"}"
          ])}
        end)

      end_resp = {conn_id, smsg(state, @rpl_endofwho, [mask_str, "End of WHO list"])}
      {state, resps ++ [end_resp]}
    end
  end

  defp handle_oper(state, conn_id, %Message{params: params}) do
    client = Map.get(state.clients, conn_id)
    if client == nil, do: throw({:unknown_conn, conn_id})

    if not client.registered do
      resp = smsg(state, @err_notregistered, ["You have not registered"])
      {state, [{conn_id, resp}]}
    else
      case params do
        [_username, password] ->
          if state.oper_password != "" and password == state.oper_password do
            client2 = %{client | is_oper: true}
            state2 = put_in(state, [:clients, conn_id], client2)
            resp = smsg(state2, @rpl_youreoper, ["You are now an IRC operator"])
            {state2, [{conn_id, resp}]}
          else
            resp = smsg(state, @err_passwdmismatch, ["Password incorrect"])
            {state, [{conn_id, resp}]}
          end

        _ ->
          resp = smsg(state, @err_needmoreparams, ["OPER", "Not enough parameters"])
          {state, [{conn_id, resp}]}
      end
    end
  end

  defp handle_unknown(state, conn_id, %Message{command: cmd}) do
    client = Map.get(state.clients, conn_id)
    if client == nil, do: throw({:unknown_conn, conn_id})

    resp = smsg(state, @err_unknowncommand, [cmd, "Unknown command"])
    {state, [{conn_id, resp}]}
  end

  # ---------------------------------------------------------------------------
  # Registration logic
  # ---------------------------------------------------------------------------

  # Attempt to register the client if both NICK and USER have been received.
  defp maybe_register(state, conn_id) do
    client = Map.get(state.clients, conn_id)
    if client == nil, do: throw({:unknown_conn, conn_id})

    if client.nick != nil and client.username != nil and not client.registered do
      client2 = %{client | registered: true}
      state2 = put_in(state, [:clients, conn_id], client2)
      {state2, welcome_responses(state2, conn_id)}
    else
      {state, []}
    end
  end

  defp welcome_responses(state, conn_id) do
    client = Map.get(state.clients, conn_id)
    nick = client.nick

    welcome = smsg(state, @rpl_welcome, [nick, "Welcome to the IRC network #{nick}!#{client.username}@#{client.hostname}"])
    yourhost = smsg(state, @rpl_yourhost, [nick, "Your host is #{state.server_name}, running version #{state.version}"])
    created = smsg(state, @rpl_created, [nick, "This server was created today"])
    myinfo = smsg(state, @rpl_myinfo, [nick, state.server_name, state.version, "io", "biklmnopqstv"])
    luserclient = smsg(state, @rpl_luserclient, ["There are #{map_size(state.clients)} users on 1 server"])

    motd_start = smsg(state, @rpl_motdstart, [nick, "- #{state.server_name} Message of the Day -"])
    motd_lines = Enum.map(state.motd, fn line -> smsg(state, @rpl_motd, [nick, "- #{line}"]) end)
    motd_end = smsg(state, @rpl_endofmotd, [nick, "End of MOTD command"])

    msgs = [welcome, yourhost, created, myinfo, luserclient, motd_start] ++ motd_lines ++ [motd_end]
    Enum.map(msgs, fn m -> {conn_id, m} end)
  end

  # ---------------------------------------------------------------------------
  # Cleanup
  # ---------------------------------------------------------------------------

  defp cleanup_client(state, conn_id, reason) do
    client = Map.get(state.clients, conn_id)

    if client == nil do
      {state, []}
    else

    # Build QUIT message for broadcast.
    quit_msg = %Message{prefix: mask(client), command: "QUIT", params: [reason]}
    peers = unique_peers(state, conn_id)
    quit_resps = Enum.map(peers, fn pid -> {pid, quit_msg} end)

    # Part all channels.
    state2 =
      Enum.reduce(MapSet.to_list(client.channels), state, fn lower_chan, s ->
        case Map.get(s.channels, lower_chan) do
          nil -> s
          chan ->
            chan2 = %{chan | members: Map.delete(chan.members, conn_id)}
            if map_size(chan2.members) == 0 do
              %{s | channels: Map.delete(s.channels, lower_chan)}
            else
              put_in(s, [:channels, lower_chan], chan2)
            end
        end
      end)

    # Remove nick from index.
    nicks2 =
      if client.nick do
        Map.delete(state2.nicks, String.downcase(client.nick))
      else
        state2.nicks
      end

    state3 = %{state2 | nicks: nicks2, clients: Map.delete(state2.clients, conn_id)}
    {state3, quit_resps}
    end
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  # Build a server-sourced message (prefix is the server name).
  defp smsg(state, command, params) do
    %Message{prefix: state.server_name, command: command, params: params}
  end

  # Build a nick!user@host mask for a client.
  defp mask(%{nick: nick, username: username, hostname: hostname}) do
    "#{nick}!#{username || "~u"}@#{hostname || "unknown"}"
  end

  # Collect unique conn_ids of all peers sharing at least one channel with conn_id.
  defp unique_peers(state, conn_id) do
    client = Map.get(state.clients, conn_id)

    if client == nil do
      []
    else
      MapSet.to_list(client.channels)
      |> Enum.flat_map(fn lower_chan ->
        case Map.get(state.channels, lower_chan) do
          nil -> []
          chan -> Map.keys(chan.members)
        end
      end)
      |> Enum.uniq()
      |> Enum.filter(fn pid -> pid != conn_id end)
    end
  end

  # Validate nick: letters, digits, and a few special chars; max 30 chars.
  defp valid_nick?(nick) do
    byte_size(nick) > 0 and byte_size(nick) <= 30 and
      Regex.match?(~r/^[a-zA-Z\[\]\\`_^{|}][a-zA-Z0-9\[\]\\`_^{|}-]*$/, nick)
  end
end
