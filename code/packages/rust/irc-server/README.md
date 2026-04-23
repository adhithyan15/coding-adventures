# irc-server

RFC 1459 IRC server state machine. Pure Rust, no I/O.

## What it does

`irc-server` is the brain of the IRC stack. It manages clients, channels, and nick mappings, and implements all IRC commands. The transport layer calls three methods; `irc-server` returns lists of messages to send — it never touches sockets.

## Layer position

```
irc-net-stdlib   ← TCP event loop
irc-server       ← THIS CRATE: channels, nicks, command dispatch
irc-framing      ← TCP byte stream → complete IRC lines
irc-proto        ← parse() / serialize()
```

## Usage

```rust
use irc_server::{IRCServer, ConnId};
use irc_proto::parse;

let mut server = IRCServer::new("irc.local", vec!["Welcome!".to_string()], "");

// New connection:
server.on_connect(ConnId(1), "127.0.0.1");

// Client registers:
let msg = parse("NICK alice").unwrap();
server.on_message(ConnId(1), &msg);
let msg = parse("USER alice 0 * :Alice Smith").unwrap();
let responses = server.on_message(ConnId(1), &msg);
// responses: [001 Welcome, 002 YourHost, 003 Created, 004 MyInfo, 251 LuserClient, 375 MotdStart, 372 Motd, 376 EndOfMotd]

// Client disconnects:
let responses = server.on_disconnect(ConnId(1));
// responses: QUIT broadcast to channel peers
```

## Commands implemented

| Command   | Description                                    |
|-----------|------------------------------------------------|
| NICK      | Set or change nickname                         |
| USER      | Supply username + realname (completes registration) |
| CAP       | Capability negotiation (returns empty ACK)     |
| PASS      | Connection password (accepted, ignored)        |
| QUIT      | Graceful disconnect                            |
| JOIN      | Join a channel (creates if new, first = op)   |
| PART      | Leave a channel                                |
| PRIVMSG   | Send message to nick or channel                |
| NOTICE    | Send notice (no auto-reply)                    |
| NAMES     | List channel members                           |
| LIST      | List all channels                              |
| TOPIC     | Get or set channel topic                       |
| KICK      | Remove member from channel (op only)          |
| INVITE    | Invite a nick to a channel                     |
| MODE      | Get or set channel/user modes                  |
| PING      | Keepalive (returns PONG)                       |
| PONG      | Client response to PING (no-op)               |
| AWAY      | Set or clear away status                       |
| WHOIS     | Look up user information                       |
| WHO       | List users in a channel or globally            |
| OPER      | Gain IRC operator privileges                   |

## Running tests

```bash
cargo test -p irc-server
```
