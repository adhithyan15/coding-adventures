"""irc-proto — Pure IRC message parsing and serialization (RFC 1459)

This package is the foundation of the IRC stack in the coding-adventures monorepo.
It knows nothing about sockets, threads, or buffers — it only converts between
the raw text lines of the IRC protocol and structured ``Message`` values.

Every other IRC package depends on ``irc-proto``'s ``Message`` type, but
``irc-proto`` itself depends on nothing. This is intentional: a pure parsing
library is easy to test exhaustively and easy to port to new languages.

The IRC message grammar (from RFC 1459) in informal BNF::

    message    = [ ":" prefix SPACE ] command [ params ] CRLF
    prefix     = servername / ( nick [ "!" user ] [ "@" host ] )
    command    = 1*letter / 3digit
    params     = 0*14( SPACE middle ) [ SPACE ":" trailing ]
               / 14( SPACE middle ) [ SPACE [ ":" ] trailing ]
    middle     = nospcrlfcl *( ":" / nospcrlfcl )
    trailing   = *( ":" / " " / nospcrlfcl )
    SPACE      = 0x20

In practice: a message is at most 512 bytes including the final CRLF, and
carries a prefix, a command, and up to 15 parameters (the last of which may
contain spaces when prefixed by ``:``.
"""

from __future__ import annotations

from dataclasses import dataclass, field

__version__ = "0.1.0"

# ---------------------------------------------------------------------------
# Data model
# ---------------------------------------------------------------------------

@dataclass
class Message:
    """A single parsed IRC protocol message.

    Think of this as a plain envelope with three slots:

    - ``prefix``  — *Who sent it?*  ``None`` for client-originated messages.
      For server messages this is a server name (``"irc.example.com"``).
      For relayed client messages it is a full nick-mask
      (``"alice!alice@127.0.0.1"``).
    - ``command`` — *What kind of message is it?*  Always uppercase, e.g.
      ``"PRIVMSG"``, ``"JOIN"``, or the 3-digit numeric string ``"001"``.
    - ``params``  — *The arguments.* A plain Python list of strings.  The
      "trailing" param (the one that may contain spaces) is already stripped
      of its leading ``:`` and lives as the last element of the list — no
      special treatment needed by callers.

    Examples::

        Message(prefix=None, command='NICK', params=['alice'])
        Message(prefix='irc.local', command='001', params=['alice', 'Welcome!'])
        Message(prefix='alice!alice@host', command='PRIVMSG',
                params=['#general', 'hello world'])
    """

    prefix: str | None
    command: str
    params: list[str] = field(default_factory=list)


class ParseError(Exception):
    """Raised when a raw line cannot be understood as an IRC message.

    Callers should catch this when reading from untrusted sources and either
    skip the offending line or close the connection, depending on policy.
    """


# ---------------------------------------------------------------------------
# Parsing
# ---------------------------------------------------------------------------

# RFC 1459 allows at most 15 parameters in a single message.  A 16th token
# (or any beyond) is silently discarded.  This constant documents that limit.
_MAX_PARAMS = 15


