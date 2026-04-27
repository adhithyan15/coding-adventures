# irc-proto — IRC Message Parsing and Serialization

## Overview

`irc-proto` is a pure library with no I/O, no threads, and no side effects. It converts between
the raw text lines of the IRC protocol and structured `Message` values, and back again.

Every other package in the IRC system depends on `irc-proto`'s `Message` type, but `irc-proto`
itself depends on nothing. This makes it the ideal starting point for any new language port: get
the parser right first, test it exhaustively, and the rest of the stack has a solid foundation.

---

## Layer Position

```
irc-server   ← consumes Message values, produces Message values to send
     ↑
irc-proto    ← THIS PACKAGE: parse(line) → Message, serialize(msg) → bytes
     ↑
irc-framing  ← feeds complete \r\n-stripped lines upward
```

`irc-proto` knows nothing about connections, sockets, threads, or buffers. It operates on
`str` (incoming) and returns `bytes` (outgoing). The framer handles the byte ↔ str boundary.

---

## Concepts

### The IRC Message Format (RFC 1459)

An IRC message is a single line of text terminated by `\r\n`. The maximum length including the
terminator is 512 bytes. The format is:

```
[:prefix SPACE] COMMAND [SPACE param]* [SPACE COLON trailing] CR LF
```

Breaking this down:

```
:nick!user@host PRIVMSG #general :Hello, world!\r\n
 ───────────────  ───────  ────────  ──────────────
    prefix        command   param      trailing param
```

- **prefix** — Optional. Always starts with `:`. Can be a server name (`irc.example.com`) or a
  full nick mask (`nick!user@host`). Identifies who sent the message.
- **command** — Required. Either a word (`PRIVMSG`, `JOIN`) or a 3-digit numeric (`001`, `433`).
  Commands from clients have no prefix. Commands from servers have a server-name prefix.
- **params** — Zero to 15 space-separated values. The last param may be prefixed with `:` to
  allow spaces within it (the "trailing" param). All params are collected into a flat list.

### Examples

```
# Client sends (no prefix):
NICK alice\r\n
USER alice 0 * :Alice Smith\r\n
JOIN #general\r\n
PRIVMSG #general :Hello everyone!\r\n
PING :server1\r\n
QUIT :Goodbye\r\n

# Server sends (with prefix):
:irc.local 001 alice :Welcome to the IRC network, alice!\r\n
:irc.local 353 alice = #general :alice bob carol\r\n
:alice!alice@127.0.0.1 JOIN #general\r\n
:irc.local PING :irc.local\r\n
```

### Parsing Rules

1. If the line starts with `:`, everything up to the first space is the prefix (strip the `:`).
2. The next token (space-delimited) is the command. Normalize to uppercase.
3. Remaining tokens are params. If a token starts with `:`, it and everything after it (including
   spaces) is the trailing param — the last param in the list.
4. Maximum 15 params. Any beyond are silently dropped.
5. Commands are case-insensitive (`join` == `JOIN`). Numerics are three-digit strings (`"001"`).

---

## Public API

```python
from __future__ import annotations

from dataclasses import dataclass, field


@dataclass
class Message:
    """A single parsed IRC protocol message.

    prefix is None for client-originated messages.
    command is always uppercase (or a 3-digit numeric string).
    params includes the trailing param as the last element (no colon prefix).
    """
    prefix: str | None
    command: str
    params: list[str] = field(default_factory=list)


class ParseError(Exception):
    """Raised when a line cannot be parsed as an IRC message."""


def parse(line: str) -> Message:
    """Parse a single IRC message line into a Message.

    The line should already have the trailing \\r\\n stripped.
    Raises ParseError if the line is empty or malformed.

    Examples:
        >>> parse("NICK alice")
        Message(prefix=None, command='NICK', params=['alice'])

        >>> parse(":irc.local 001 alice :Welcome!")
        Message(prefix='irc.local', command='001', params=['alice', 'Welcome!'])

        >>> parse(":alice!alice@host PRIVMSG #chan :hello world")
        Message(prefix='alice!alice@host', command='PRIVMSG', params=['#chan', 'hello world'])
    """
    ...


def serialize(msg: Message) -> bytes:
    """Serialize a Message back to IRC wire format.

    Returns CRLF-terminated bytes ready to write to a socket.
    Params containing spaces are automatically written as trailing params (with ':' prefix).
    If prefix is set, it is prepended with ':'.

    Examples:
        >>> serialize(Message(None, 'NICK', ['alice']))
        b'NICK alice\\r\\n'

        >>> serialize(Message('irc.local', '001', ['alice', 'Welcome!']))
        b':irc.local 001 alice :Welcome!\\r\\n'
    """
    ...
```

---

## Numeric Reply Reference

Servers send numeric replies to indicate success or error conditions. These are 3-digit strings
used as the `command` field of a `Message`. Important numerics:

