# irc-server — IRC State Machine

## Overview

`irc-server` is the IRC state machine. It tracks connected clients, registered nicks, channels,
and channel membership. It receives `Message` values from the network layer (via `irc-proto`)
and returns lists of `(ConnId, Message)` pairs describing what to send to which connection.

This package has **no I/O**. It does not know what sockets are. It does not read or write bytes.
It is a pure state machine that transforms events into responses. The `ircd` program is
responsible for driving it: reading from the network, calling the server methods, and writing the
returned messages back to the appropriate connections.

`irc-server` depends only on `irc-proto` (for the `Message` type). No other dependencies.

---

## Layer Position

```
ircd (program)    ← drives the event loop; calls server methods, sends responses
     ↓
irc-server        ← THIS PACKAGE: IRC state machine
     ↓
irc-proto         ← Message type; parse() and serialize()
```

The `ircd` program sits above `irc-server` in the call stack: it calls `on_connect`,
`on_message`, and `on_disconnect`, then takes the returned messages and hands them to the
network layer to write. `irc-server` never calls out to anything.

---

## Concepts

### Connection Lifecycle

A TCP connection goes through these phases:

```
TCP connect → on_connect() called
    → client must send NICK then USER (in any order)
    → once both received: registration complete → welcome sequence sent
    → normal operation: JOIN, PART, PRIVMSG, etc.
TCP close  → on_disconnect() called → server cleans up client state
```

Clients that send any command other than `NICK`, `USER`, `CAP`, or `QUIT` before completing
registration receive `451 ERR_NOTREGISTERED`.

### Nick Registration Handshake

```
Client → NICK alice
Client → USER alice 0 * :Alice Smith
Server → :irc.local 001 alice :Welcome to the IRC Network, alice!
Server → :irc.local 002 alice :Your host is irc.local, running version 1.0
Server → :irc.local 003 alice :This server was created today
Server → :irc.local 004 alice irc.local 1.0 o o
Server → :irc.local 375 alice :- irc.local Message of the Day -
Server → :irc.local 372 alice :- Welcome to this IRC server.
Server → :irc.local 376 alice :End of /MOTD command.
```

NICK and USER can arrive in either order. Registration completes when both have been received.

### Channel Lifecycle

- A channel is created implicitly when the first user JOINs it.
- The first user to join a channel becomes a channel operator (`@` prefix in NAMES).
- A channel is destroyed when its last member PARTs or QUITs.
- Channel names must start with `#` (or `&` for local channels, though `#` is the standard).

### Operator Model

- The first user to join a channel is granted operator status (`mode +o`).
- Operators can KICK users, set channel MODE, and use INVITE.
- Server operators (OPER command) have global privileges.

---

## State Model

```python
from __future__ import annotations

from dataclasses import dataclass, field
from typing import NewType

ConnId = NewType('ConnId', int)


@dataclass
class Client:
    """Represents a connected client, registered or not."""
    id: ConnId
    nick: str | None = None          # None until NICK received
    username: str | None = None      # None until USER received
    realname: str | None = None      # None until USER received
    hostname: str = "unknown"        # set from peer address at connect time
    registered: bool = False         # True after both NICK and USER received
    channels: set[str] = field(default_factory=set)   # channel names (lowercase)
    away_message: str | None = None  # set by AWAY, cleared by AWAY with no args
    is_oper: bool = False            # set by OPER command


@dataclass
class ChannelMember:
    """A client's membership in a channel, including their mode flags."""
    client: Client
    is_operator: bool = False        # +o
    has_voice: bool = False          # +v


@dataclass
class Channel:
    """A named chat room."""
    name: str                        # always lowercase; includes '#'
    topic: str = ""
    members: dict[ConnId, ChannelMember] = field(default_factory=dict)
    modes: set[str] = field(default_factory=set)   # channel mode flags (e.g. 'n', 't', 'm')
    invite_list: set[str] = field(default_factory=set)  # nicks invited (for +i channels)
    ban_list: list[str] = field(default_factory=list)   # ban masks


@dataclass
class ServerState:
    """The complete mutable state of an IRC server."""
    server_name: str
    version: str
    motd: list[str]
    clients: dict[ConnId, Client] = field(default_factory=dict)
    nicks: dict[str, ConnId] = field(default_factory=dict)     # lowercase nick → ConnId
    channels: dict[str, Channel] = field(default_factory=dict) # lowercase name → Channel
    oper_password: str = ""
```