def parse(line: str) -> Message:
    """Parse a single IRC message line into a :class:`Message`.

    The ``line`` argument must already have its trailing ``\\r\\n`` stripped
    (that is the responsibility of the framing layer, which hands us clean
    lines to interpret).

    Raises :class:`ParseError` when:
    - the line is empty, or
    - the line contains only whitespace (nothing to parse), or
    - there is no command token after the (optional) prefix.

    Parsing proceeds in three stages:

    1. **Optional prefix** — if the line starts with ``:``, consume everything
       up to the first space as the prefix (dropping the leading ``:``.
    2. **Command** — the next whitespace-delimited token, normalised to
       uppercase.
    3. **Params** — each remaining space-delimited token is a param.  When a
       token begins with ``:``, that token *and everything that follows it*
       (spaces included) forms the last param, with the ``:`` stripped.

    Examples::

        >>> parse("NICK alice")
        Message(prefix=None, command='NICK', params=['alice'])

        >>> parse(":irc.local 001 alice :Welcome!")
        Message(prefix='irc.local', command='001', params=['alice', 'Welcome!'])

        >>> parse(":alice!alice@host PRIVMSG #chan :hello world")
        Message(prefix='alice!alice@host', command='PRIVMSG',
                params=['#chan', 'hello world'])

        >>> parse("join #foo")
        Message(prefix=None, command='JOIN', params=['#foo'])
    """
    # ── Stage 0: reject empty / whitespace-only input ──────────────────────
    # An empty line carries no information and RFC 1459 does not permit them.
    # A whitespace-only line likewise has no command and cannot be parsed.
    if not line or not line.strip():
        raise ParseError(f"empty or whitespace-only line: {line!r}")

    # We work with a mutable "rest" view of the input, consuming tokens from
    # the left as we identify each field.
    rest = line

    # ── Stage 1: optional prefix ────────────────────────────────────────────
    # The presence of a leading colon is the unambiguous signal that a prefix
    # follows.  The prefix ends at the first space character.
    #
    #   ":irc.local 001 alice :Welcome!\r\n"
    #    ↑                                   ← leading colon triggers prefix parsing
    #       ↑↑↑↑↑↑↑↑                        ← prefix value (colon stripped)
    prefix: str | None = None
    if rest.startswith(":"):
        # Split on the first space only so the prefix can contain no spaces.
        # (It never legitimately does, but being explicit avoids surprises.)
        space_pos = rest.find(" ")
        if space_pos == -1:
            # A line that is *only* a prefix with no command is malformed.
            raise ParseError(f"line has prefix but no command: {line!r}")
        # Strip the leading colon when storing the prefix value.
        prefix = rest[1:space_pos]
        # Advance past the prefix and the separating space.
        rest = rest[space_pos + 1:]

    # ── Stage 2: command ────────────────────────────────────────────────────
    # The command is the first whitespace-delimited token remaining.
    # RFC 1459 says commands are case-insensitive; we normalise to uppercase
    # so the rest of the stack never has to deal with mixed-case commands.
    #
    #   "001 alice :Welcome!"  →  command="001", rest="alice :Welcome!"
    #   "PRIVMSG #c :hi"       →  command="PRIVMSG", rest="#c :hi"
    parts = rest.split(" ", 1)  # at most one split: command + remainder
    command = parts[0].upper()
    if not command:
        raise ParseError(f"could not extract command from line: {line!r}")
    # Anything after the command (may be empty if there are no params).
    rest = parts[1] if len(parts) > 1 else ""

    # ── Stage 3: parameters ─────────────────────────────────────────────────
    # Parameters are collected one token at a time.  When we encounter a token
    # that begins with ``:``, it signals the start of the *trailing* param:
    # everything from that ``:``, through to the end of the line (spaces and
    # all), belongs to this single parameter.  The leading ``:`` is stripped.
    #
    # Example:
    #   "#c :hello world"
    #   → first token: "#c"   (regular param)
    #   → next token starts with ":": trailing = "hello world"
    #
    # We also enforce the RFC 1459 limit of 15 params; extras are discarded.
    params: list[str] = []

    while rest:
        if rest.startswith(":"):
            # Trailing param — absorbs the rest of the line.
            # Strip the leading colon; spaces are preserved as-is.
            params.append(rest[1:])
            break  # nothing can follow the trailing param

        # Split off the next space-delimited token.
        space_pos = rest.find(" ")
        if space_pos == -1:
            # No more spaces: the remainder is a single final token.
            params.append(rest)
            break
        else:
            token = rest[:space_pos]
            params.append(token)
            rest = rest[space_pos + 1:]

        # Enforce the maximum parameter count.  If we have already collected
        # 15 params, stop — any trailing content is silently dropped.
        if len(params) == _MAX_PARAMS:
            break

    return Message(prefix=prefix, command=command, params=params)


# ---------------------------------------------------------------------------
# Serialization
# ---------------------------------------------------------------------------

def serialize(msg: Message) -> bytes:
    """Serialize a :class:`Message` back to IRC wire format.

    Returns CRLF-terminated bytes ready to be written to a socket or compared
    against expected protocol output in tests.

    Serialization rules:

    1. If ``msg.prefix`` is set, the output begins with ``:<prefix> ``.
    2. The command follows, already normalised to uppercase by the caller or
       the ``parse()`` function.
    3. Each param is appended with a leading space.  If any param contains a
       space character, it **must** be the last param and is written with a
       leading ``:`` so the receiver knows it is the trailing param.
    4. The message always ends with ``\\r\\n`` (CRLF), the IRC line terminator.

    Examples::

        >>> serialize(Message(None, 'NICK', ['alice']))
        b'NICK alice\\r\\n'

        >>> serialize(Message('irc.local', '001', ['alice', 'Welcome!']))
        b':irc.local 001 alice :Welcome!\\r\\n'

        >>> serialize(Message(None, 'PRIVMSG', ['#chan', 'hello world']))
        b'PRIVMSG #chan :hello world\\r\\n'
    """
    # We build the message as a list of string fragments, then join and encode.
    # This avoids repeated string concatenation, which is O(n²) in Python.
    parts: list[str] = []

    # ── Prefix ──────────────────────────────────────────────────────────────
    # Prefix, if present, is always wrapped in a leading colon and followed by
    # a single space so the receiver can find the boundary between prefix and
    # command.
    if msg.prefix is not None:
        parts.append(f":{msg.prefix}")

    # ── Command ─────────────────────────────────────────────────────────────
    parts.append(msg.command)

    # ── Parameters ──────────────────────────────────────────────────────────
    # Walk through every param.  For all but the last we emit the value as-is
    # (preceded by a space).  For the last param, we check whether it contains
    # a space; if it does, it must be serialized as a trailing param with a
    # leading colon so the receiver knows to absorb the rest of the line.
    for i, param in enumerate(msg.params):
        is_last = i == len(msg.params) - 1

        if is_last and " " in param:
            # Trailing param: the colon signals "everything from here to CRLF
            # belongs to this single parameter, spaces and all".
            parts.append(f":{param}")
        else:
            parts.append(param)

    # Join with spaces and append the mandatory CRLF line terminator.
    # IRC uses CRLF (0x0D 0x0A), not just LF.
    line = " ".join(parts) + "\r\n"

    # Encode to bytes for direct socket/buffer use.
    # IRC is specified in ASCII, but UTF-8 is widely accepted in practice.
    return line.encode("utf-8")