| Numeric | Name | Meaning |
|---|---|---|
| `001` | RPL_WELCOME | Successful registration — sent first after NICK+USER |
| `002` | RPL_YOURHOST | Server version info |
| `003` | RPL_CREATED | Server creation date |
| `004` | RPL_MYINFO | Server capabilities summary |
| `251` | RPL_LUSERCLIENT | Number of users/servers |
| `353` | RPL_NAMREPLY | List of nicks in a channel (response to NAMES or JOIN) |
| `366` | RPL_ENDOFNAMES | End of NAMES list |
| `372` | RPL_MOTD | Line of the Message of the Day |
| `375` | RPL_MOTDSTART | Start of MOTD |
| `376` | RPL_ENDOFMOTD | End of MOTD |
| `401` | ERR_NOSUCHNICK | No such nick or channel |
| `403` | ERR_NOSUCHCHANNEL | No such channel |
| `411` | ERR_NORECIPIENT | No recipient given |
| `412` | ERR_NOTEXTTOSEND | No text to send |
| `421` | ERR_UNKNOWNCOMMAND | Unknown command |
| `431` | ERR_NONICKNAMEGIVEN | No nickname given |
| `432` | ERR_ERRONEUSNICKNAME | Invalid characters in nick |
| `433` | ERR_NICKNAMEINUSE | Nick already taken |
| `441` | ERR_USERNOTINCHANNEL | User not in that channel |
| `442` | ERR_NOTONCHANNEL | You are not on that channel |
| `451` | ERR_NOTREGISTERED | Must register first (NICK + USER) |
| `461` | ERR_NEEDMOREPARAMS | Not enough parameters |
| `462` | ERR_ALREADYREGISTRED | Already registered |
| `482` | ERR_CHANOPRIVSNEEDED | Channel operator privileges needed |

---

## Parsing Edge Cases

| Input | Expected behaviour |
|---|---|
| `""` (empty line) | Raise `ParseError` |
| `"PING"` (no params) | `Message(None, 'PING', [])` |
| `":server PING"` | `Message('server', 'PING', [])` |
| `"join #foo"` | `Message(None, 'JOIN', ['#foo'])` — command uppercased |
| `"PRIVMSG #c :hello world"` | trailing includes spaces: `params=['#c', 'hello world']` |
| `"PRIVMSG #c :"` | empty trailing is valid: `params=['#c', '']` |
| `":pre CMD a b c d e f g h i j k l m n o"` | 16 params — drop last, keep 15 |
| Line > 510 bytes (excl. CRLF) | Accept and parse; framer enforces max length upstream |

---

## Test Strategy

Tests live in the package's `tests/` directory. Coverage target: 98%+.

### Parsing tests

- **Happy path**: one test per example in the "Examples" section above
- **No prefix**: `"NICK alice"` → `Message(None, 'NICK', ['alice'])`
- **Server prefix**: `":irc.local 001 alice :Welcome"` → correct prefix, command, params
- **Numeric command**: `":srv 433 * nick :Nick in use"` → command is `"433"` (string)
- **Trailing with spaces**: `"PRIVMSG #c :hello world"` → `params=['#c', 'hello world']`
- **Empty trailing**: `"PRIVMSG #c :"` → `params=['#c', '']`
- **No params**: `"QUIT"` → `Message(None, 'QUIT', [])`
- **Command case normalization**: `"join #foo"` produces `command='JOIN'`
- **15-param cap**: construct a line with 16 params; verify only 15 returned
- **Empty line**: `parse("")` raises `ParseError`
- **Whitespace-only line**: `parse("   ")` raises `ParseError`
- **Full nick mask prefix**: `":alice!alice@127.0.0.1 QUIT :bye"` → correct prefix

### Serialization tests

- **Round-trip**: `parse(line)` then `serialize(msg).decode().rstrip('\r\n')` equals the original
  line for a representative set of well-formed inputs
- **CRLF termination**: every `serialize()` result ends with `b'\r\n'`
- **Prefix emission**: message with prefix serializes as `":prefix COMMAND ...\r\n"`
- **No prefix**: message without prefix serializes without leading `:`
- **Trailing param**: param containing a space is automatically written with `:` prefix
- **Multiple params**: `Message(None, 'MODE', ['#foo', '+o', 'alice'])` → `b'MODE #foo +o alice\r\n'`

---

## Future Extensions

- **IRCv3 tags**: messages can have `@key=value;key2=value2` tag blocks before the prefix.
  The parser should be extended to extract these into a separate `tags: dict[str, str]` field.
- **ISUPPORT (005)**: servers advertise protocol capabilities. A capability parser could be
  added as a separate module within `irc-proto`.
- **Strict mode**: an optional flag to `parse()` that rejects lines exceeding 512 bytes rather
  than accepting them (for use in conformance testing).
