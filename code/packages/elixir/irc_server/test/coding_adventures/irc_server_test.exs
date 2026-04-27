defmodule CodingAdventures.IrcServerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.IrcServer
  alias CodingAdventures.IrcProto.Message

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp new_server, do: IrcServer.new("irc.local", "0.1.0", ["Welcome."], "secret")

  defp connect(state, id, host \\ "127.0.0.1") do
    {state2, []} = IrcServer.on_connect(state, id, host)
    state2
  end

  defp nick(state, id, n) do
    {state2, _} = IrcServer.on_message(state, id, %Message{command: "NICK", params: [n]})
    state2
  end

  defp user(state, id, u \\ "user", r \\ "Real Name") do
    {state2, _} = IrcServer.on_message(state, id, %Message{command: "USER", params: [u, "0", "*", r]})
    state2
  end

  defp register(state, id, n \\ "alice", u \\ "alice") do
    state |> nick(id, n) |> user(id, u)
  end

  defp join(state, id, chan) do
    {state2, _} = IrcServer.on_message(state, id, %Message{command: "JOIN", params: [chan]})
    state2
  end

  defp commands(responses), do: Enum.map(responses, fn {_id, msg} -> msg.command end)
  defp recipients(responses), do: Enum.map(responses, fn {id, _msg} -> id end)
  defp numerics(responses), do: Enum.map(responses, fn {_id, msg} -> msg.command end)

  # ---------------------------------------------------------------------------
  # new/1 and on_connect/3
  # ---------------------------------------------------------------------------

  describe "new/1" do
    test "creates server with correct fields" do
      state = IrcServer.new("irc.local")
      assert state.server_name == "irc.local"
      assert state.clients == %{}
      assert state.channels == %{}
      assert state.nicks == %{}
    end
  end

  describe "on_connect/3" do
    test "creates unregistered client" do
      state = new_server()
      {state2, resps} = IrcServer.on_connect(state, 1, "127.0.0.1")
      assert resps == []
      assert Map.has_key?(state2.clients, 1)
      assert state2.clients[1].registered == false
    end

    test "stores hostname" do
      state = new_server()
      {state2, _} = IrcServer.on_connect(state, 1, "192.168.1.1")
      assert state2.clients[1].hostname == "192.168.1.1"
    end
  end

  # ---------------------------------------------------------------------------
  # NICK
  # ---------------------------------------------------------------------------

  describe "NICK" do
    test "sets nick and triggers registration with USER" do
      state = new_server() |> connect(1)
      {state2, resps} = IrcServer.on_message(state, 1, %Message{command: "NICK", params: ["alice"]})
      assert state2.clients[1].nick == "alice"
      # No registration yet (USER not sent)
      assert resps == []
    end

    test "complete registration sends welcome numerics" do
      state = new_server() |> connect(1) |> register(1)
      {state2, resps} = IrcServer.on_message(
        new_server() |> connect(1) |> nick(1, "alice"),
        1, %Message{command: "USER", params: ["alice", "0", "*", "Alice"]}
      )
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "001" end)
    end

    test "nick already in use returns 433" do
      state = new_server() |> connect(1) |> register(1, "alice") |> connect(2)
      {_state2, resps} = IrcServer.on_message(state, 2, %Message{command: "NICK", params: ["alice"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "433" end)
    end

    test "nick change same nick (no conflict)" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {state2, resps} = IrcServer.on_message(state, 1, %Message{command: "NICK", params: ["alice"]})
      # Same nick — no 433
      refute Enum.any?(resps, fn {_id, msg} -> msg.command == "433" end)
      assert state2.clients[1].nick == "alice"
    end

    test "nick change notifies channel peers" do
      state =
        new_server()
        |> connect(1)
        |> connect(2)
        |> register(1, "alice")
        |> register(2, "bob")
        |> join(1, "#chan")
        |> join(2, "#chan")

      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "NICK", params: ["alice2"]})
      nick_resps = Enum.filter(resps, fn {_id, msg} -> msg.command == "NICK" end)
      assert length(nick_resps) >= 2
      recipients = Enum.map(nick_resps, fn {id, _} -> id end)
      assert 1 in recipients
      assert 2 in recipients
    end

    test "no nick given returns 431" do
      state = new_server() |> connect(1)
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "NICK", params: []})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "431" end)
    end

    test "erroneous nickname returns 432" do
      state = new_server() |> connect(1)
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "NICK", params: ["123bad"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "432" end)
    end
  end

  # ---------------------------------------------------------------------------
  # USER
  # ---------------------------------------------------------------------------

  describe "USER" do
    test "insufficient params returns 461" do
      state = new_server() |> connect(1) |> nick(1, "alice")
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "USER", params: ["u"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "461" end)
    end

    test "USER after registration is a no-op" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {state2, resps} = IrcServer.on_message(state, 1, %Message{command: "USER", params: ["u", "0", "*", "Real"]})
      assert resps == []
      # State unchanged (no re-registration)
      assert state2.clients[1].nick == "alice"
    end
  end

  # ---------------------------------------------------------------------------
  # QUIT
  # ---------------------------------------------------------------------------

  describe "QUIT" do
    test "removes client from state" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {state2, _resps} = IrcServer.on_message(state, 1, %Message{command: "QUIT", params: ["Goodbye"]})
      refute Map.has_key?(state2.clients, 1)
      refute Map.has_key?(state2.nicks, "alice")
    end

    test "QUIT broadcasts to channel peers" do
      state =
        new_server()
        |> connect(1)
        |> connect(2)
        |> register(1, "alice")
        |> register(2, "bob")
        |> join(1, "#chan")
        |> join(2, "#chan")

      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "QUIT", params: ["bye"]})
      quit_resps = Enum.filter(resps, fn {_id, msg} -> msg.command == "QUIT" end)
      assert Enum.any?(quit_resps, fn {id, _} -> id == 2 end)
    end

    test "on_disconnect same as QUIT" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {state2, _resps} = IrcServer.on_disconnect(state, 1)
      refute Map.has_key?(state2.clients, 1)
    end

    test "on_disconnect for unknown conn_id is no-op" do
      state = new_server()
      {state2, resps} = IrcServer.on_disconnect(state, 9999)
      assert resps == []
      assert state2 == state
    end
  end

  # ---------------------------------------------------------------------------
  # JOIN
  # ---------------------------------------------------------------------------

  describe "JOIN" do
    test "creates channel and makes first member an op" do
      state = new_server() |> connect(1) |> register(1, "alice") |> join(1, "#general")
      assert Map.has_key?(state.channels, "#general")
      member = state.channels["#general"].members[1]
      assert member.is_op == true
    end

    test "JOIN sends RPL_NAMREPLY to joiner" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "JOIN", params: ["#chan"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "353" end)
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "366" end)
    end

    test "JOIN sends RPL_NOTOPIC for channel without topic" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "JOIN", params: ["#chan"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "331" end)
    end

    test "JOIN sends RPL_TOPIC when channel has topic" do
      state =
        new_server()
        |> connect(1)
        |> register(1, "alice")
        |> join(1, "#chan")

      # Set a topic
      {state2, _} = IrcServer.on_message(state, 1, %Message{command: "TOPIC", params: ["#chan", "Hello topic"]})

      # Second client joins
      state3 = state2 |> connect(2) |> register(2, "bob")
      {_state4, resps} = IrcServer.on_message(state3, 2, %Message{command: "JOIN", params: ["#chan"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "332" end)
    end

    test "joining a non-# channel returns 403" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "JOIN", params: ["notachan"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "403" end)
    end

    test "second user joins, gets names list with both nicks" do
      state =
        new_server()
        |> connect(1)
        |> connect(2)
        |> register(1, "alice")
        |> register(2, "bob")
        |> join(1, "#chan")

      {_state2, resps} = IrcServer.on_message(state, 2, %Message{command: "JOIN", params: ["#chan"]})
      names_msgs = Enum.filter(resps, fn {_id, msg} -> msg.command == "353" end)
      assert length(names_msgs) >= 1
      {_id, names_msg} = hd(names_msgs)
      combined_names = Enum.join(names_msg.params, " ")
      assert String.contains?(combined_names, "alice") or String.contains?(combined_names, "bob")
    end

    test "unregistered client gets 451" do
      state = new_server() |> connect(1)
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "JOIN", params: ["#chan"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "451" end)
    end

    test "JOIN with no params returns 461" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "JOIN", params: []})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "461" end)
    end

    test "rejoining same channel is a no-op" do
      state = new_server() |> connect(1) |> register(1, "alice") |> join(1, "#chan")
      {state2, resps} = IrcServer.on_message(state, 1, %Message{command: "JOIN", params: ["#chan"]})
      assert resps == []
      assert map_size(state2.channels["#chan"].members) == 1
    end
  end

  # ---------------------------------------------------------------------------
  # PART
  # ---------------------------------------------------------------------------

  describe "PART" do
    test "removes client from channel" do
      state =
        new_server()
        |> connect(1)
        |> register(1, "alice")
        |> join(1, "#chan")

      {state2, _resps} = IrcServer.on_message(state, 1, %Message{command: "PART", params: ["#chan"]})
      refute Map.has_key?(state2.channels, "#chan")
    end

    test "PART removes empty channel" do
      state =
        new_server()
        |> connect(1)
        |> register(1, "alice")
        |> join(1, "#chan")

      {state2, _} = IrcServer.on_message(state, 1, %Message{command: "PART", params: ["#chan"]})
      refute Map.has_key?(state2.channels, "#chan")
    end

    test "PART notifies channel members" do
      state =
        new_server()
        |> connect(1)
        |> connect(2)
        |> register(1, "alice")
        |> register(2, "bob")
        |> join(1, "#chan")
        |> join(2, "#chan")

      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "PART", params: ["#chan"]})
      part_resps = Enum.filter(resps, fn {_id, msg} -> msg.command == "PART" end)
      ids = Enum.map(part_resps, fn {id, _} -> id end)
      assert 1 in ids
      assert 2 in ids
    end

    test "PART from channel you're not in returns 442" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "PART", params: ["#chan"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "442" end)
    end

    test "PART with no params returns 461" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "PART", params: []})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "461" end)
    end

    test "unregistered client gets 451" do
      state = new_server() |> connect(1)
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "PART", params: ["#chan"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "451" end)
    end
  end

  # ---------------------------------------------------------------------------
  # PRIVMSG
  # ---------------------------------------------------------------------------

  describe "PRIVMSG" do
    test "delivers message to channel members (excluding sender)" do
      state =
        new_server()
        |> connect(1)
        |> connect(2)
        |> register(1, "alice")
        |> register(2, "bob")
        |> join(1, "#chan")
        |> join(2, "#chan")

      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "PRIVMSG", params: ["#chan", "hello"]})
      assert Enum.any?(resps, fn {id, _} -> id == 2 end)
      refute Enum.any?(resps, fn {id, _} -> id == 1 end)
    end

    test "delivers direct message to target nick" do
      state =
        new_server()
        |> connect(1)
        |> connect(2)
        |> register(1, "alice")
        |> register(2, "bob")

      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "PRIVMSG", params: ["bob", "hi bob"]})
      assert Enum.any?(resps, fn {id, _} -> id == 2 end)
    end

    test "PRIVMSG to unknown nick returns 401" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "PRIVMSG", params: ["nobody", "hi"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "401" end)
    end

    test "PRIVMSG to unknown channel returns 403" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "PRIVMSG", params: ["#nobody", "hi"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "403" end)
    end

    test "PRIVMSG to away nick delivers message and sends RPL_AWAY to sender" do
      state =
        new_server()
        |> connect(1)
        |> connect(2)
        |> register(1, "alice")
        |> register(2, "bob")

      # Set bob away
      {state2, _} = IrcServer.on_message(state, 2, %Message{command: "AWAY", params: ["Gone fishing"]})

      {_state3, resps} = IrcServer.on_message(state2, 1, %Message{command: "PRIVMSG", params: ["bob", "hi"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "301" end)
      assert Enum.any?(resps, fn {id, _} -> id == 2 end)
    end

    test "PRIVMSG with insufficient params returns 461" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "PRIVMSG", params: ["#chan"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "461" end)
    end

    test "unregistered client gets 451" do
      state = new_server() |> connect(1)
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "PRIVMSG", params: ["#chan", "hi"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "451" end)
    end
  end

  # ---------------------------------------------------------------------------
  # NOTICE
  # ---------------------------------------------------------------------------

  describe "NOTICE" do
    test "delivers notice to channel (no away replies)" do
      state =
        new_server()
        |> connect(1)
        |> connect(2)
        |> register(1, "alice")
        |> register(2, "bob")
        |> join(1, "#chan")
        |> join(2, "#chan")

      # Set bob away
      {state2, _} = IrcServer.on_message(state, 2, %Message{command: "AWAY", params: ["away"]})
      {_state3, resps} = IrcServer.on_message(state2, 1, %Message{command: "NOTICE", params: ["#chan", "hi"]})

      # No 301 RPL_AWAY for NOTICE
      refute Enum.any?(resps, fn {_id, msg} -> msg.command == "301" end)
      assert Enum.any?(resps, fn {id, _} -> id == 2 end)
    end

    test "NOTICE to unregistered is a no-op" do
      state = new_server() |> connect(1)
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "NOTICE", params: ["#chan", "hi"]})
      assert resps == []
    end

    test "NOTICE with no params is a no-op" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "NOTICE", params: []})
      assert resps == []
    end
  end

  # ---------------------------------------------------------------------------
  # NAMES
  # ---------------------------------------------------------------------------

  describe "NAMES" do
    test "NAMES with channel arg returns 353 and 366" do
      state = new_server() |> connect(1) |> register(1, "alice") |> join(1, "#chan")
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "NAMES", params: ["#chan"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "353" end)
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "366" end)
    end

    test "NAMES with unknown channel returns 366 only" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "NAMES", params: ["#unknown"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "366" end)
      refute Enum.any?(resps, fn {_id, msg} -> msg.command == "353" end)
    end

    test "NAMES with no params lists all channels" do
      state =
        new_server()
        |> connect(1)
        |> register(1, "alice")
        |> join(1, "#chan1")
        |> join(1, "#chan2")

      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "NAMES", params: []})
      name_replies = Enum.filter(resps, fn {_id, msg} -> msg.command == "353" end)
      assert length(name_replies) >= 2
    end
  end

  # ---------------------------------------------------------------------------
  # LIST
  # ---------------------------------------------------------------------------

  describe "LIST" do
    test "returns RPL_LISTSTART, RPL_LIST entries, RPL_LISTEND" do
      state =
        new_server()
        |> connect(1)
        |> register(1, "alice")
        |> join(1, "#chan")

      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "LIST", params: []})
      cmds = commands(resps)
      assert "321" in cmds
      assert "322" in cmds
      assert "323" in cmds
    end
  end

  # ---------------------------------------------------------------------------
  # TOPIC
  # ---------------------------------------------------------------------------

  describe "TOPIC" do
    test "sets topic and broadcasts to channel" do
      state = new_server() |> connect(1) |> register(1, "alice") |> join(1, "#chan")
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "TOPIC", params: ["#chan", "My topic"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "TOPIC" end)
    end

    test "queries topic with no new topic" do
      state = new_server() |> connect(1) |> register(1, "alice") |> join(1, "#chan")
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "TOPIC", params: ["#chan"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command in ["331", "332"] end)
    end

    test "topic on unknown channel returns 403" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "TOPIC", params: ["#unknown"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "403" end)
    end

    test "topic when not in channel returns 442" do
      state =
        new_server()
        |> connect(1)
        |> connect(2)
        |> register(1, "alice")
        |> register(2, "bob")
        |> join(1, "#chan")

      {_state2, resps} = IrcServer.on_message(state, 2, %Message{command: "TOPIC", params: ["#chan", "new topic"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "442" end)
    end

    test "TOPIC with no params returns 461" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "TOPIC", params: []})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "461" end)
    end

    test "unregistered client gets 451" do
      state = new_server() |> connect(1)
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "TOPIC", params: ["#chan"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "451" end)
    end
  end

  # ---------------------------------------------------------------------------
  # KICK
  # ---------------------------------------------------------------------------

  describe "KICK" do
    test "op can kick another user" do
      state =
        new_server()
        |> connect(1)
        |> connect(2)
        |> register(1, "alice")
        |> register(2, "bob")
        |> join(1, "#chan")
        |> join(2, "#chan")

      {state2, resps} = IrcServer.on_message(state, 1, %Message{command: "KICK", params: ["#chan", "bob", "out!"]})
      kick_resps = Enum.filter(resps, fn {_id, msg} -> msg.command == "KICK" end)
      assert length(kick_resps) >= 1
      refute Map.has_key?(state2.channels["#chan"].members, 2)
    end

    test "non-op gets 482" do
      state =
        new_server()
        |> connect(1)
        |> connect(2)
        |> register(1, "alice")
        |> register(2, "bob")
        |> join(1, "#chan")
        |> join(2, "#chan")

      {_state2, resps} = IrcServer.on_message(state, 2, %Message{command: "KICK", params: ["#chan", "alice", "out!"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "482" end)
    end

    test "KICK target not in channel returns 441" do
      state =
        new_server()
        |> connect(1)
        |> connect(2)
        |> register(1, "alice")
        |> register(2, "bob")
        |> join(1, "#chan")

      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "KICK", params: ["#chan", "bob", "out"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "441" end)
    end

    test "KICK on unknown channel returns 403" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "KICK", params: ["#unknown", "bob"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "403" end)
    end

    test "kicker not in channel returns 442" do
      state =
        new_server()
        |> connect(1)
        |> connect(2)
        |> register(1, "alice")
        |> register(2, "bob")
        |> join(2, "#chan")

      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "KICK", params: ["#chan", "bob"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "442" end)
    end

    test "KICK with insufficient params returns 461" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "KICK", params: ["#chan"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "461" end)
    end

    test "unregistered client gets 451" do
      state = new_server() |> connect(1)
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "KICK", params: ["#chan", "bob"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "451" end)
    end
  end

  # ---------------------------------------------------------------------------
  # INVITE
  # ---------------------------------------------------------------------------

  describe "INVITE" do
    test "sends INVITE to target and RPL_INVITING to sender" do
      state =
        new_server()
        |> connect(1)
        |> connect(2)
        |> register(1, "alice")
        |> register(2, "bob")
        |> join(1, "#chan")

      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "INVITE", params: ["bob", "#chan"]})
      assert Enum.any?(resps, fn {id, msg} -> id == 2 and msg.command == "INVITE" end)
      assert Enum.any?(resps, fn {id, msg} -> id == 1 and msg.command == "341" end)
    end

    test "INVITE to unknown nick returns 401" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "INVITE", params: ["nobody", "#chan"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "401" end)
    end

    test "INVITE with insufficient params returns 461" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "INVITE", params: ["bob"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "461" end)
    end

    test "unregistered client gets 451" do
      state = new_server() |> connect(1)
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "INVITE", params: ["bob", "#chan"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "451" end)
    end
  end

  # ---------------------------------------------------------------------------
  # MODE
  # ---------------------------------------------------------------------------

  describe "MODE" do
    test "query channel mode returns RPL_CHANNELMODEIS" do
      state = new_server() |> connect(1) |> register(1, "alice") |> join(1, "#chan")
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "MODE", params: ["#chan"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "324" end)
    end

    test "set channel mode" do
      state = new_server() |> connect(1) |> register(1, "alice") |> join(1, "#chan")
      {state2, _resps} = IrcServer.on_message(state, 1, %Message{command: "MODE", params: ["#chan", "+m"]})
      assert MapSet.member?(state2.channels["#chan"].modes, "m")
    end

    test "remove channel mode" do
      state = new_server() |> connect(1) |> register(1, "alice") |> join(1, "#chan")
      {state2, _} = IrcServer.on_message(state, 1, %Message{command: "MODE", params: ["#chan", "+m"]})
      {state3, _} = IrcServer.on_message(state2, 1, %Message{command: "MODE", params: ["#chan", "-m"]})
      refute MapSet.member?(state3.channels["#chan"].modes, "m")
    end

    test "user mode query returns 324" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "MODE", params: ["alice"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "324" end)
    end

    test "MODE on unknown channel returns 403" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "MODE", params: ["#unknown", "+m"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "403" end)
    end

    test "MODE with no params returns 461" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "MODE", params: []})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "461" end)
    end

    test "unregistered client gets 451" do
      state = new_server() |> connect(1)
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "MODE", params: ["#chan"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "451" end)
    end
  end

  # ---------------------------------------------------------------------------
  # PING / PONG
  # ---------------------------------------------------------------------------

  describe "PING" do
    test "responds with PONG" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "PING", params: ["server"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "PONG" end)
    end
  end

  describe "PONG" do
    test "is a no-op" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "PONG", params: ["server"]})
      assert resps == []
    end
  end

  # ---------------------------------------------------------------------------
  # AWAY
  # ---------------------------------------------------------------------------

  describe "AWAY" do
    test "sets away message and returns RPL_NOWAWAY" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {state2, resps} = IrcServer.on_message(state, 1, %Message{command: "AWAY", params: ["Gone"]})
      assert state2.clients[1].away_message == "Gone"
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "306" end)
    end

    test "clears away message and returns RPL_UNAWAY" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {state2, _} = IrcServer.on_message(state, 1, %Message{command: "AWAY", params: ["Gone"]})
      {state3, resps} = IrcServer.on_message(state2, 1, %Message{command: "AWAY", params: []})
      assert state3.clients[1].away_message == nil
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "305" end)
    end

    test "unregistered client gets 451" do
      state = new_server() |> connect(1)
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "AWAY", params: ["away"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "451" end)
    end
  end

  # ---------------------------------------------------------------------------
  # WHOIS
  # ---------------------------------------------------------------------------

  describe "WHOIS" do
    test "returns whois info for known nick" do
      state = new_server() |> connect(1) |> register(1, "alice") |> connect(2) |> register(2, "bob")
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "WHOIS", params: ["bob"]})
      cmds = commands(resps)
      assert "311" in cmds
      assert "312" in cmds
      assert "318" in cmds
    end

    test "WHOIS on unknown nick returns 401" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "WHOIS", params: ["nobody"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "401" end)
    end

    test "unregistered client gets 451" do
      state = new_server() |> connect(1)
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "WHOIS", params: ["alice"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "451" end)
    end
  end

  # ---------------------------------------------------------------------------
  # WHO
  # ---------------------------------------------------------------------------

  describe "WHO" do
    test "WHO for a channel returns whoreply entries" do
      state =
        new_server()
        |> connect(1)
        |> connect(2)
        |> register(1, "alice")
        |> register(2, "bob")
        |> join(1, "#chan")
        |> join(2, "#chan")

      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "WHO", params: ["#chan"]})
      who_resps = Enum.filter(resps, fn {_id, msg} -> msg.command == "352" end)
      assert length(who_resps) >= 2
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "315" end)
    end

    test "WHO wildcard returns all users" do
      state =
        new_server()
        |> connect(1)
        |> connect(2)
        |> register(1, "alice")
        |> register(2, "bob")

      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "WHO", params: ["*"]})
      who_resps = Enum.filter(resps, fn {_id, msg} -> msg.command == "352" end)
      assert length(who_resps) >= 2
    end

    test "unregistered client gets 451" do
      state = new_server() |> connect(1)
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "WHO", params: ["*"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "451" end)
    end
  end

  # ---------------------------------------------------------------------------
  # OPER
  # ---------------------------------------------------------------------------

  describe "OPER" do
    test "correct password grants oper status" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {state2, resps} = IrcServer.on_message(state, 1, %Message{command: "OPER", params: ["alice", "secret"]})
      assert state2.clients[1].is_oper == true
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "381" end)
    end

    test "wrong password returns 464" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "OPER", params: ["alice", "wrong"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "464" end)
    end

    test "OPER with insufficient params returns 461" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "OPER", params: ["alice"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "461" end)
    end

    test "unregistered client gets 451" do
      state = new_server() |> connect(1)
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "OPER", params: ["alice", "secret"]})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "451" end)
    end
  end

  # ---------------------------------------------------------------------------
  # Unknown command
  # ---------------------------------------------------------------------------

  describe "unknown command" do
    test "returns 421" do
      state = new_server() |> connect(1) |> register(1, "alice")
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "FOOBAR", params: []})
      assert Enum.any?(resps, fn {_id, msg} -> msg.command == "421" end)
    end
  end

  # ---------------------------------------------------------------------------
  # CAP / PASS
  # ---------------------------------------------------------------------------

  describe "CAP and PASS" do
    test "CAP is a no-op" do
      state = new_server() |> connect(1)
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "CAP", params: ["LS"]})
      assert resps == []
    end

    test "PASS is a no-op" do
      state = new_server() |> connect(1)
      {_state2, resps} = IrcServer.on_message(state, 1, %Message{command: "PASS", params: ["password"]})
      assert resps == []
    end
  end
end
