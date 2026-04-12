defmodule CodingAdventures.IrcProtoTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.IrcProto
  alias CodingAdventures.IrcProto.Message

  # ---------------------------------------------------------------------------
  # parse/1 — basic cases
  # ---------------------------------------------------------------------------

  describe "parse/1 — basic cases" do
    test "parses command with no prefix and no params" do
      assert {:ok, %Message{prefix: nil, command: "QUIT", params: []}} =
               IrcProto.parse("QUIT")
    end

    test "parses command with one param" do
      assert {:ok, %Message{command: "NICK", params: ["alice"]}} =
               IrcProto.parse("NICK alice")
    end

    test "parses command with multiple params" do
      assert {:ok, %Message{command: "USER", params: ["alice", "0", "*", "Alice Smith"]}} =
               IrcProto.parse("USER alice 0 * :Alice Smith")
    end

    test "normalises command to uppercase" do
      assert {:ok, %Message{command: "NICK"}} = IrcProto.parse("nick alice")
    end

    test "parses mixed-case command to uppercase" do
      assert {:ok, %Message{command: "JOIN"}} = IrcProto.parse("Join #chan")
    end

    test "parses trailing param (with colon)" do
      assert {:ok, msg} = IrcProto.parse("PRIVMSG #chan :Hello, world!")
      assert msg.params == ["#chan", "Hello, world!"]
    end

    test "trailing param with multiple spaces preserved" do
      assert {:ok, msg} = IrcProto.parse("PRIVMSG #chan :one two  three")
      assert msg.params == ["#chan", "one two  three"]
    end

    test "returns error for empty line" do
      assert {:error, "empty line"} = IrcProto.parse("")
    end

    test "returns error for whitespace-only line" do
      assert {:error, "empty line"} = IrcProto.parse("   ")
    end

    test "returns error for line with only prefix" do
      assert {:error, "missing command"} = IrcProto.parse(":irc.local")
    end

    test "strips trailing CRLF before parsing" do
      assert {:ok, %Message{command: "PING"}} = IrcProto.parse("PING :server\r\n")
    end

    test "strips trailing LF before parsing" do
      assert {:ok, %Message{command: "PING"}} = IrcProto.parse("PING :server\n")
    end
  end

  # ---------------------------------------------------------------------------
  # parse/1 — prefix handling
  # ---------------------------------------------------------------------------

  describe "parse/1 — prefix handling" do
    test "parses server name prefix" do
      assert {:ok, %Message{prefix: "irc.local", command: "NOTICE"}} =
               IrcProto.parse(":irc.local NOTICE alice :Server restarting")
    end

    test "parses nick!user@host prefix" do
      assert {:ok, msg} = IrcProto.parse(":alice!alice@127.0.0.1 PRIVMSG #chan :hi")
      assert msg.prefix == "alice!alice@127.0.0.1"
      assert msg.command == "PRIVMSG"
      assert msg.params == ["#chan", "hi"]
    end

    test "prefix is nil when absent" do
      assert {:ok, %Message{prefix: nil}} = IrcProto.parse("PING :server")
    end

    test "numeric reply with prefix" do
      assert {:ok, msg} = IrcProto.parse(":irc.local 001 alice Welcome!")
      assert msg.prefix == "irc.local"
      assert msg.command == "001"
      assert msg.params == ["alice", "Welcome!"]
    end
  end

  # ---------------------------------------------------------------------------
  # parse/1 — parameter edge cases
  # ---------------------------------------------------------------------------

  describe "parse/1 — parameter edge cases" do
    test "command with no parameters" do
      assert {:ok, %Message{params: []}} = IrcProto.parse("QUIT")
    end

    test "colon-prefixed param that is the only param" do
      assert {:ok, msg} = IrcProto.parse("QUIT :Goodbye!")
      assert msg.params == ["Goodbye!"]
    end

    test "15-param limit: collects exactly 15 params" do
      params = Enum.map(1..15, fn i -> "p#{i}" end)
      line = "CMD " <> Enum.join(params, " ")
      assert {:ok, msg} = IrcProto.parse(line)
      assert length(msg.params) == 15
    end

    test "params beyond 15 are silently dropped" do
      params = Enum.map(1..20, fn i -> "p#{i}" end)
      line = "CMD " <> Enum.join(params, " ")
      assert {:ok, msg} = IrcProto.parse(line)
      assert length(msg.params) == 15
    end

    test "empty trailing param (just a colon)" do
      assert {:ok, msg} = IrcProto.parse("QUIT :")
      assert msg.params == [""]
    end
  end

  # ---------------------------------------------------------------------------
  # serialize/1
  # ---------------------------------------------------------------------------

  describe "serialize/1" do
    test "serializes command only" do
      msg = %Message{command: "QUIT"}
      assert IrcProto.serialize(msg) == "QUIT\r\n"
    end

    test "serializes command with one bare param" do
      msg = %Message{command: "NICK", params: ["alice"]}
      assert IrcProto.serialize(msg) == "NICK alice\r\n"
    end

    test "serializes last param with colon when it contains a space" do
      msg = %Message{command: "PRIVMSG", params: ["#chan", "Hello World"]}
      assert IrcProto.serialize(msg) == "PRIVMSG #chan :Hello World\r\n"
    end

    test "serializes last param without colon when it has no space" do
      msg = %Message{command: "NICK", params: ["alice"]}
      assert IrcProto.serialize(msg) == "NICK alice\r\n"
    end

    test "serializes with prefix" do
      msg = %Message{prefix: "irc.local", command: "001", params: ["alice", "Welcome!"]}
      assert IrcProto.serialize(msg) == ":irc.local 001 alice Welcome!\r\n"
    end

    test "serializes numeric reply with prefix and trailing param" do
      msg = %Message{prefix: "irc.local", command: "001", params: ["alice", "Welcome!"]}
      wire = IrcProto.serialize(msg)
      assert wire == ":irc.local 001 alice Welcome!\r\n"
    end

    test "serialize always ends with CRLF" do
      msg = %Message{command: "PING", params: ["server"]}
      assert String.ends_with?(IrcProto.serialize(msg), "\r\n")
    end

    test "serialize then parse round-trips correctly" do
      original = %Message{
        prefix: "irc.local",
        command: "PRIVMSG",
        params: ["#general", "Hello, world!"]
      }

      wire = IrcProto.serialize(original)
      {:ok, parsed} = IrcProto.parse(String.trim_trailing(wire))

      assert parsed.prefix == original.prefix
      assert parsed.command == original.command
      assert parsed.params == original.params
    end

    test "serialize multi-param message" do
      msg = %Message{command: "USER", params: ["alice", "0", "*", "Alice Smith"]}
      assert IrcProto.serialize(msg) == "USER alice 0 * :Alice Smith\r\n"
    end
  end

  # ---------------------------------------------------------------------------
  # Integration: parse then serialize
  # ---------------------------------------------------------------------------

  describe "round-trip" do
    test "NICK round-trip" do
      line = "NICK alice"
      {:ok, msg} = IrcProto.parse(line)
      assert IrcProto.serialize(msg) == "NICK alice\r\n"
    end

    test "PRIVMSG with trailing param round-trip" do
      line = "PRIVMSG #chan :Hello world"
      {:ok, msg} = IrcProto.parse(line)
      assert IrcProto.serialize(msg) == "PRIVMSG #chan :Hello world\r\n"
    end

    test "server message round-trip" do
      line = ":irc.local 001 alice :Welcome to the IRC network"
      {:ok, msg} = IrcProto.parse(line)
      assert IrcProto.serialize(msg) == ":irc.local 001 alice :Welcome to the IRC network\r\n"
    end
  end
end
