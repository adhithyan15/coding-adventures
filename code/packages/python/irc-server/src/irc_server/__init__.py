"""irc-server — IRC server state machine (channels, nicks, command dispatch)

This package is the brain of an IRC server.  It knows nothing about sockets,
threads, or I/O — it is a *pure state machine* that consumes ``Message`` values
(from ``irc_proto``) and produces lists of ``(ConnId, Message)`` pairs that the
transport layer should forward to the appropriate connections.

Architecture overview
---------------------
An IRC server manages three kinds of mutable state:

  1. **Clients** — each TCP connection is represented by a ``Client`` object
     keyed by a ``ConnId`` (an integer the transport layer assigns and owns).
     A client starts in the *unregistered* state and becomes *registered* once
     it has supplied both a NICK and a USER command.  Only registered clients
     may join channels or send messages.

  2. **Channels** — a ``Channel`` groups a set of registered clients.  The
     first client to join a channel automatically becomes its *operator* and
     gains the power to kick members and change the topic.

  3. **Nick index** — a ``dict[str, ConnId]`` mapping lowercase nick names to
     connection IDs enables O(1) uniqueness checks and direct-message delivery.

The public interface (``IRCServer``) has exactly three methods that the
transport layer calls:

  * ``on_connect(conn_id, host)`` — a new TCP connection arrived.
  * ``on_message(conn_id, msg)``  — a parsed message arrived from a client.
  * ``on_disconnect(conn_id)``    — the TCP connection closed (with or without
                                    a prior QUIT command).

Each method returns a *list of responses*: ``(ConnId, Message)`` tuples.  The
transport layer iterates this list and sends each message to the given
connection.  The server itself never touches sockets.

RFC 1459 references
-------------------
Commands:  https://www.rfc-editor.org/rfc/rfc1459#section-4
Numerics:  https://www.rfc-editor.org/rfc/rfc1459#section-6
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from typing import NewType

from irc_proto import Message

__version__ = "0.1.0"

# ---------------------------------------------------------------------------
# Core type aliases
# ---------------------------------------------------------------------------

# ConnId is an opaque integer that the transport layer assigns to each
# TCP connection.  Using a NewType rather than a bare ``int`` lets mypy
# catch accidental mix-ups between connection IDs and other integers.
ConnId = NewType("ConnId", int)

# A Response is one outbound message destined for a specific connection.
# The IRCServer never writes to sockets — it only produces these pairs.
Response = tuple[ConnId, Message]


# ---------------------------------------------------------------------------
# Nick validation
# ---------------------------------------------------------------------------

# RFC 1459 §2.3.1 defines which characters may appear in a nickname.
#
# A valid nick:
#   - is 1–9 characters long
#   - starts with a letter OR one of: [ ] \ ` ^ { | } _
#   - subsequent characters may additionally include digits and the hyphen
#
# We compile the regex once at module load time for performance.
_NICK_RE = re.compile(
    r"^[a-zA-Z\[\]\\`^{|}_]"   # first char: letter or special
    r"[a-zA-Z0-9\[\]\\`^{|}_-]{0,8}$"  # rest: adds digits and hyphen
)


def _valid_nick(nick: str) -> bool:
    """Return True if *nick* conforms to RFC 1459 nickname rules.

    The rules enforce a maximum length of 9 and restrict the character set
    so that nicks can be used safely in IRC protocol messages without quoting.

    Examples::

        >>> _valid_nick("alice")
        True
        >>> _valid_nick("_bot")
        True
        >>> _valid_nick("")      # empty → invalid
        False
        >>> _valid_nick("a" * 10)  # too long → invalid
        False
        >>> _valid_nick("bad nick")  # space → invalid
        False
    """
    return bool(_NICK_RE.match(nick))


# ---------------------------------------------------------------------------
# State model
# ---------------------------------------------------------------------------

@dataclass
class Client:
    """All the server-side state we keep for one TCP connection.

    A freshly-connected client is *unregistered*: ``registered=False``,
    ``nick=None``, ``username=None``, ``realname=None``.  The client
    transitions to *registered* once both ``NICK`` and ``USER`` have been
    successfully processed.  Until that point, only ``NICK``, ``USER``,
    ``CAP``, and ``QUIT`` are accepted; everything else gets ``451
    ERR_NOTREGISTERED``.

    The ``channels`` field tracks lowercase channel names the client has
    joined, giving us O(1) membership tests and enabling cleanup on disconnect
    without iterating every channel on the server.
    """

    # The transport-layer connection identifier.  Immutable once assigned.
    id: ConnId

    # IRC nickname.  None until the client sends NICK.
    nick: str | None = None

    # IRC username (from the USER command's first parameter).
    username: str | None = None

    # Real name / GECOS (from the USER command's trailing parameter).
    realname: str | None = None

    # Hostname of the connecting peer, supplied by the transport layer on
    # connect.  Used in the ``nick!user@host`` mask that we attach to relayed
    # messages so other clients know who originated a message.
    hostname: str = "unknown"

    # True once both NICK and USER have been processed successfully.
    registered: bool = False

    # Lowercase channel names this client has joined.  Kept in sync with the
    # Channel.members dicts so that on_disconnect can efficiently clean up.
    channels: set[str] = field(default_factory=set)

    # Optional away message.  None means the client is not away.
    # Set via the AWAY command; cleared by sending AWAY with no message.
    away_message: str | None = None

    # True if the client has authenticated with OPER.  IRC operators can use
    # privileged commands (in a real server; here we record the flag for WHOIS).
    is_oper: bool = False

    @property
    def mask(self) -> str:
        """Return the ``nick!user@host`` mask used as a message prefix.

        This is the standard IRC identity string.  Other clients see this in
        the prefix of any message we relay on behalf of this client.

        Example:  ``"alice!alice@192.168.1.1"``
        """
        nick = self.nick or "*"
        user = self.username or "*"
        return f"{nick}!{user}@{self.hostname}"


@dataclass
class ChannelMember:
    """Per-membership metadata for a client inside a channel.

    A single client may be in many channels simultaneously; each membership
    is represented by a separate ``ChannelMember`` instance stored in
    ``Channel.members``.

    ``is_operator``  — True if this client is a channel operator (@).
                       The first member of a newly-created channel gets this.
    ``has_voice``    — True if this client has voice privilege (+v).
                       Voice allows speaking in moderated (+m) channels.
    """

    client: Client
    is_operator: bool = False
    has_voice: bool = False


@dataclass
class Channel:
    """All server-side state for one IRC channel.

    Channel names are always stored and compared in lowercase, normalised when
    the client sends ``JOIN``.  Clients see the lowercase name in all responses.

    ``members``   — maps ConnId to ChannelMember.  Using ConnId as the key
                    gives O(1) look-ups by connection without needing the nick.
    ``modes``     — the set of single-character channel mode letters currently
                    active (e.g. ``{'n', 't'}``).  We store mode letters but
                    only partially implement mode *setting* in this v1 scope.
    ``ban_list``  — list of nick/host mask patterns that are banned.
                    Stored but not enforced in this v1 scope.
    """

    # Lowercase channel name including the '#' sigil.
    name: str

    # Human-readable topic string.  Empty string means no topic is set.
    topic: str = ""

    # Active members indexed by ConnId.
    members: dict[ConnId, ChannelMember] = field(default_factory=dict)

    # Active channel mode flags (single characters).
    modes: set[str] = field(default_factory=set)

    # Ban mask list.
    ban_list: list[str] = field(default_factory=list)


# ---------------------------------------------------------------------------
# IRC numeric reply constants
# ---------------------------------------------------------------------------
# Named constants make the handler code self-documenting.  Instead of
# scattering magic string literals like "433" throughout the code, we define
# them here with their RFC 1459 symbolic names so readers can look them up.

RPL_WELCOME = "001"       # :Welcome to the IRC Network, nick!user@host
RPL_YOURHOST = "002"      # :Your host is <name>, running version <ver>
RPL_CREATED = "003"       # :This server was created today
RPL_MYINFO = "004"        # <servername> <version> <usermodes> <chanmodes>
RPL_LUSERCLIENT = "251"   # :There are N users on 1 server
RPL_AWAY = "301"          # <nick> :<away message>
RPL_UNAWAY = "305"        # :You are no longer marked as being away
RPL_NOWAWAY = "306"       # :You have been marked as being away
RPL_WHOISUSER = "311"     # <nick> <user> <host> * :<realname>
RPL_WHOISSERVER = "312"   # <nick> <server> :<server info>
RPL_WHOISCHANNELS = "319" # <nick> :{[@|+]channel...}
RPL_LIST = "322"          # <channel> <# visible> :<topic>
RPL_LISTSTART = "321"     # Channel :Users  Name
RPL_LISTEND = "323"       # :End of /LIST
RPL_CHANNELMODEIS = "324" # <channel> <mode> [<mode params>]
RPL_NOTOPIC = "331"       # <channel> :No topic is set
RPL_TOPIC = "332"         # <channel> :<topic>
RPL_INVITING = "341"      # <channel> <nick>
RPL_WHOREPLY = "352"      # <channel> <user> <host> <server> <nick> H|G :hops realname
RPL_NAMREPLY = "353"      # = <channel> :<prefix>nick...
RPL_ENDOFNAMES = "366"    # <channel> :End of /NAMES list
RPL_ENDOFWHO = "315"      # <name> :End of /WHO list
RPL_MOTDSTART = "375"     # :- <server> Message of the Day -
RPL_MOTD = "372"          # :- <text> -
RPL_ENDOFMOTD = "376"     # :End of /MOTD command
RPL_YOUREOPER = "381"     # :You are now an IRC operator
RPL_ENDOFWHOIS = "318"    # <nick> :End of /WHOIS list

ERR_NOSUCHNICK = "401"    # <nick/channel> :No such nick/channel
ERR_NOSUCHCHANNEL = "403" # <channel> :No such channel
ERR_NOTEXTTOSEND = "412"  # :No text to send
ERR_UNKNOWNCOMMAND = "421" # <command> :Unknown command
ERR_NONICKNAMEGIVEN = "431" # :No nickname given
ERR_ERRONEUSNICKNAME = "432" # <nick> :Erroneous nickname
ERR_NICKNAMEINUSE = "433"  # <nick> :Nickname is already in use
ERR_USERNOTINCHANNEL = "441" # <nick> <channel> :They aren't on that channel
ERR_NOTONCHANNEL = "442"   # <channel> :You're not on that channel
ERR_NEEDMOREPARAMS = "461" # <command> :Not enough parameters
ERR_PASSWDMISMATCH = "464" # :Password incorrect
ERR_CHANOPRIVSNEEDED = "482" # <channel> :You're not channel operator
ERR_NOTREGISTERED = "451"  # :You have not registered


# ---------------------------------------------------------------------------
# IRCServer
# ---------------------------------------------------------------------------

class IRCServer:
    """Pure IRC server state machine.

    This class contains the complete server state (clients, channels, nick
    index) and the logic for every IRC command.  It never touches the network
    — the transport layer calls ``on_connect``, ``on_message``, and
    ``on_disconnect``, and the server returns lists of ``(ConnId, Message)``
    pairs that the transport should deliver.

    Concurrency note: this class is intentionally NOT thread-safe.  If the
    transport layer is async or multi-threaded, it must serialize calls to
    these three methods (e.g., with a lock or by running on a single event-
    loop thread).

    Usage example::

        server = IRCServer("irc.example.com")

        # New connection arrives:
        responses = server.on_connect(ConnId(1), "192.168.1.10")

        # Client sends "NICK alice\\r\\n":
        msg = irc_proto.parse("NICK alice")
        responses = server.on_message(ConnId(1), msg)

        # Client sends "USER alice 0 * :Alice Smith\\r\\n":
        msg = irc_proto.parse("USER alice 0 * :Alice Smith")
        responses = server.on_message(ConnId(1), msg)
        # → responses now contains the 001–376 welcome sequence

        # Later, the TCP connection drops:
        responses = server.on_disconnect(ConnId(1))
        # → QUIT is broadcast to all channels alice was in
    """

    def __init__(
        self,
        server_name: str,
        version: str = "1.0",
        motd: list[str] | None = None,
        oper_password: str = "",
    ) -> None:
        """Initialise the server with static configuration.

        Parameters
        ----------
        server_name:
            The hostname this server advertises (e.g. ``"irc.example.com"``).
            Appears in the prefix of all server-generated messages and in the
            001/002 welcome numerics.
        version:
            Software version string, shown in 002 and 004 numerics.
        motd:
            Lines of the Message of the Day.  ``None`` or an empty list both
            result in an empty MOTD section in the welcome sequence.
        oper_password:
            Plaintext password for the ``OPER`` command.  An empty string
            disables oper promotion (the password will never match).
        """
        self._server_name = server_name
        self._version = version
        self._motd: list[str] = motd if motd is not None else []
        self._oper_password = oper_password

        # All known clients keyed by ConnId.
        self._clients: dict[ConnId, Client] = {}

        # All active channels keyed by lowercase name (including '#').
        self._channels: dict[str, Channel] = {}

        # Nick → ConnId index.  All nicks stored in lowercase for case-
        # insensitive uniqueness checks.
        self._nicks: dict[str, ConnId] = {}

    # -----------------------------------------------------------------------
    # Public API
    # -----------------------------------------------------------------------

    def on_connect(self, conn_id: ConnId, host: str) -> list[Response]:
        """Register a new TCP connection.

        Creates a ``Client`` record for the connection but does not send
        anything — IRC clients are expected to initiate registration by sending
        ``CAP``, ``NICK``, and ``USER``.

        Returns an empty list; no messages are sent until the client speaks.
        """
        self._clients[conn_id] = Client(id=conn_id, hostname=host)
        return []

    def on_message(self, conn_id: ConnId, msg: Message) -> list[Response]:
        """Dispatch an inbound IRC message and return the resulting responses.

        This is the central dispatch method.  It:

        1. Looks up the client record for *conn_id*.
        2. Routes the message to the appropriate ``_handle_*`` method based
           on ``msg.command``.
        3. Returns the list of ``(ConnId, Message)`` pairs to send.

        If the client is unknown (which should not happen in normal usage, but
        might if the transport layer calls this after a disconnect) we return
        an empty list rather than crashing.

        Pre-registration gate
        ~~~~~~~~~~~~~~~~~~~~~
        A client that has not yet completed the NICK+USER handshake may only
        send: ``NICK``, ``USER``, ``CAP``, ``QUIT``, and ``PASS`` (not
        implemented here).  Any other command gets ``451 ERR_NOTREGISTERED``.
        This mirrors the behaviour of real IRC servers like InspIRCd and
        ircd-seven.
        """
        client = self._clients.get(conn_id)
        if client is None:
            # Unknown connection — should not happen in normal operation.
            return []

        # Commands permitted before registration is complete.
        pre_reg_allowed = {"NICK", "USER", "CAP", "QUIT", "PASS"}

        # Map command strings to their handler methods.  We look up the
        # command in this table rather than using getattr() so that mypy can
        # verify the handler signatures are correct.
        #
        # Every handler receives (client, msg) and returns list[Response].
        handlers = {
            "NICK":    self._handle_nick,
            "USER":    self._handle_user,
            "CAP":     self._handle_cap,
            "QUIT":    self._handle_quit,
            "PASS":    self._handle_pass,
            "JOIN":    self._handle_join,
            "PART":    self._handle_part,
            "PRIVMSG": self._handle_privmsg,
            "NOTICE":  self._handle_notice,
            "NAMES":   self._handle_names,
            "LIST":    self._handle_list,
            "TOPIC":   self._handle_topic,
            "KICK":    self._handle_kick,
            "INVITE":  self._handle_invite,
            "MODE":    self._handle_mode,
            "PING":    self._handle_ping,
            "PONG":    self._handle_pong,
            "AWAY":    self._handle_away,
            "WHOIS":   self._handle_whois,
            "WHO":     self._handle_who,
            "OPER":    self._handle_oper,
        }

        # Gate: reject post-registration commands from unregistered clients.
        if not client.registered and msg.command not in pre_reg_allowed:
            return [
                (
                    conn_id,
                    self._make_msg(ERR_NOTREGISTERED, "*", ":You have not registered"),
                )
            ]

        handler = handlers.get(msg.command)
        if handler is None:
            # We do not know this command.
            return [
                (
                    conn_id,
                    self._make_msg(
                        ERR_UNKNOWNCOMMAND,
                        client.nick or "*",
                        msg.command,
                        ":Unknown command",
                    ),
                )
            ]

        return handler(client, msg)

    def on_disconnect(self, conn_id: ConnId) -> list[Response]:
        """Clean up state after a TCP connection closes.

        This is called by the transport layer when it detects that a connection
        has been closed, either cleanly (after a QUIT) or unexpectedly (e.g. a
        network error).

        The cleanup procedure:
        1. If the client has a nick, broadcast a QUIT message to all channel
           members who share a channel with the disconnecting client.
        2. Remove the client from every channel they were in.  Destroy any
           channel that becomes empty.
        3. Remove the client's nick from the nick index.
        4. Remove the client record entirely.
        """
        client = self._clients.get(conn_id)
        if client is None:
            return []  # Already cleaned up (e.g., called twice).

        responses: list[Response] = []

        # Only registered clients (with a nick) get a quit broadcast.
        if client.registered and client.nick:
            quit_msg = Message(
                prefix=client.mask,
                command="QUIT",
                params=["Connection closed"],
            )
            # Send to all unique channel members (excluding the quitting client).
            for conn in self._unique_channel_peers(client):
                responses.append((conn, quit_msg))

        # Remove the client from every channel they were in.
        for chan_name in list(client.channels):
            channel = self._channels.get(chan_name)
            if channel:
                channel.members.pop(conn_id, None)
                # Destroy the channel if it is now empty.
                if not channel.members:
                    del self._channels[chan_name]

        # Remove from nick index.
        if client.nick:
            self._nicks.pop(client.nick.lower(), None)

        # Remove the client record.
        del self._clients[conn_id]

        return responses

    # -----------------------------------------------------------------------
    # Private helpers
    # -----------------------------------------------------------------------

    def _make_msg(self, command: str, *params: str) -> Message:
        """Build a Message whose prefix is the server name.

        This is a convenience wrapper so every handler can write::

            self._make_msg("001", nick, ":Welcome!")

        instead of the more verbose::

            Message(prefix=self._server_name, command="001", params=[nick, "Welcome!"])

        Parameters that start with ``:`` are kept as-is (the serializer will
        handle them correctly); parameters that don't are also kept as-is.
        The colon in the ``params`` list here is purely for readability in
        calling code — the actual ``Message.params`` list does NOT include
        the colon; that's added by the serializer only for trailing params
        that contain spaces.

        Wait — actually, let's think about this carefully.  The irc_proto
        ``Message.params`` list stores *decoded* values (no leading colons).
        The colon is a wire-format artefact that ``irc_proto.serialize`` adds
        automatically when needed.  So we must strip any leading ``:`` from
        params we pass here.
        """
        # Strip the leading colon that callers often include for readability.
        # The serialize() function re-adds it if the param contains spaces.
        cleaned: list[str] = [p.lstrip(":") if p.startswith(":") else p for p in params]
        return Message(prefix=self._server_name, command=command, params=list(cleaned))

    def _client_msg(self, client: Client, command: str, *params: str) -> Message:
        """Build a server message addressed to *client*'s nick.

        Like ``_make_msg`` but automatically inserts the client's nick (or
        ``*`` if not yet set) as the first parameter.  Most IRC numerics have
        the form::

            :server <numeric> <target_nick> <rest...>

        so this helper eliminates the repeated ``client.nick or "*"`` boilerplate.
        """
        nick = client.nick or "*"
        return self._make_msg(command, nick, *params)

    def _welcome(self, client: Client) -> list[Response]:
        """Send the RFC 1459 welcome sequence to a newly-registered client.

        This sequence is sent exactly once, immediately after both NICK and
        USER have been received.  It consists of numerics 001–004 (the core
        welcome), 251 (LUSERCLIENT), and the MOTD block (375/372.../376).

        Numeric breakdown:
          001 — Personalised welcome message.
          002 — Which server the client is connected to and its version.
          003 — When the server was "created" (we always say "today").
          004 — Machine-readable server capabilities summary.
          251 — How many users are currently on the network.
          375 — MOTD header line.
          372 — One per MOTD line (may be zero lines).
          376 — MOTD footer line.
        """
        nick = client.nick or "*"
        host = self._server_name
        ver = self._version

        # Count total registered users for the 251 numeric.
        user_count = sum(1 for c in self._clients.values() if c.registered)

        responses: list[Response] = [
            (client.id, self._client_msg(
                client, RPL_WELCOME,
                f"Welcome to the IRC Network, {client.mask}",
            )),
            (client.id, self._client_msg(
                client, RPL_YOURHOST,
                f"Your host is {host}, running version {ver}",
            )),
            (client.id, self._client_msg(
                client, RPL_CREATED,
                "This server was created today",
            )),
            (client.id, self._make_msg(
                RPL_MYINFO, nick, host, ver, "o", "o",
            )),
            (client.id, self._client_msg(
                client, RPL_LUSERCLIENT,
                f"There are {user_count} users on 1 server",
            )),
            # MOTD header
            (client.id, self._client_msg(
                client, RPL_MOTDSTART,
                f"- {host} Message of the Day -",
            )),
        ]

        # One 372 line per MOTD line (may be zero).
        for line in self._motd:
            responses.append((
                client.id,
                self._client_msg(client, RPL_MOTD, f"- {line} -"),
            ))

        # MOTD footer.
        responses.append((
            client.id,
            self._client_msg(client, RPL_ENDOFMOTD, "End of /MOTD command."),
        ))

        return responses

    def _names(self, channel: Channel, requesting_nick: str) -> list[Response]:
        """Build 353 (NAMREPLY) + 366 (ENDOFNAMES) responses for *channel*.

        The 353 line lists all visible members of the channel.  Each member's
        nick is prefixed with ``@`` if they are a channel operator or ``+`` if
        they have voice (but are not an operator).  Regular members have no
        prefix.

        The requesting client gets both the 353 and the 366 terminator.

        We need to look up the requesting client to find their ConnId for the
        response target.  If the nick is not found, we return an empty list
        (this should not happen in normal operation).

        Example 353 payload::
            = #general :@alice bob +carol

        The ``=`` token indicates a public channel (as opposed to ``@``
        for secret or ``*`` for private).
        """
        conn_id = self._nicks.get(requesting_nick.lower())
        if conn_id is None:
            return []

        # Build the space-separated list of prefixed nicks.
        names_parts: list[str] = []
        for member in channel.members.values():
            if member.is_operator:
                names_parts.append(f"@{member.client.nick}")
            elif member.has_voice:
                names_parts.append(f"+{member.client.nick}")
            else:
                names_parts.append(member.client.nick or "")

        names_str = " ".join(names_parts)

        return [
            (
                conn_id,
                self._make_msg(
                    RPL_NAMREPLY,
                    requesting_nick,
                    "=",
                    channel.name,
                    names_str,
                ),
            ),
            (
                conn_id,
                self._make_msg(
                    RPL_ENDOFNAMES,
                    requesting_nick,
                    channel.name,
                    "End of /NAMES list",
                ),
            ),
        ]

    def _unique_channel_peers(self, client: Client) -> set[ConnId]:
        """Return the set of ConnIds that share at least one channel with *client*.

        This is used when broadcasting a QUIT or NICK-change: we need to reach
        every other client who can "see" this client (i.e. is in at least one
        common channel) exactly once, even if they share multiple channels.

        The *client* itself is excluded from the returned set.
        """
        peers: set[ConnId] = set()
        for chan_name in client.channels:
            channel = self._channels.get(chan_name)
            if channel:
                for conn_id in channel.members:
                    if conn_id != client.id:
                        peers.add(conn_id)
        return peers

    # -----------------------------------------------------------------------
    # Command handlers
    # -----------------------------------------------------------------------
    # Each handler receives the Client object for the sender and the parsed
    # Message, and returns a list[Response].  Handlers may mutate server state
    # freely; they are only called from on_message (which is already holding
    # the logical "lock" in the sense that calls are serialized).

    def _handle_cap(self, client: Client, msg: Message) -> list[Response]:
        """Handle the CAP (Capability Negotiation) command.

        Modern IRC clients send ``CAP LS`` at the start of a connection to
        discover server capabilities before sending NICK/USER.  We do not
        implement capability negotiation, so we acknowledge all CAP requests
        with an empty ACK and move on.  This prevents clients that require a
        response from hanging.

        A real server would enumerate its supported capabilities here
        (e.g., ``multi-prefix``, ``sasl``, ``away-notify``).  For this v1
        implementation we keep it simple: advertise nothing, accept everything.
        """
        # Reply with an empty CAP ACK to satisfy clients that wait for it.
        # The format is: CAP * ACK :<capabilities>
        # An empty capabilities list means "no capabilities negotiated".
        return [
            (
                client.id,
                Message(
                    prefix=self._server_name,
                    command="CAP",
                    params=["*", "ACK", ""],
                ),
            )
        ]

    def _handle_pass(self, client: Client, msg: Message) -> list[Response]:
        """Handle the PASS command (connection password).

        PASS is sent before NICK/USER on servers that require a connection
        password.  We do not enforce connection passwords in this v1, so we
        accept and silently ignore PASS.
        """
        # Silently accept; no response needed.
        return []

    def _handle_nick(self, client: Client, msg: Message) -> list[Response]:
        """Handle the NICK command — set or change a client's nickname.

        Pre-registration:
          The client is trying to set their nick for the first time.  We
          validate it, check uniqueness, store it, and — if USER has already
          been received — trigger the welcome sequence.

        Post-registration (nick change):
          Broadcast ``:old!user@host NICK new`` to all clients who share a
          channel with the nick-changer, then update the nick index.

        Error cases:
          431  — No nick given (empty params).
          432  — Nick fails the RFC 1459 character/length validation.
          433  — Nick is already in use by another client.
        """
        # ── Validate params ──────────────────────────────────────────────────
        if not msg.params:
            return [
                (
                    client.id,
                    self._client_msg(
                        client, ERR_NONICKNAMEGIVEN, "No nickname given"
                    ),
                )
            ]

        new_nick = msg.params[0]

        # ── Validate nick format ─────────────────────────────────────────────
        if not _valid_nick(new_nick):
            return [
                (
                    client.id,
                    self._client_msg(
                        client, ERR_ERRONEUSNICKNAME,
                        new_nick, "Erroneous nickname",
                    ),
                )
            ]

        # ── Check uniqueness (case-insensitive) ──────────────────────────────
        existing = self._nicks.get(new_nick.lower())
        if existing is not None and existing != client.id:
            # The nick is taken by a *different* client.
            return [
                (
                    client.id,
                    self._client_msg(
                        client, ERR_NICKNAMEINUSE,
                        new_nick, "Nickname is already in use",
                    ),
                )
            ]

        # ── Apply the nick change ────────────────────────────────────────────
        old_nick = client.nick
        old_mask = client.mask  # capture before we mutate

        # Remove old nick from the index (if any).
        if old_nick:
            self._nicks.pop(old_nick.lower(), None)

        # Register the new nick.
        client.nick = new_nick
        self._nicks[new_nick.lower()] = client.id

        # ── Post-registration: broadcast NICK change to peers ────────────────
        if client.registered and old_nick is not None:
            # All clients sharing a channel with this client (including the
            # changer themselves) need to see the NICK message.
            nick_change_msg = Message(
                prefix=old_mask,
                command="NICK",
                params=[new_nick],
            )
            responses: list[Response] = []
            # Notify the changer themselves.
            responses.append((client.id, nick_change_msg))
            # Notify all channel peers (unique, excluding client already added).
            for peer_id in self._unique_channel_peers(client):
                responses.append((peer_id, nick_change_msg))
            return responses

        # ── Pre-registration: check if USER already done → welcome ───────────
        if client.username is not None:
            client.registered = True
            return self._welcome(client)

        # NICK stored; waiting for USER — send nothing yet.
        return []

    def _handle_user(self, client: Client, msg: Message) -> list[Response]:
        """Handle the USER command — supply username and real name.

        Syntax: ``USER <username> <mode> <unused> :<realname>``

        The second and third parameters (mode flags and "unused") are accepted
        and discarded — most servers ignore them on initial registration.

        Error cases:
          461  — Not enough parameters (need at least 4).

        After successfully storing username/realname, if NICK has already been
        received, we trigger the welcome sequence and mark the client as
        registered.

        Note: USER may only be sent once.  Subsequent USER commands from
        already-registered clients are silently ignored (some servers send 462
        ERR_ALREADYREGISTERED; we keep it simple).
        """
        if client.registered:
            # Already registered — ignore duplicate USER.
            return []

        # We need at least 4 params: username, mode, unused, realname.
        if len(msg.params) < 4:
            return [
                (
                    client.id,
                    self._client_msg(
                        client, ERR_NEEDMOREPARAMS,
                        "USER", "Not enough parameters",
                    ),
                )
            ]

        client.username = msg.params[0]
        # params[1] is mode flags (e.g., "0" or "8"), params[2] is "*" — both ignored.
        client.realname = msg.params[3]

        # If NICK has already been received, complete the registration handshake.
        if client.nick is not None:
            client.registered = True
            return self._welcome(client)

        # USER stored; waiting for NICK — send nothing yet.
        return []

    def _handle_quit(self, client: Client, msg: Message) -> list[Response]:
        """Handle the QUIT command — graceful client disconnect.

        The client is telling us they are leaving.  We:
        1. Broadcast a QUIT message to all channel peers.
        2. Send an ERROR message to the quitting client (RFC 1459 §4.1.6).
        3. Call ``on_disconnect`` to clean up state.

        The optional trailing parameter is the quit message text.  If omitted
        we use a default message.
        """
        # Extract the optional quit message.
        quit_reason = msg.params[0] if msg.params else "Quit"

        responses: list[Response] = []

        # Broadcast QUIT to channel peers.
        if client.registered and client.nick:
            quit_broadcast = Message(
                prefix=client.mask,
                command="QUIT",
                params=[quit_reason],
            )
            for peer_id in self._unique_channel_peers(client):
                responses.append((peer_id, quit_broadcast))

        # Send ERROR to the quitting client as a farewell.
        responses.append((
            client.id,
            Message(
                prefix=None,
                command="ERROR",
                params=[f"Closing Link: {client.hostname} (Quit: {quit_reason})"],
            ),
        ))

        # Now clean up state (this removes the client from all data structures).
        # We do this manually rather than calling on_disconnect so we don't
        # double-broadcast — on_disconnect also broadcasts QUIT.
        for chan_name in list(client.channels):
            channel = self._channels.get(chan_name)
            if channel:
                channel.members.pop(client.id, None)
                if not channel.members:
                    del self._channels[chan_name]

        if client.nick:
            self._nicks.pop(client.nick.lower(), None)

        self._clients.pop(client.id, None)

        return responses

    def _handle_join(self, client: Client, msg: Message) -> list[Response]:
        """Handle the JOIN command — add a client to a channel.

        Syntax: ``JOIN <#channel>[,<#channel2>...]``

        IRC allows joining multiple channels in one command (comma-separated),
        but for v1 we handle only the first channel listed.  Clients rarely
        send multi-channel joins.

        When joining a channel:
        - If the channel does not exist, create it.  The first member becomes
          the channel operator automatically (indicated by ``@`` in NAMES).
        - If the channel already exists, add the client as a regular member.
        - Broadcast ``:nick!user@host JOIN #channel`` to ALL members of the
          channel (including the joiner — clients use this to confirm they
          joined).
        - Send NAMES (353 + 366) to the joiner so they know who is in the room.

        Error cases:
          461  — No channel specified.
        """
        if not msg.params:
            return [
                (
                    client.id,
                    self._client_msg(
                        client, ERR_NEEDMOREPARAMS, "JOIN", "Not enough parameters"
                    ),
                )
            ]

        # Handle comma-separated list; take all of them.
        chan_names = msg.params[0].split(",")
        responses: list[Response] = []

        for chan_name_raw in chan_names:
            chan_name = chan_name_raw.lower()

            # Skip if already in channel.
            if chan_name in client.channels:
                continue

            # Create the channel if it does not exist yet.
            is_new_channel = chan_name not in self._channels
            if is_new_channel:
                self._channels[chan_name] = Channel(name=chan_name)

            channel = self._channels[chan_name]

            # Add the client to the channel.  The first member is the operator.
            is_first_member = len(channel.members) == 0
            channel.members[client.id] = ChannelMember(
                client=client,
                is_operator=is_first_member,
            )
            client.channels.add(chan_name)

            # Broadcast JOIN to all current members (including the joiner).
            join_msg = Message(
                prefix=client.mask,
                command="JOIN",
                params=[chan_name],
            )
            for member_conn_id in channel.members:
                responses.append((member_conn_id, join_msg))

            # Send the current topic if one is set.
            nick = client.nick or "*"
            if channel.topic:
                responses.append((
                    client.id,
                    self._make_msg(RPL_TOPIC, nick, chan_name, channel.topic),
                ))
            else:
                responses.append((
                    client.id,
                    self._make_msg(RPL_NOTOPIC, nick, chan_name, "No topic is set"),
                ))

            # Send NAMES (353 + 366) to the joiner.
            responses.extend(self._names(channel, nick))

        return responses

    def _handle_part(self, client: Client, msg: Message) -> list[Response]:
        """Handle the PART command — remove a client from a channel.

        Syntax: ``PART <#channel> [:<message>]``

        The optional part message is relayed in the PART broadcast.  If
        omitted, the part message defaults to the client's nick.

        After removing the client:
        - If the channel is now empty, it is destroyed (channels don't persist
          without members in a basic IRC server).
        - The PART message is broadcast to everyone who was in the channel,
          including the departing client (so the client's UI can update).

        Error cases:
          442  — Client is not in the channel.
          461  — No channel specified.
        """
        if not msg.params:
            return [
                (
                    client.id,
                    self._client_msg(
                        client, ERR_NEEDMOREPARAMS, "PART", "Not enough parameters"
                    ),
                )
            ]

        chan_name = msg.params[0].lower()
        part_msg_text = msg.params[1] if len(msg.params) > 1 else (client.nick or "")

        # Check the client is actually in this channel.
        if chan_name not in client.channels:
            return [
                (
                    client.id,
                    self._client_msg(
                        client, ERR_NOTONCHANNEL, chan_name,
                        "You're not on that channel",
                    ),
                )
            ]

        channel = self._channels.get(chan_name)
        if channel is None:
            return [
                (
                    client.id,
                    self._client_msg(
                        client, ERR_NOTONCHANNEL, chan_name,
                        "You're not on that channel",
                    ),
                )
            ]

        # Build the PART message *before* removing the client from the channel
        # so that they still appear in the member list and receive their own
        # PART broadcast.
        part_broadcast = Message(
            prefix=client.mask,
            command="PART",
            params=[chan_name, part_msg_text],
        )

        # Collect member IDs *before* removing the client.
        member_ids = list(channel.members.keys())

        # Remove the client from the channel.
        channel.members.pop(client.id, None)
        client.channels.discard(chan_name)

        # Broadcast PART to all former members (including the departing client).
        responses: list[Response] = [
            (member_id, part_broadcast) for member_id in member_ids
        ]

        # Destroy the channel if it is now empty.
        if not channel.members:
            del self._channels[chan_name]

        return responses

    def _deliver_message(
        self,
        client: Client,
        msg: Message,
        command: str,
    ) -> list[Response]:
        """Common logic for PRIVMSG and NOTICE delivery.

        Both commands use the same delivery mechanics:
        - Target is either a channel (starts with ``#``) or a nick.
        - For channels: deliver to all members except the sender.
        - For nicks: deliver directly to the target client.

        Error codes:
          411  — No target given.
          412  — No text given.
          401  — Target nick not found.
          403  — Target channel not found.
        """
        if not msg.params:
            return [
                (
                    client.id,
                    self._client_msg(
                        client, ERR_NEEDMOREPARAMS, command, "No recipient given"
                    ),
                )
            ]

        target = msg.params[0]
        text = msg.params[1] if len(msg.params) > 1 else ""

        if not text:
            return [
                (client.id, self._client_msg(client, "412", "No text to send"))
            ]

        out_msg = Message(
            prefix=client.mask,
            command=command,
            params=[target, text],
        )

        responses: list[Response] = []

        if target.startswith("#"):
            # ── Channel message ──────────────────────────────────────────────
            chan_name = target.lower()
            channel = self._channels.get(chan_name)
            if channel is None:
                return [
                    (
                        client.id,
                        self._client_msg(
                            client, ERR_NOSUCHCHANNEL, target, "No such channel"
                        ),
                    )
                ]
            # Deliver to all members except the sender.
            for member_conn_id, _member in channel.members.items():
                if member_conn_id != client.id:
                    responses.append((member_conn_id, out_msg))
        else:
            # ── Private message to a nick ────────────────────────────────────
            target_conn = self._nicks.get(target.lower())
            if target_conn is None:
                return [
                    (
                        client.id,
                        self._client_msg(
                            client, ERR_NOSUCHNICK, target, "No such nick/channel"
                        ),
                    )
                ]
            target_client = self._clients.get(target_conn)
            if target_client is None:
                return [
                    (
                        client.id,
                        self._client_msg(
                            client, ERR_NOSUCHNICK, target, "No such nick/channel"
                        ),
                    )
                ]
            responses.append((target_conn, out_msg))
            # For PRIVMSG (not NOTICE), if the target is away, inform the sender.
            if command == "PRIVMSG" and target_client.away_message is not None:
                responses.append((
                    client.id,
                    self._client_msg(
                        client, RPL_AWAY,
                        target_client.nick or target,
                        target_client.away_message,
                    ),
                ))

        return responses

    def _handle_privmsg(self, client: Client, msg: Message) -> list[Response]:
        """Handle the PRIVMSG command — send a message to a nick or channel.

        PRIVMSG is the primary way users communicate in IRC.  It delivers text
        to a specific nick or to all members of a channel (except the sender).

        If the recipient nick is away, an automatic 301 RPL_AWAY reply is sent
        back to the sender with the away message text.  NOTICE does not trigger
        this automatic reply — see _handle_notice.
        """
        return self._deliver_message(client, msg, "PRIVMSG")

    def _handle_notice(self, client: Client, msg: Message) -> list[Response]:
        """Handle the NOTICE command — send a notice to a nick or channel.

        NOTICE behaves exactly like PRIVMSG for delivery, but by convention
        (and per RFC 1459) servers and clients must NEVER auto-reply to a
        NOTICE.  This prevents infinite loops between bots and servers.

        We share the delivery logic with PRIVMSG via ``_deliver_message``, but
        we pass ``"NOTICE"`` as the command so away-message auto-replies are
        suppressed (``_deliver_message`` only sends 301 for PRIVMSG).
        """
        return self._deliver_message(client, msg, "NOTICE")

    def _handle_names(self, client: Client, msg: Message) -> list[Response]:
        """Handle the NAMES command — list members of a channel.

        Syntax: ``NAMES [<#channel>]``

        Returns 353 (NAMREPLY) + 366 (ENDOFNAMES) for the requested channel.
        If no channel is specified, we return NAMES for all channels (not
        commonly needed but required by the spec).

        Error cases (no error, just empty response for unknown channels):
          If the channel doesn't exist, we still send the 366 terminator.
        """
        nick = client.nick or "*"

        if msg.params:
            chan_name = msg.params[0].lower()
            channel = self._channels.get(chan_name)
            if channel:
                return self._names(channel, nick)
            else:
                # Channel not found — send just the terminator.
                return [
                    (
                        client.id,
                        self._make_msg(
                            RPL_ENDOFNAMES, nick, chan_name, "End of /NAMES list"
                        ),
                    )
                ]
        else:
            # No channel specified — send NAMES for all channels.
            responses: list[Response] = []
            for channel in self._channels.values():
                responses.extend(self._names(channel, nick))
            return responses

    def _handle_list(self, client: Client, msg: Message) -> list[Response]:
        """Handle the LIST command — enumerate all channels.

        Returns:
          321  — RPL_LISTSTART header.
          322  — RPL_LIST, one per channel: name, member count, topic.
          323  — RPL_LISTEND terminator.

        The 322 format is: ``<channel> <visible_count> :<topic>``
        """
        nick = client.nick or "*"
        responses: list[Response] = [
            (client.id, self._make_msg(RPL_LISTSTART, nick, "Channel", "Users  Name")),
        ]

        for channel in self._channels.values():
            responses.append((
                client.id,
                self._make_msg(
                    RPL_LIST,
                    nick,
                    channel.name,
                    str(len(channel.members)),
                    channel.topic,
                ),
            ))

        responses.append((client.id, self._make_msg(RPL_LISTEND, nick, "End of /LIST")))
        return responses

    def _handle_topic(self, client: Client, msg: Message) -> list[Response]:
        """Handle the TOPIC command — get or set a channel's topic.

        Syntax:
          ``TOPIC <#channel>``           — query the current topic.
          ``TOPIC <#channel> :<topic>``  — set a new topic.

        Query responses:
          332  — RPL_TOPIC if a topic is set.
          331  — RPL_NOTOPIC if no topic is set.

        Set behaviour:
          Update the channel's topic and broadcast ``TOPIC`` to all members.

        Error cases:
          442  — Sender is not in the channel.
          461  — No channel specified.
        """
        if not msg.params:
            return [
                (
                    client.id,
                    self._client_msg(
                        client, ERR_NEEDMOREPARAMS, "TOPIC", "Not enough parameters"
                    ),
                )
            ]

        chan_name = msg.params[0].lower()
        nick = client.nick or "*"

        channel = self._channels.get(chan_name)
        if channel is None:
            return [
                (
                    client.id,
                    self._client_msg(
                        client, ERR_NOSUCHCHANNEL, chan_name, "No such channel"
                    ),
                )
            ]

        # Check the client is in the channel.
        if client.id not in channel.members:
            return [
                (
                    client.id,
                    self._client_msg(
                        client, ERR_NOTONCHANNEL, chan_name,
                        "You're not on that channel",
                    ),
                )
            ]

        if len(msg.params) < 2:
            # ── Query mode ───────────────────────────────────────────────────
            if channel.topic:
                return [
                    (
                        client.id,
                        self._make_msg(RPL_TOPIC, nick, chan_name, channel.topic),
                    )
                ]
            else:
                return [
                    (
                        client.id,
                        self._make_msg(RPL_NOTOPIC, nick, chan_name, "No topic is set"),
                    )
                ]
        else:
            # ── Set mode ─────────────────────────────────────────────────────
            new_topic = msg.params[1]
            channel.topic = new_topic

            # Broadcast the new topic to all channel members.
            topic_broadcast = Message(
                prefix=client.mask,
                command="TOPIC",
                params=[chan_name, new_topic],
            )
            return [(member_id, topic_broadcast) for member_id in channel.members]

    def _handle_kick(self, client: Client, msg: Message) -> list[Response]:
        """Handle the KICK command — remove a member from a channel (op only).

        Syntax: ``KICK <#channel> <nick> [:<reason>]``

        Only channel operators may use KICK.  If the kicker is not an operator,
        they receive ``482 ERR_CHANOPRIVSNEEDED``.

        After kicking:
        - Broadcast ``:kicker!user@host KICK #channel victim :<reason>`` to all
          current channel members (so they know the victim was removed).
        - Remove the victim from the channel's member list.
        - Remove the channel from the victim's channel set.

        Error cases:
          441  — Target nick is not in the channel.
          442  — Kicker is not in the channel.
          461  — Not enough parameters.
          482  — Kicker is not a channel operator.
        """
        if len(msg.params) < 2:
            return [
                (
                    client.id,
                    self._client_msg(
                        client, ERR_NEEDMOREPARAMS, "KICK", "Not enough parameters"
                    ),
                )
            ]

        chan_name = msg.params[0].lower()
        target_nick = msg.params[1]
        reason = msg.params[2] if len(msg.params) > 2 else (client.nick or "")

        channel = self._channels.get(chan_name)
        if channel is None:
            return [
                (
                    client.id,
                    self._client_msg(
                        client, ERR_NOSUCHCHANNEL, chan_name, "No such channel"
                    ),
                )
            ]

        # Verify the kicker is in the channel.
        kicker_member = channel.members.get(client.id)
        if kicker_member is None:
            return [
                (
                    client.id,
                    self._client_msg(
                        client, ERR_NOTONCHANNEL, chan_name,
                        "You're not on that channel",
                    ),
                )
            ]

        # Verify the kicker has operator privileges.
        if not kicker_member.is_operator:
            return [
                (
                    client.id,
                    self._client_msg(
                        client, ERR_CHANOPRIVSNEEDED, chan_name,
                        "You're not channel operator",
                    ),
                )
            ]

        # Find the target nick in the channel.
        target_conn = self._nicks.get(target_nick.lower())
        if target_conn is None or target_conn not in channel.members:
            return [
                (
                    client.id,
                    self._client_msg(
                        client, ERR_USERNOTINCHANNEL,
                        target_nick, chan_name, "They aren't on that channel",
                    ),
                )
            ]

        target_client = self._clients.get(target_conn)

        # Broadcast KICK to all current members (before removing the victim).
        kick_broadcast = Message(
            prefix=client.mask,
            command="KICK",
            params=[chan_name, target_nick, reason],
        )
        responses: list[Response] = [
            (member_id, kick_broadcast) for member_id in channel.members
        ]

        # Remove victim from the channel.
        channel.members.pop(target_conn, None)
        if target_client:
            target_client.channels.discard(chan_name)

        # Destroy channel if empty.
        if not channel.members:
            del self._channels[chan_name]

        return responses

    def _handle_invite(self, client: Client, msg: Message) -> list[Response]:
        """Handle the INVITE command — invite a nick to a channel.

        Syntax: ``INVITE <nick> <#channel>``

        Sends an INVITE message directly to the target nick.  The inviting
        client receives 341 RPL_INVITING as confirmation.

        We do not enforce +i (invite-only) mode in v1, so any channel member
        can invite anyone.  In a full implementation, invites to non-+i channels
        might just be advisory.

        Error cases:
          401  — Target nick not found.
          442  — Inviter not in channel.
          461  — Not enough parameters.
        """
        if len(msg.params) < 2:
            return [
                (
                    client.id,
                    self._client_msg(
                        client, ERR_NEEDMOREPARAMS, "INVITE", "Not enough parameters"
                    ),
                )
            ]

        target_nick = msg.params[0]
        chan_name = msg.params[1].lower()
        nick = client.nick or "*"

        # Find the target.
        target_conn = self._nicks.get(target_nick.lower())
        if target_conn is None:
            return [
                (
                    client.id,
                    self._client_msg(
                        client, ERR_NOSUCHNICK, target_nick, "No such nick/channel"
                    ),
                )
            ]

        # Confirmation to the inviter.
        responses: list[Response] = [
            (
                client.id,
                self._make_msg(RPL_INVITING, nick, chan_name, target_nick),
            )
        ]

        # Send INVITE to the target.
        invite_msg = Message(
            prefix=client.mask,
            command="INVITE",
            params=[target_nick, chan_name],
        )
        responses.append((target_conn, invite_msg))

        return responses

    def _handle_mode(self, client: Client, msg: Message) -> list[Response]:
        """Handle the MODE command — query or set channel/user modes.

        This v1 implementation supports:
        - ``MODE #channel``       → 324 RPL_CHANNELMODEIS (current channel modes).
        - ``MODE nick``           → 221 RPL_UMODEIS (current user modes).
        - ``MODE #channel +/-X``  → acknowledge with a MODE broadcast.
        - ``MODE nick +/-X``      → acknowledge with a MODE broadcast.

        Full mode enforcement (e.g., +k for key, +l for limit) is out of scope
        for v1.  We accept mode strings and echo them back without enforcing them.

        Error cases:
          461  — No target specified.
        """
        if not msg.params:
            return [
                (
                    client.id,
                    self._client_msg(
                        client, ERR_NEEDMOREPARAMS, "MODE", "Not enough parameters"
                    ),
                )
            ]

        target = msg.params[0]
        nick = client.nick or "*"

        if target.startswith("#"):
            # ── Channel MODE ─────────────────────────────────────────────────
            chan_name = target.lower()
            channel = self._channels.get(chan_name)
            if channel is None:
                return [
                    (
                        client.id,
                        self._client_msg(
                            client, ERR_NOSUCHCHANNEL, chan_name, "No such channel"
                        ),
                    )
                ]

            if len(msg.params) == 1:
                # Query: return current channel modes.
                mode_str = (
                    "+" + "".join(sorted(channel.modes)) if channel.modes else "+"
                )
                return [
                    (
                        client.id,
                        self._make_msg(RPL_CHANNELMODEIS, nick, chan_name, mode_str),
                    )
                ]
            else:
                # Set: acknowledge by broadcasting MODE to channel members.
                mode_str = msg.params[1]
                # Apply simple single-char modes (no parameters in v1).
                if mode_str.startswith("+"):
                    for ch in mode_str[1:]:
                        channel.modes.add(ch)
                elif mode_str.startswith("-"):
                    for ch in mode_str[1:]:
                        channel.modes.discard(ch)

                mode_broadcast = Message(
                    prefix=client.mask,
                    command="MODE",
                    params=[chan_name, mode_str],
                )
                return [(member_id, mode_broadcast) for member_id in channel.members]
        else:
            # ── User MODE ────────────────────────────────────────────────────
            if len(msg.params) == 1:
                # Query: return user modes.  We don't track user modes in v1.
                return [
                    (
                        client.id,
                        self._make_msg("221", nick, "+"),
                    )
                ]
            else:
                # Set user mode: acknowledge.
                mode_str = msg.params[1]
                mode_broadcast = Message(
                    prefix=client.mask,
                    command="MODE",
                    params=[target, mode_str],
                )
                return [(client.id, mode_broadcast)]

    def _handle_ping(self, client: Client, msg: Message) -> list[Response]:
        """Handle the PING command — keepalive from client to server.

        Syntax: ``PING :<server>``

        The client sends PING periodically to verify the connection is alive.
        We respond with a matching PONG.  If the client doesn't see a PONG
        within its timeout window, it will close the connection.

        The PONG carries the same server token the client sent in the PING,
        which lets the client match the response to its request.
        """
        server_token = msg.params[0] if msg.params else self._server_name
        return [
            (
                client.id,
                Message(
                    prefix=self._server_name,
                    command="PONG",
                    params=[self._server_name, server_token],
                ),
            )
        ]

    def _handle_pong(self, client: Client, msg: Message) -> list[Response]:
        """Handle the PONG command — client's response to a server PING.

        Servers send PING to verify clients are still alive; clients reply with
        PONG.  We don't send server-initiated PINGs in v1, so we simply ignore
        any PONG we receive from a client.
        """
        # Nothing to do.  In a full implementation we'd update a last-seen
        # timestamp here and cancel the connection-timeout timer.
        return []

    def _handle_away(self, client: Client, msg: Message) -> list[Response]:
        """Handle the AWAY command — set or clear away status.

        Syntax:
          ``AWAY :<message>``  — mark as away with the given message.
          ``AWAY``             — clear away status (mark as present).

        Responses:
          306  — RPL_NOWAWAY when away message is set.
          305  — RPL_UNAWAY when away status is cleared.

        When another client sends PRIVMSG to an away user, the server
        automatically sends 301 RPL_AWAY with the away message text.
        """
        if msg.params and msg.params[0]:
            # Setting away.
            client.away_message = msg.params[0]
            return [
                (
                    client.id,
                    self._client_msg(
                        client, RPL_NOWAWAY, "You have been marked as being away"
                    ),
                )
            ]
        else:
            # Clearing away.
            client.away_message = None
            return [
                (
                    client.id,
                    self._client_msg(
                        client, RPL_UNAWAY, "You are no longer marked as being away"
                    ),
                )
            ]

    def _handle_whois(self, client: Client, msg: Message) -> list[Response]:
        """Handle the WHOIS command — retrieve information about a nick.

        Syntax: ``WHOIS <nick>``

        Returns a sequence of numerics describing the target user:
          311  — RPL_WHOISUSER: nick, username, hostname, realname.
          312  — RPL_WHOISSERVER: which server the user is on.
          319  — RPL_WHOISCHANNELS: list of channels the user is in.
          301  — RPL_AWAY: away message (only if user is away).
          318  — RPL_ENDOFWHOIS: terminator.

        Error cases:
          401  — Nick not found.
          461  — No nick specified.
        """
        if not msg.params:
            return [
                (
                    client.id,
                    self._client_msg(
                        client, ERR_NEEDMOREPARAMS, "WHOIS", "Not enough parameters"
                    ),
                )
            ]

        target_nick = msg.params[0]
        nick = client.nick or "*"

        target_conn = self._nicks.get(target_nick.lower())
        if target_conn is None:
            return [
                (
                    client.id,
                    self._client_msg(
                        client, ERR_NOSUCHNICK, target_nick, "No such nick/channel"
                    ),
                )
            ]

        target = self._clients.get(target_conn)
        if target is None:
            return [
                (
                    client.id,
                    self._client_msg(
                        client, ERR_NOSUCHNICK, target_nick, "No such nick/channel"
                    ),
                )
            ]

        target_nick_str = target.nick or target_nick
        responses: list[Response] = [
            # 311: nick user host * :realname
            (
                client.id,
                self._make_msg(
                    RPL_WHOISUSER,
                    nick,
                    target_nick_str,
                    target.username or "*",
                    target.hostname,
                    "*",
                    target.realname or "",
                ),
            ),
            # 312: nick server :server info
            (
                client.id,
                self._make_msg(
                    RPL_WHOISSERVER,
                    nick,
                    target_nick_str,
                    self._server_name,
                    "IRC server",
                ),
            ),
        ]

        # 319: channels the user is in.
        if target.channels:
            chan_list = " ".join(sorted(target.channels))
            responses.append((
                client.id,
                self._make_msg(RPL_WHOISCHANNELS, nick, target_nick_str, chan_list),
            ))

        # 301: away message if applicable.
        if target.away_message is not None:
            responses.append((
                client.id,
                self._make_msg(RPL_AWAY, nick, target_nick_str, target.away_message),
            ))

        # 318: terminator.
        responses.append((
            client.id,
            self._make_msg(RPL_ENDOFWHOIS, nick, target_nick_str, "End of /WHOIS list"),
        ))

        return responses

    def _handle_who(self, client: Client, msg: Message) -> list[Response]:
        """Handle the WHO command — list users matching a mask.

        Syntax: ``WHO [<mask>]``

        Returns 352 (WHOREPLY) rows followed by 315 (ENDOFWHO).

        The 352 format is:
          ``<channel> <user> <host> <server> <nick> H|G :<hopcount> <realname>``

        ``H`` = here (not away), ``G`` = gone (away).

        If a mask is provided and starts with ``#``, we return only members of
        that channel.  Otherwise we return all registered users (simplified
        behaviour — a real server would pattern-match against nick/host).
        """
        nick = client.nick or "*"
        mask = msg.params[0] if msg.params else "*"
        responses: list[Response] = []

        def who_row(target_client: Client, channel_name: str = "*") -> Response:
            here_or_gone = "G" if target_client.away_message is not None else "H"
            return (
                client.id,
                self._make_msg(
                    RPL_WHOREPLY,
                    nick,
                    channel_name,
                    target_client.username or "*",
                    target_client.hostname,
                    self._server_name,
                    target_client.nick or "*",
                    here_or_gone,
                    f"0 {target_client.realname or ''}",
                ),
            )

        if mask.startswith("#"):
            # List members of the given channel.
            chan_name = mask.lower()
            channel = self._channels.get(chan_name)
            if channel:
                for member in channel.members.values():
                    responses.append(who_row(member.client, chan_name))
        else:
            # List all registered clients.
            for c in self._clients.values():
                if c.registered:
                    responses.append(who_row(c))

        responses.append((
            client.id,
            self._make_msg(RPL_ENDOFWHO, nick, mask, "End of /WHO list"),
        ))
        return responses

    def _handle_oper(self, client: Client, msg: Message) -> list[Response]:
        """Handle the OPER command — gain IRC operator privileges.

        Syntax: ``OPER <name> <password>``

        If the supplied password matches the server's configured oper password,
        the client's ``is_oper`` flag is set to True and they receive 381
        RPL_YOUREOPER.  Otherwise they receive 464 ERR_PASSWDMISMATCH.

        In a production server, OPER would use per-operator named entries and
        host restrictions.  Here we use a single global password for simplicity.

        Error cases:
          461  — Not enough parameters.
          464  — Wrong password.
        """
        if len(msg.params) < 2:
            return [
                (
                    client.id,
                    self._client_msg(
                        client, ERR_NEEDMOREPARAMS, "OPER", "Not enough parameters"
                    ),
                )
            ]

        # We ignore the name (params[0]); only the password matters.
        password = msg.params[1]

        if self._oper_password and password == self._oper_password:
            client.is_oper = True
            return [
                (
                    client.id,
                    self._client_msg(
                        client, RPL_YOUREOPER, "You are now an IRC operator"
                    ),
                )
            ]
        else:
            return [
                (
                    client.id,
                    self._client_msg(client, ERR_PASSWDMISMATCH, "Password incorrect"),
                )
            ]