---

## Public API

```python
from __future__ import annotations

from typing import Protocol
from irc_proto import Message


Response = tuple[ConnId, Message]


class Handler(Protocol):
    """The interface that irc-server exposes to the ircd program.

    Each method returns a list of (ConnId, Message) pairs.
    The ircd program serializes each Message and sends it to the corresponding ConnId.
    """

    def on_connect(self, conn_id: ConnId, host: str) -> list[Response]:
        """Called when a new TCP connection is established.

        host is the peer's IP address string (used to populate the client's hostname).
        Returns an empty list — no messages are sent until registration completes.
        """
        ...

    def on_message(self, conn_id: ConnId, msg: Message) -> list[Response]:
        """Called when a complete IRC message arrives from a client.

        Returns zero or more (ConnId, Message) pairs.
        Some commands (PRIVMSG to a channel) fan out to many ConnIds.
        Some commands (PING) return a single response to the sender.
        Some commands (QUIT) return messages to other clients and none to the sender.
        """
        ...

    def on_disconnect(self, conn_id: ConnId) -> list[Response]:
        """Called when a TCP connection closes (gracefully or not).

        Removes the client from all channels, notifies channel members,
        and cleans up the nick table. Returns QUIT notifications for channel members.
        """
        ...


class IRCServer:
    """Concrete IRC state machine. Implements Handler.

    Thread safety: IRCServer is NOT thread-safe. If the network layer uses multiple
    threads, access to IRCServer must be serialized with a lock in the ircd program.
    """

    def __init__(
        self,
        server_name: str,
        version: str = "1.0",
        motd: list[str] | None = None,
        oper_password: str = "",
    ) -> None: ...

    def on_connect(self, conn_id: ConnId, host: str) -> list[Response]: ...
    def on_message(self, conn_id: ConnId, msg: Message) -> list[Response]: ...
    def on_disconnect(self, conn_id: ConnId) -> list[Response]: ...
```

---

## Command Reference (Full RFC 1459 Set)

### Registration Commands

| Command | Params | Before Registration | Behaviour |
|---|---|---|---|
| `NICK` | `<nickname>` | Allowed | Set/change nickname. Fails with 433 if nick in use, 432 if invalid characters |
| `USER` | `<user> <mode> <unused> :<realname>` | Allowed | Set username and realname. Ignored after registration (462). |
| `PASS` | `<password>` | Allowed | Server password. Must come before NICK/USER. Not implemented in basic server. |
| `CAP` | subcommand | Allowed | Capability negotiation (IRCv3). Basic server replies `CAP * ACK :` or ignores. |
| `QUIT` | `[:<message>]` | Allowed | Disconnect. Broadcasts QUIT to all shared channels. |

### Messaging Commands

| Command | Params | Behaviour |
|---|---|---|
| `PRIVMSG` | `<target> :<text>` | Send to nick or channel. Target must exist (401/403 if not). |
| `NOTICE` | `<target> :<text>` | Like PRIVMSG but never generates an auto-reply. |

### Channel Commands

