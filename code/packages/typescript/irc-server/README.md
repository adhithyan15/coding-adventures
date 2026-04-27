# @coding-adventures/irc-server

Pure IRC server state machine — processes IRC messages and returns response lists, with zero I/O.

## Architecture

This package is the "brain" of the IRC stack.  It knows nothing about sockets or networking.  The transport layer calls three methods:

```typescript
const server = new IRCServer("irc.local");

// New TCP connection arrived:
const r1 = server.onConnect(1 as ConnId, "192.168.1.10");

// Client sent "NICK alice\r\n" (already parsed by irc-proto):
const r2 = server.onMessage(1 as ConnId, { prefix: null, command: "NICK", params: ["alice"] });

// Client sent "USER alice 0 * :Alice Smith\r\n":
const r3 = server.onMessage(1 as ConnId, { prefix: null, command: "USER", params: ["alice", "0", "*", "Alice Smith"] });
// → r3 contains the 001–376 welcome sequence as [ConnId, Message][] pairs

// TCP connection closed:
const r4 = server.onDisconnect(1 as ConnId);
// → broadcasts QUIT to all channels alice was in
```

## Supported Commands

| Command  | Description |
|----------|-------------|
| NICK     | Set or change nickname |
| USER     | Supply username and realname (registration) |
| CAP      | Capability negotiation (acknowledged, not enforced) |
| QUIT     | Graceful disconnect |
| JOIN     | Join a channel (creates it if new) |
| PART     | Leave a channel |
| PRIVMSG  | Send message to nick or channel |
| NOTICE   | Send notice (no auto-replies) |
| NAMES    | List channel members |
| LIST     | Enumerate all channels |
| TOPIC    | Get or set channel topic |
| KICK     | Remove member from channel (op only) |
| INVITE   | Invite nick to channel |
| MODE     | Get or set channel/user modes |
| PING     | Keepalive (responds with PONG) |
| AWAY     | Set or clear away status |
| WHOIS    | Get user information |
| WHO      | List users |
| OPER     | Gain IRC operator privileges |

## Stack Position

```
irc-server  ← THIS PACKAGE: IRCServer state machine
    ↑
irc-framing ← feeds complete lines to irc-proto → onMessage()
    ↑
irc-net-*   ← raw TCP bytes
```
