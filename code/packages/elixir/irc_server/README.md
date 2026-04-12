# irc_server (Elixir)

Pure functional RFC 1459 IRC server state machine.

## Overview

`irc_server` is the third layer of the IRC stack. It implements the full IRC
server logic as a **pure functional state machine**: given the current server
state and an incoming event, it returns the new state plus a list of outgoing
messages to send. No I/O, no processes, no side effects.

## Usage

```elixir
alias CodingAdventures.IrcServer
alias CodingAdventures.IrcProto.Message

# Create a new server state.
state = IrcServer.new("irc.local", "0.1.0", ["Welcome!"], "oper_secret")

# A client connects (conn_id=1, from 127.0.0.1).
{state, []} = IrcServer.on_connect(state, 1, "127.0.0.1")

# The client sends NICK.
{:ok, nick_msg} = IrcProto.parse("NICK alice")
{state, responses} = IrcServer.on_message(state, 1, nick_msg)

# The client sends USER to complete registration.
{:ok, user_msg} = IrcProto.parse("USER alice 0 * :Alice Smith")
{state, responses} = IrcServer.on_message(state, 1, user_msg)
# responses contains 001 welcome, 002, 003, 004, 375/372/376 MOTD burst

# Send responses to the client.
Enum.each(responses, fn {conn_id, msg} ->
  wire = IrcProto.serialize(msg)
  send_to(conn_id, wire)
end)

# Client disconnects.
{state, quit_responses} = IrcServer.on_disconnect(state, 1)
```

## Supported Commands

| Command  | Description                                       |
|----------|---------------------------------------------------|
| NICK     | Set or change nickname                            |
| USER     | Complete registration with username/realname      |
| PASS     | Pre-registration password (accepted, not checked) |
| CAP      | IRC capability negotiation (ACK/NAK)              |
| QUIT     | Disconnect with optional message                  |
| JOIN     | Join one or more channels                         |
| PART     | Leave a channel                                   |
| PRIVMSG  | Send a message to a nick or channel               |
| NOTICE   | Like PRIVMSG but no auto-reply                    |
| NAMES    | List nicks in a channel                           |
| LIST     | List all channels                                 |
| TOPIC    | Get or set channel topic                          |
| KICK     | Remove a user from a channel (ops only)           |
| INVITE   | Invite a user to a channel                        |
| MODE     | Query or set user/channel modes (basic)           |
| PING     | Keep-alive ping                                   |
| PONG     | Reply to a ping                                   |
| AWAY     | Set or clear away message                         |
| WHOIS    | Query nick information                            |
| WHO      | Query channel membership                          |
| OPER     | Gain IRC operator privileges                      |

Unknown commands return `421 ERR_UNKNOWNCOMMAND`.

## Design

- The server state is an ordinary Elixir map — no structs, no processes.
- `on_message/3` dispatches on `msg.command` and returns `{new_state, [{conn_id, Message}]}`.
- Responses are always `{conn_id, %Message{}}` tuples so the caller can
  route them to the right TCP connection.
- The `smsg/3` helper builds server-originated messages with the server name
  as prefix.

## In the Stack

    ircd (program)
      |
      +-- irc_net_stdlib   <-- TCP event loop
      |
      +-- irc_server       <-- this package (RFC 1459 state machine)
      |
      +-- irc_framing      <-- CRLF framing
      |
      +-- irc_proto        <-- message parsing

## Dependencies

- `irc_proto` — for the `Message` struct and `IrcProto.parse/1` / `IrcProto.serialize/1`.