| Command | Params | Behaviour |
|---|---|---|
| `JOIN` | `<channel>[,<channel>]` | Join one or more channels. Creates channel if new. Sends JOIN to all members. Follows with 353 NAMES and 366 ENDOFNAMES. |
| `PART` | `<channel>[,<channel>] [:<message>]` | Leave channel(s). Sends PART to all members. Destroys channel if empty. |
| `NAMES` | `[<channel>]` | List nicks in a channel (or all channels if no arg). 353 + 366. |
| `LIST` | `[<channel>]` | List channels with member counts and topics. 321 + 322 rows + 323. |
| `TOPIC` | `<channel> [:<topic>]` | Get or set channel topic. Get: 332 (topic) or 331 (no topic). Set: requires operator if +t mode. |
| `INVITE` | `<nick> <channel>` | Invite nick to channel. Requires operator status. Sends INVITE to target. |
| `KICK` | `<channel> <nick> [:<reason>]` | Remove nick from channel. Requires operator status. Sends KICK to all. |
| `MODE` | `<target> [<modestring> [params]]` | Get or set channel/user modes. See mode table below. |

### Server Commands

| Command | Params | Behaviour |
|---|---|---|
| `PING` | `:<server>` | Server or client keepalive. Server replies `PONG :<server>`. |
| `PONG` | `:<server>` | Response to PING. Clients send this; server resets their ping timer. |
| `AWAY` | `[:<message>]` | Set away message (with text) or return from away (no text). 306/305. |
| `WHO` | `[<mask>]` | List users matching mask. 352 rows + 315. |
| `WHOIS` | `<nick>` | Detail about a user. 311, 312, 319, 301 (if away), 318. |
| `OPER` | `<name> <password>` | Gain IRC operator status. 381 on success, 464 on failure. |
| `KILL` | `<nick> :<reason>` | (Oper only) Disconnect a user. |
| `WALLOPS` | `:<message>` | (Oper only) Broadcast to all opers. |

### Channel Modes

| Mode | Symbol | Meaning |
|---|---|---|
| No external messages | `+n` | Only channel members can send PRIVMSG to the channel |
| Topic protection | `+t` | Only operators can change the topic |
| Moderated | `+m` | Only voiced users (+v) and operators can speak |
| Invite only | `+i` | Only invited users can join |
| Key (password) | `+k <key>` | Joining requires the correct key |
| Limit | `+l <count>` | Maximum member count |
| Operator | `+o <nick>` | Grant/revoke operator status |
| Voice | `+v <nick>` | Grant/revoke voice in moderated channels |
| Ban | `+b <mask>` | Ban mask (nick!user@host glob) |

### User Modes

| Mode | Symbol | Meaning |
|---|---|---|
| Invisible | `+i` | User not shown in WHO unless in shared channel |
| Oper | `+o` | IRC operator (set by OPER command) |
| Wallops | `+w` | Receive WALLOPS messages |
| Away | `+a` | User is away (set implicitly by AWAY command) |

---

## Registration Flow (Detail)

```
TCP connect
    server: [internal] creates Client(id=conn_id, registered=False)
    server: [no response]

Client: NICK alice
    server: validates nick (no spaces, starts with letter or _)
    server: checks nick not in use
    server: stores client.nick = "alice"
    server: [no response yet — waiting for USER]

Client: USER alice 0 * :Alice Smith
    server: stores client.username = "alice", client.realname = "Alice Smith"
    server: client.registered = True
    server: registers nick in nicks table
    server: sends welcome sequence:
        :server 001 alice :Welcome to the IRC Network, alice!
        :server 002 alice :Your host is irc.local, running version 1.0
        :server 003 alice :This server was created <date>
        :server 004 alice irc.local 1.0 o o
        :server 251 alice :There are N users on 1 server
        :server 375 alice :- irc.local Message of the Day -
        :server 372 alice :- <motd line> -
        :server 376 alice :End of /MOTD command.
```

---

## Channel JOIN Flow (Detail)

```
Client: JOIN #general
    server: channel name must start with # (or &)
    server: if channel doesn't exist:
        create Channel(name="#general")
        add client as member with is_operator=True
    server: if channel exists:
        check ban list — if matched: 474 ERR_BANNEDFROMCHAN
        check invite-only mode — if +i and not invited: 473 ERR_INVITEONLYCHAN
        check key mode — if +k and wrong key: 475 ERR_BADCHANNELKEY
        check limit mode — if +l and at limit: 471 ERR_CHANNELISFULL
        add client as member
    server: broadcasts JOIN to ALL channel members (including the joiner):
        :alice!alice@host JOIN #general    (sent to all members)
    server: sends topic (if set):
        :server 332 alice #general :Topic text    (if topic exists)
        :server 331 alice #general :No topic set  (if no topic)
    server: sends NAMES list:
        :server 353 alice = #general :@bob alice carol
        :server 366 alice #general :End of /NAMES list.
```

---

## Nick Validation Rules

A valid nickname:
- Length: 1–9 characters (RFC 1459 limit; some servers allow up to 30)
- First character: letter (`A-Z`, `a-z`) or special (`_`, `[`, `]`, `\`, `` ` ``, `^`, `{`, `}`, `|`)
- Remaining characters: any of the above, plus digits (`0-9`) and `-`
- Must not conflict with an existing nick (case-insensitive comparison)

Invalid nick → `432 ERR_ERRONEUSNICKNAME`. Nick in use → `433 ERR_NICKNAMEINUSE`.

---

## Test Strategy

Tests live in `tests/`. Coverage target: 95%+.

### Registration

- NICK then USER → welcome sequence sent; client marked registered
- USER then NICK → same result (order should not matter)
- Duplicate NICK → 433 returned; original nick unchanged
- Invalid nick characters → 432 returned
- Command before registration → 451 ERR_NOTREGISTERED
- QUIT before registration → connection cleaned up, no crash

### Channel Operations

- JOIN creates channel, sender becomes operator
- Second JOIN: sender added as non-operator; all existing members notified
- JOIN sends correct NAMES list with `@` prefix for operators
- PART removes member; remaining members notified
- PART of last member destroys channel
- PRIVMSG to channel: delivered to all members except sender
- PRIVMSG to nick: delivered to target; 401 if no such nick

### NICK Change

- Registered client changes nick: 433 if in use, else nick updated
- NICK change while in channels: all channel members receive `:old!u@h NICK new`
- Nick table updated atomically (old nick freed, new nick registered)

### Error Conditions

- PRIVMSG with no params → 411 ERR_NORECIPIENT
- PRIVMSG with no text → 412 ERR_NOTEXTTOSEND
- PRIVMSG to non-existent channel → 403 ERR_NOSUCHCHANNEL
- JOIN invalid channel name (no `#`) → 403 ERR_NOSUCHCHANNEL
- KICK non-member → 441 ERR_USERNOTINCHANNEL
- KICK without operator → 482 ERR_CHANOPRIVSNEEDED
- MODE without operator → 482 ERR_CHANOPRIVSNEEDED

### Disconnect Cleanup

- Client in 3 channels disconnects → QUIT sent to all unique channel members
- Nick freed in nick table
- Client removed from all channel member lists
- Empty channels destroyed

### AWAY

- AWAY :Gone → client.away_message set; 306 RPL_NOWAWAY
- AWAY (no text) → away cleared; 305 RPL_UNAWAY
- WHOIS on away user → includes 301 RPL_AWAY with message

---

## Future Extensions

- **Server linking**: multiple servers connected as a network (RFC 2813). Requires server-to-server
  protocol and global nick/channel state propagation. Out of scope for v1.
- **IRCv3 capabilities**: SASL authentication, message tags, account tracking. Can be layered
  on top of the existing command dispatch without restructuring the state machine.
- **Persistence**: channels and their topics currently vanish when the server restarts. A future
  extension could serialize `ServerState` to disk on shutdown.
- **Rate limiting**: per-client message rate limiting to prevent flood attacks. The `Client` struct
  would gain a token bucket or sliding window counter.
