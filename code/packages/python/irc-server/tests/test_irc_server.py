"""Tests for irc-server — IRC state machine.

These tests exercise the IRCServer class in isolation, with no I/O.  Every
test follows the same pattern:

  1. Construct an IRCServer.
  2. Call on_connect() to register one or more connections.
  3. Drive the state machine via on_message() with parsed Message objects.
  4. Assert on the list of (ConnId, Message) Response tuples returned.

Helpers at the top of the file reduce boilerplate and make assertions readable.
"""

from __future__ import annotations

from irc_proto import Message, parse

from irc_server import (
    ConnId,
    IRCServer,
    Response,
)

# ---------------------------------------------------------------------------
# Test helpers
# ---------------------------------------------------------------------------

def make_server(
    name: str = "irc.test",
    motd: list[str] | None = None,
    oper_password: str = "secret",
) -> IRCServer:
    """Create a fresh IRCServer for each test."""
    return IRCServer(
        server_name=name, version="1.0", motd=motd, oper_password=oper_password
    )


def connect(server: IRCServer, conn_id: int, host: str = "127.0.0.1") -> ConnId:
    """Connect a client and return its ConnId."""
    cid = ConnId(conn_id)
    server.on_connect(cid, host)
    return cid


def send(server: IRCServer, conn_id: ConnId, raw: str) -> list[Response]:
    """Send a raw IRC line and return the response list."""
    return server.on_message(conn_id, parse(raw))


def register(
    server: IRCServer,
    conn_id: int,
    nick: str = "alice",
    user: str = "alice",
    realname: str = "Alice Smith",
    host: str = "127.0.0.1",
) -> ConnId:
    """Connect and complete the NICK+USER registration handshake.

    Returns the ConnId.  The welcome sequence is discarded; callers only
    care about post-registration responses in most tests.
    """
    cid = connect(server, conn_id, host)
    send(server, cid, f"NICK {nick}")
    send(server, cid, f"USER {user} 0 * :{realname}")
    return cid


def commands(responses: list[Response]) -> list[str]:
    """Extract just the command strings from a list of responses."""
    return [msg.command for _, msg in responses]


def numerics(responses: list[Response]) -> list[str]:
    """Extract numeric reply codes from a list of responses."""
    return [msg.command for _, msg in responses if msg.command.isdigit()]


def find(responses: list[Response], command: str) -> list[Message]:
    """Return all messages with the given command from a response list."""
    return [msg for _, msg in responses if msg.command == command]


def targets(responses: list[Response]) -> list[ConnId]:
    """Extract the ConnId targets from a response list."""
    return [cid for cid, _ in responses]


# ---------------------------------------------------------------------------
# Registration
# ---------------------------------------------------------------------------

class TestRegistration:
    """Tests for the NICK+USER registration handshake."""

    def test_nick_then_user_sends_welcome(self) -> None:
        """NICK followed by USER triggers the 001–376 welcome sequence."""
        server = make_server()
        cid = connect(server, 1)

        # NICK alone sends nothing.
        r1 = send(server, cid, "NICK alice")
        assert r1 == []

        # USER completes registration — welcome sequence fires.
        r2 = send(server, cid, "USER alice 0 * :Alice Smith")
        cmds = commands(r2)
        assert "001" in cmds, "001 RPL_WELCOME must be in the welcome sequence"
        assert "376" in cmds, "376 RPL_ENDOFMOTD must close the welcome sequence"

    def test_user_then_nick_sends_welcome(self) -> None:
        """USER before NICK also triggers the welcome sequence once NICK arrives."""
        server = make_server()
        cid = connect(server, 1)

        # USER alone sends nothing (NICK not yet set).
        r1 = send(server, cid, "USER alice 0 * :Alice Smith")
        assert r1 == []

        # NICK completes registration.
        r2 = send(server, cid, "NICK alice")
        assert "001" in commands(r2)

    def test_welcome_001_first(self) -> None:
        """001 must be the very first message in the welcome sequence."""
        server = make_server()
        cid = connect(server, 1)
        send(server, cid, "NICK alice")
        responses = send(server, cid, "USER alice 0 * :Alice Smith")
        assert responses[0][1].command == "001"

    def test_welcome_contains_nick_in_params(self) -> None:
        """The 001 message params must include the client's nick."""
        server = make_server()
        cid = connect(server, 1)
        send(server, cid, "NICK alice")
        responses = send(server, cid, "USER alice 0 * :Alice Smith")
        welcome = find(responses, "001")[0]
        assert "alice" in welcome.params

    def test_welcome_motd_lines(self) -> None:
        """Each MOTD line produces a 372 reply; zero MOTD lines means no 372."""
        server = make_server(motd=["Hello", "World"])
        cid = connect(server, 1)
        send(server, cid, "NICK alice")
        responses = send(server, cid, "USER alice 0 * :Alice Smith")
        motd_lines = [msg for _, msg in responses if msg.command == "372"]
        assert len(motd_lines) == 2

    def test_welcome_no_motd(self) -> None:
        """An empty MOTD still sends 375 and 376 but no 372."""
        server = make_server(motd=[])
        cid = connect(server, 1)
        send(server, cid, "NICK alice")
        responses = send(server, cid, "USER alice 0 * :Alice Smith")
        motd_lines = [msg for _, msg in responses if msg.command == "372"]
        assert motd_lines == []
        assert "375" in commands(responses)
        assert "376" in commands(responses)

    def test_command_before_registration_returns_451(self) -> None:
        """Any non-registration command before NICK+USER gets 451 ERR_NOTREGISTERED."""
        server = make_server()
        cid = connect(server, 1)
        responses = send(server, cid, "JOIN #general")
        assert "451" in commands(responses)

    def test_cap_allowed_before_registration(self) -> None:
        """CAP is allowed before registration and does not cause 451."""
        server = make_server()
        cid = connect(server, 1)
        responses = send(server, cid, "CAP LS")
        assert "451" not in commands(responses)

    def test_cap_returns_ack(self) -> None:
        """CAP command always returns a CAP ACK."""
        server = make_server()
        cid = connect(server, 1)
        responses = send(server, cid, "CAP LS")
        cap_msgs = find(responses, "CAP")
        assert cap_msgs, "CAP response expected"
        assert "ACK" in cap_msgs[0].params

    def test_duplicate_user_ignored(self) -> None:
        """A second USER command after registration is silently ignored."""
        server = make_server()
        cid = register(server, 1)
        # Sending USER again should not crash or re-send welcome.
        responses = send(server, cid, "USER bob 0 * :Bob")
        # Should not contain 001 again.
        assert "001" not in commands(responses)


# ---------------------------------------------------------------------------
# Nick management
# ---------------------------------------------------------------------------

class TestNick:
    """Tests for NICK command: validation, uniqueness, and nick changes."""

    def test_nick_in_use_returns_433(self) -> None:
        """Attempting to take a nick already in use gives 433 ERR_NICKNAMEINUSE."""
        server = make_server()
        _ = register(server, 1, nick="alice")
        cid2 = connect(server, 2)
        responses = send(server, cid2, "NICK alice")
        assert "433" in commands(responses)

    def test_nick_in_use_case_insensitive(self) -> None:
        """Nick uniqueness check is case-insensitive (ALICE == alice)."""
        server = make_server()
        _ = register(server, 1, nick="alice")
        cid2 = connect(server, 2)
        responses = send(server, cid2, "NICK ALICE")
        assert "433" in commands(responses)

    def test_invalid_nick_returns_432(self) -> None:
        """A nick that fails the RFC 1459 character rules gives 432."""
        server = make_server()
        cid = connect(server, 1)
        # A nick containing '!' is not in the allowed character set.
        # We construct the Message directly to avoid the IRC parser splitting on space.
        msg = Message(prefix=None, command="NICK", params=["bad!nick"])
        responses = server.on_message(cid, msg)
        assert "432" in commands(responses)

    def test_invalid_nick_too_long(self) -> None:
        """A nick longer than 9 characters is rejected with 432."""
        server = make_server()
        cid = connect(server, 1)
        responses = send(server, cid, "NICK " + "a" * 10)
        assert "432" in commands(responses)

    def test_invalid_nick_starts_with_digit(self) -> None:
        """A nick starting with a digit is rejected with 432."""
        server = make_server()
        cid = connect(server, 1)
        responses = send(server, cid, "NICK 1alice")
        assert "432" in commands(responses)

    def test_valid_nick_specials(self) -> None:
        """Nicks starting with special chars like _ or [ are valid."""
        server = make_server()
        cid = connect(server, 1)
        responses = send(server, cid, "NICK _bot")
        # No 432; nick should be accepted (will send nothing until USER).
        assert "432" not in commands(responses)

    def test_nick_change_broadcasts_to_channel_peers(self) -> None:
        """A registered user changing their nick broadcasts NICK to channel peers."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        bob = register(server, 2, nick="bob")
        send(server, alice, "JOIN #general")
        send(server, bob, "JOIN #general")

        # Alice changes nick.
        responses = send(server, alice, "NICK alicia")

        # Both alice and bob should receive the NICK message.
        nick_msgs = [cid for cid, msg in responses if msg.command == "NICK"]
        assert alice in nick_msgs, "alice (the changer) must receive the NICK broadcast"
        assert bob in nick_msgs, "bob (a channel peer) must receive the NICK broadcast"

    def test_nick_change_updates_nick_in_use_check(self) -> None:
        """After changing nick, the old nick becomes available again."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        send(server, alice, "NICK alicia")

        # Now "alice" should be free.
        cid2 = connect(server, 2)
        responses = send(server, cid2, "NICK alice")
        assert "433" not in commands(responses)

    def test_nick_no_params_returns_431(self) -> None:
        """NICK with no arguments returns 431 ERR_NONICKNAMEGIVEN."""
        server = make_server()
        cid = connect(server, 1)
        # We can't use parse() for this because NICK with no params
        # won't parse well; construct Message directly.
        msg = Message(prefix=None, command="NICK", params=[])
        responses = server.on_message(cid, msg)
        assert "431" in commands(responses)

    def test_same_nick_change_allowed(self) -> None:
        """A client can re-set their own nick to the same value (no 433)."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        responses = send(server, alice, "NICK alice")
        assert "433" not in commands(responses)


# ---------------------------------------------------------------------------
# Channels — JOIN, PART, NAMES
# ---------------------------------------------------------------------------

class TestChannels:
    """Tests for channel join/part lifecycle and member listing."""

    def test_join_creates_channel(self) -> None:
        """Joining a non-existent channel creates it."""
        server = make_server()
        alice = register(server, 1)
        responses = send(server, alice, "JOIN #general")
        join_msgs = find(responses, "JOIN")
        assert join_msgs, "JOIN must be broadcast on channel join"

    def test_join_first_member_is_operator(self) -> None:
        """The first client to join a channel gets operator status (@)."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        send(server, alice, "JOIN #general")

        # NAMES response should list @alice.
        responses = send(server, alice, "NAMES #general")
        namreply = find(responses, "353")
        assert namreply, "353 NAMREPLY expected"
        names_str = namreply[0].params[-1]
        assert "@alice" in names_str, "First member should be @alice (operator)"

    def test_join_second_member_not_operator(self) -> None:
        """The second client to join is a regular member (no @ prefix)."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        bob = register(server, 2, nick="bob")
        send(server, alice, "JOIN #general")
        send(server, bob, "JOIN #general")

        responses = send(server, alice, "NAMES #general")
        namreply = find(responses, "353")
        names_str = namreply[0].params[-1]
        assert "@alice" in names_str
        assert "bob" in names_str
        assert "@bob" not in names_str, "Second member should NOT be an operator"

    def test_join_broadcasts_to_existing_members(self) -> None:
        """When a new client joins, existing members receive the JOIN broadcast."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        bob = register(server, 2, nick="bob")
        send(server, alice, "JOIN #general")

        # Bob joins.
        responses = send(server, bob, "JOIN #general")
        join_targets = [cid for cid, msg in responses if msg.command == "JOIN"]
        assert alice in join_targets, "alice must receive the JOIN broadcast for bob"
        assert bob in join_targets, "bob must also receive JOIN (to confirm join)"

    def test_join_sends_notopic_when_empty(self) -> None:
        """Joining a channel with no topic sends 331 RPL_NOTOPIC."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        responses = send(server, alice, "JOIN #general")
        assert "331" in commands(responses)

    def test_join_sends_topic_when_set(self) -> None:
        """Joining a channel that already has a topic sends 332 RPL_TOPIC."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        bob = register(server, 2, nick="bob")
        send(server, alice, "JOIN #general")
        send(server, alice, "TOPIC #general :Welcome here")
        responses = send(server, bob, "JOIN #general")
        assert "332" in commands(responses)

    def test_part_removes_member(self) -> None:
        """PART removes the client from the channel and broadcasts PART."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        bob = register(server, 2, nick="bob")
        send(server, alice, "JOIN #general")
        send(server, bob, "JOIN #general")

        responses = send(server, bob, "PART #general")
        part_msgs = find(responses, "PART")
        assert part_msgs, "PART must be broadcast"

        # After part, bob should not be in NAMES.
        names_resp = send(server, alice, "NAMES #general")
        namreply = find(names_resp, "353")
        names_str = namreply[0].params[-1]
        assert "bob" not in names_str

    def test_part_destroys_empty_channel(self) -> None:
        """When the last member parts, the channel is destroyed."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        send(server, alice, "JOIN #general")
        send(server, alice, "PART #general")

        # Listing channels should show none.
        responses = send(server, alice, "LIST")
        list_items = find(responses, "322")
        assert list_items == []

    def test_part_not_in_channel_returns_442(self) -> None:
        """PARTing a channel you're not in returns 442 ERR_NOTONCHANNEL."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        responses = send(server, alice, "PART #nonexistent")
        assert "442" in commands(responses)

    def test_part_broadcasts_to_all_former_members(self) -> None:
        """PART is broadcast to everyone who was in the channel."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        bob = register(server, 2, nick="bob")
        carol = register(server, 3, nick="carol")
        send(server, alice, "JOIN #general")
        send(server, bob, "JOIN #general")
        send(server, carol, "JOIN #general")

        responses = send(server, carol, "PART #general")
        part_targets = {cid for cid, msg in responses if msg.command == "PART"}
        assert alice in part_targets
        assert bob in part_targets
        assert carol in part_targets

    def test_names_endofnames_included(self) -> None:
        """NAMES response always includes a 366 RPL_ENDOFNAMES terminator."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        send(server, alice, "JOIN #general")
        responses = send(server, alice, "NAMES #general")
        assert "366" in commands(responses)

    def test_names_unknown_channel(self) -> None:
        """NAMES for a non-existent channel still returns 366."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        responses = send(server, alice, "NAMES #notexist")
        assert "366" in commands(responses)


# ---------------------------------------------------------------------------
# Messaging — PRIVMSG, NOTICE
# ---------------------------------------------------------------------------

class TestMessaging:
    """Tests for PRIVMSG and NOTICE delivery."""

    def test_privmsg_to_channel_delivered_to_others(self) -> None:
        """PRIVMSG to channel: all members except sender receive the message."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        bob = register(server, 2, nick="bob")
        carol = register(server, 3, nick="carol")
        send(server, alice, "JOIN #general")
        send(server, bob, "JOIN #general")
        send(server, carol, "JOIN #general")

        responses = send(server, alice, "PRIVMSG #general :hello everyone")
        priv_targets = {cid for cid, msg in responses if msg.command == "PRIVMSG"}

        assert alice not in priv_targets, "Sender must NOT receive their own PRIVMSG"
        assert bob in priv_targets, "bob must receive the channel PRIVMSG"
        assert carol in priv_targets, "carol must receive the channel PRIVMSG"

    def test_privmsg_to_nick_delivered(self) -> None:
        """PRIVMSG to a nick delivers directly to that client."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        bob = register(server, 2, nick="bob")

        responses = send(server, alice, "PRIVMSG bob :hi bob")
        priv_targets = [cid for cid, msg in responses if msg.command == "PRIVMSG"]
        assert bob in priv_targets
        assert alice not in priv_targets

    def test_privmsg_missing_nick_returns_401(self) -> None:
        """PRIVMSG to an unknown nick returns 401 ERR_NOSUCHNICK."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        responses = send(server, alice, "PRIVMSG nobody :hello")
        assert "401" in commands(responses)

    def test_privmsg_missing_channel_returns_403(self) -> None:
        """PRIVMSG to an unknown channel returns 403 ERR_NOSUCHCHANNEL."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        responses = send(server, alice, "PRIVMSG #nobody :hello")
        assert "403" in commands(responses)

    def test_privmsg_no_text_returns_412(self) -> None:
        """PRIVMSG with no text returns 412 ERR_NOTEXTTOSEND."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        register(server, 2, nick="bob")
        msg = Message(prefix=None, command="PRIVMSG", params=["bob", ""])
        responses = server.on_message(alice, msg)
        assert "412" in commands(responses)

    def test_notice_to_channel_no_auto_reply(self) -> None:
        """NOTICE works like PRIVMSG for delivery but never sends auto-replies."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        bob = register(server, 2, nick="bob")
        send(server, alice, "JOIN #general")
        send(server, bob, "JOIN #general")

        # Bob is away.
        send(server, bob, "AWAY :gone")

        # Alice sends NOTICE to channel — no 301 auto-reply expected.
        responses = send(server, alice, "NOTICE #general :hey")
        assert "301" not in commands(responses)

    def test_privmsg_to_away_user_sends_301(self) -> None:
        """PRIVMSG to an away user sends 301 RPL_AWAY back to the sender."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        bob = register(server, 2, nick="bob")
        send(server, bob, "AWAY :out for lunch")

        responses = send(server, alice, "PRIVMSG bob :are you there?")
        assert "301" in commands(responses)

    def test_notice_to_nick_no_301(self) -> None:
        """NOTICE to an away user does NOT trigger a 301 auto-reply."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        bob = register(server, 2, nick="bob")
        send(server, bob, "AWAY :brb")

        responses = send(server, alice, "NOTICE bob :hey")
        assert "301" not in commands(responses)


# ---------------------------------------------------------------------------
# QUIT and on_disconnect
# ---------------------------------------------------------------------------

class TestQuitAndDisconnect:
    """Tests for graceful QUIT and unexpected on_disconnect."""

    def test_quit_broadcasts_to_channel_members(self) -> None:
        """QUIT sends a QUIT message to all channel members."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        bob = register(server, 2, nick="bob")
        send(server, alice, "JOIN #general")
        send(server, bob, "JOIN #general")

        responses = send(server, bob, "QUIT :bye")
        quit_targets = [cid for cid, msg in responses if msg.command == "QUIT"]
        assert alice in quit_targets, "alice must receive the QUIT broadcast"

    def test_quit_sends_error_to_quitter(self) -> None:
        """QUIT sends an ERROR message to the quitting client."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        responses = send(server, alice, "QUIT :leaving")
        error_msgs = find(responses, "ERROR")
        assert error_msgs, "ERROR must be sent to the quitting client"

    def test_on_disconnect_broadcasts_quit(self) -> None:
        """Unexpected disconnect (on_disconnect) broadcasts QUIT to channel peers."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        bob = register(server, 2, nick="bob")
        send(server, alice, "JOIN #general")
        send(server, bob, "JOIN #general")

        responses = server.on_disconnect(bob)
        quit_targets = [cid for cid, msg in responses if msg.command == "QUIT"]
        assert alice in quit_targets

    def test_on_disconnect_cleans_up_nick(self) -> None:
        """After disconnect, the client's nick becomes available again."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        server.on_disconnect(alice)

        # alice nick should now be free.
        cid2 = connect(server, 2)
        responses = send(server, cid2, "NICK alice")
        assert "433" not in commands(responses)

    def test_on_disconnect_twice_is_safe(self) -> None:
        """Calling on_disconnect twice does not raise an exception."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        server.on_disconnect(alice)
        # Second call should be a no-op, not a crash.
        server.on_disconnect(alice)

    def test_quit_before_registration(self) -> None:
        """QUIT before completing registration still works cleanly."""
        server = make_server()
        cid = connect(server, 1)
        send(server, cid, "NICK alice")
        # QUIT before USER.
        responses = send(server, cid, "QUIT :aborting")
        error_msgs = find(responses, "ERROR")
        assert error_msgs


# ---------------------------------------------------------------------------
# TOPIC
# ---------------------------------------------------------------------------

class TestTopic:
    """Tests for TOPIC get and set."""

    def test_topic_get_empty_returns_331(self) -> None:
        """Querying a channel with no topic returns 331 RPL_NOTOPIC."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        send(server, alice, "JOIN #general")
        responses = send(server, alice, "TOPIC #general")
        assert "331" in commands(responses)

    def test_topic_get_set_returns_332(self) -> None:
        """Querying a channel with a topic returns 332 RPL_TOPIC."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        send(server, alice, "JOIN #general")
        send(server, alice, "TOPIC #general :Hello world")
        responses = send(server, alice, "TOPIC #general")
        assert "332" in commands(responses)

    def test_topic_set_broadcasts_to_all(self) -> None:
        """Setting a topic broadcasts TOPIC to all channel members."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        bob = register(server, 2, nick="bob")
        send(server, alice, "JOIN #general")
        send(server, bob, "JOIN #general")

        responses = send(server, alice, "TOPIC #general :New topic")
        topic_targets = {cid for cid, msg in responses if msg.command == "TOPIC"}
        assert alice in topic_targets
        assert bob in topic_targets

    def test_topic_not_in_channel_returns_442(self) -> None:
        """Trying to set/get topic for a channel you're not in gives 442."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        bob = register(server, 2, nick="bob")
        send(server, alice, "JOIN #general")
        responses = send(server, bob, "TOPIC #general")
        assert "442" in commands(responses)


# ---------------------------------------------------------------------------
# KICK
# ---------------------------------------------------------------------------

class TestKick:
    """Tests for the KICK command."""

    def test_kick_by_operator_removes_target(self) -> None:
        """Channel operator can kick a member; KICK broadcast sent."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        bob = register(server, 2, nick="bob")
        send(server, alice, "JOIN #general")
        send(server, bob, "JOIN #general")

        responses = send(server, alice, "KICK #general bob :out you go")
        kick_msgs = find(responses, "KICK")
        assert kick_msgs, "KICK broadcast expected"

        # Bob should no longer be in the channel.
        names_resp = send(server, alice, "NAMES #general")
        namreply = find(names_resp, "353")
        names_str = namreply[0].params[-1]
        assert "bob" not in names_str

    def test_kick_by_non_operator_returns_482(self) -> None:
        """Non-operator KICK returns 482 ERR_CHANOPRIVSNEEDED."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        bob = register(server, 2, nick="bob")
        carol = register(server, 3, nick="carol")
        send(server, alice, "JOIN #general")
        send(server, bob, "JOIN #general")
        send(server, carol, "JOIN #general")

        # Bob tries to kick carol — bob is not an operator.
        responses = send(server, bob, "KICK #general carol :you out")
        assert "482" in commands(responses)

    def test_kick_target_not_in_channel_returns_441(self) -> None:
        """KICKing a nick not in the channel returns 441."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        register(server, 2, nick="bob")
        send(server, alice, "JOIN #general")
        # Bob is NOT in #general.

        responses = send(server, alice, "KICK #general bob :bye")
        assert "441" in commands(responses)


# ---------------------------------------------------------------------------
# PING / PONG
# ---------------------------------------------------------------------------

class TestPing:
    """Tests for PING/PONG keepalive."""

    def test_ping_returns_pong(self) -> None:
        """PING always produces a PONG reply."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        responses = send(server, alice, "PING :irc.test")
        pong_msgs = find(responses, "PONG")
        assert pong_msgs, "PONG must be returned for a PING"

    def test_pong_returns_nothing(self) -> None:
        """PONG from client is silently ignored (returns empty list)."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        responses = send(server, alice, "PONG :irc.test")
        assert responses == []

    def test_ping_before_registration(self) -> None:
        """PING before registration returns 451."""
        server = make_server()
        cid = connect(server, 1)
        responses = send(server, cid, "PING :irc.test")
        assert "451" in commands(responses)


# ---------------------------------------------------------------------------
# AWAY
# ---------------------------------------------------------------------------

class TestAway:
    """Tests for AWAY set/clear."""

    def test_away_set_returns_306(self) -> None:
        """Setting an away message returns 306 RPL_NOWAWAY."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        responses = send(server, alice, "AWAY :gone fishing")
        assert "306" in commands(responses)

    def test_away_clear_returns_305(self) -> None:
        """Clearing away status (AWAY with no message) returns 305 RPL_UNAWAY."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        send(server, alice, "AWAY :gone")
        responses = send(server, alice, "AWAY")
        assert "305" in commands(responses)


# ---------------------------------------------------------------------------
# LIST
# ---------------------------------------------------------------------------

class TestList:
    """Tests for the LIST command."""

    def test_list_returns_liststart_and_listend(self) -> None:
        """LIST always returns 321 and 323."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        responses = send(server, alice, "LIST")
        assert "321" in commands(responses)
        assert "323" in commands(responses)

    def test_list_includes_channels(self) -> None:
        """LIST includes one 322 row per channel."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        send(server, alice, "JOIN #alpha")
        send(server, alice, "JOIN #beta")
        responses = send(server, alice, "LIST")
        list_rows = find(responses, "322")
        chan_names = [row.params[1] for row in list_rows]
        assert "#alpha" in chan_names
        assert "#beta" in chan_names


# ---------------------------------------------------------------------------
# WHOIS
# ---------------------------------------------------------------------------

class TestWhois:
    """Tests for WHOIS nick info lookup."""

    def test_whois_known_nick(self) -> None:
        """WHOIS for a known nick returns 311, 312, 318."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        register(server, 2, nick="bob")
        responses = send(server, alice, "WHOIS bob")
        assert "311" in commands(responses), "311 RPL_WHOISUSER expected"
        assert "318" in commands(responses), "318 RPL_ENDOFWHOIS expected"

    def test_whois_unknown_nick_returns_401(self) -> None:
        """WHOIS for an unknown nick returns 401 ERR_NOSUCHNICK."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        responses = send(server, alice, "WHOIS nobody")
        assert "401" in commands(responses)

    def test_whois_away_user_includes_301(self) -> None:
        """WHOIS for an away user includes 301 RPL_AWAY."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        bob = register(server, 2, nick="bob")
        send(server, bob, "AWAY :be right back")
        responses = send(server, alice, "WHOIS bob")
        assert "301" in commands(responses)

    def test_whois_includes_channels(self) -> None:
        """WHOIS includes 319 RPL_WHOISCHANNELS when the target is in channels."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        bob = register(server, 2, nick="bob")
        send(server, bob, "JOIN #general")
        responses = send(server, alice, "WHOIS bob")
        assert "319" in commands(responses)


# ---------------------------------------------------------------------------
# WHO
# ---------------------------------------------------------------------------

class TestWho:
    """Tests for the WHO command."""

    def test_who_returns_315(self) -> None:
        """WHO always includes 315 RPL_ENDOFWHO."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        responses = send(server, alice, "WHO")
        assert "315" in commands(responses)

    def test_who_channel_lists_members(self) -> None:
        """WHO #channel returns 352 rows for each member."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        bob = register(server, 2, nick="bob")
        send(server, alice, "JOIN #general")
        send(server, bob, "JOIN #general")
        responses = send(server, alice, "WHO #general")
        who_rows = find(responses, "352")
        assert len(who_rows) == 2


# ---------------------------------------------------------------------------
# OPER
# ---------------------------------------------------------------------------

class TestOper:
    """Tests for IRC operator promotion."""

    def test_oper_correct_password_returns_381(self) -> None:
        """Correct OPER password returns 381 RPL_YOUREOPER."""
        server = make_server(oper_password="secret")
        alice = register(server, 1, nick="alice")
        responses = send(server, alice, "OPER alice secret")
        assert "381" in commands(responses)

    def test_oper_wrong_password_returns_464(self) -> None:
        """Wrong OPER password returns 464 ERR_PASSWDMISMATCH."""
        server = make_server(oper_password="secret")
        alice = register(server, 1, nick="alice")
        responses = send(server, alice, "OPER alice wrong")
        assert "464" in commands(responses)


# ---------------------------------------------------------------------------
# MODE
# ---------------------------------------------------------------------------

class TestMode:
    """Tests for the MODE command."""

    def test_mode_channel_query_returns_324(self) -> None:
        """MODE #channel with no modestring returns 324 RPL_CHANNELMODEIS."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        send(server, alice, "JOIN #general")
        responses = send(server, alice, "MODE #general")
        assert "324" in commands(responses)

    def test_mode_user_query_returns_221(self) -> None:
        """MODE <nick> with no modestring returns 221 RPL_UMODEIS."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        responses = send(server, alice, "MODE alice")
        assert "221" in commands(responses)

    def test_mode_channel_set_broadcasts(self) -> None:
        """Setting a channel mode broadcasts MODE to channel members."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        bob = register(server, 2, nick="bob")
        send(server, alice, "JOIN #general")
        send(server, bob, "JOIN #general")
        responses = send(server, alice, "MODE #general +n")
        mode_targets = {cid for cid, msg in responses if msg.command == "MODE"}
        assert alice in mode_targets
        assert bob in mode_targets


# ---------------------------------------------------------------------------
# INVITE
# ---------------------------------------------------------------------------

class TestInvite:
    """Tests for the INVITE command."""

    def test_invite_known_nick(self) -> None:
        """INVITE sends INVITE to target and 341 to inviter."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        bob = register(server, 2, nick="bob")
        send(server, alice, "JOIN #secret")
        responses = send(server, alice, "INVITE bob #secret")
        # 341 to alice
        assert "341" in commands(responses)
        # INVITE to bob
        invite_targets = [cid for cid, msg in responses if msg.command == "INVITE"]
        assert bob in invite_targets

    def test_invite_unknown_nick_returns_401(self) -> None:
        """INVITE to an unknown nick returns 401."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        responses = send(server, alice, "INVITE nobody #secret")
        assert "401" in commands(responses)


# ---------------------------------------------------------------------------
# Unknown command
# ---------------------------------------------------------------------------

class TestUnknownCommand:
    """Tests for the ERR_UNKNOWNCOMMAND response."""

    def test_unknown_command_returns_421(self) -> None:
        """An unrecognised command from a registered client returns 421."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        msg = Message(prefix=None, command="FLURBLE", params=[])
        responses = server.on_message(alice, msg)
        assert "421" in commands(responses)


# ---------------------------------------------------------------------------
# NAMES without channel arg
# ---------------------------------------------------------------------------

class TestNamesAll:
    """Tests for NAMES with no channel argument."""

    def test_names_no_arg_returns_all_channels(self) -> None:
        """NAMES with no argument returns NAMES for every channel."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        bob = register(server, 2, nick="bob")
        send(server, alice, "JOIN #alpha")
        send(server, bob, "JOIN #beta")
        responses = send(server, alice, "NAMES")
        namreply = find(responses, "353")
        # Should have at least one 353 for #alpha (alice is in it).
        chan_names = [msg.params[2] for msg in namreply]
        assert "#alpha" in chan_names


# ---------------------------------------------------------------------------
# Edge cases
# ---------------------------------------------------------------------------

class TestEdgeCases:
    """Miscellaneous edge-case tests."""

    def test_on_connect_returns_empty(self) -> None:
        """on_connect never sends any messages."""
        server = make_server()
        cid = ConnId(1)
        responses = server.on_connect(cid, "127.0.0.1")
        assert responses == []

    def test_on_message_unknown_conn_returns_empty(self) -> None:
        """on_message for an unknown ConnId returns an empty list (no crash)."""
        server = make_server()
        msg = Message(prefix=None, command="NICK", params=["alice"])
        responses = server.on_message(ConnId(99), msg)
        assert responses == []

    def test_join_multiple_channels_csv(self) -> None:
        """JOIN with comma-separated channels joins all of them."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        send(server, alice, "JOIN #alpha,#beta")
        # Both channels should exist.
        responses = send(server, alice, "LIST")
        list_rows = find(responses, "322")
        chan_names = [row.params[1] for row in list_rows]
        assert "#alpha" in chan_names
        assert "#beta" in chan_names

    def test_join_already_in_channel_is_noop(self) -> None:
        """JOIN when already in the channel is a no-op (no duplicate member)."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        send(server, alice, "JOIN #general")
        send(server, alice, "JOIN #general")
        responses = send(server, alice, "NAMES #general")
        namreply = find(responses, "353")
        names_str = namreply[0].params[-1]
        # alice should appear exactly once.
        assert names_str.count("alice") == 1

    def test_part_with_message(self) -> None:
        """PART with an optional message includes it in the PART broadcast."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        send(server, alice, "JOIN #general")
        responses = send(server, alice, "PART #general :see ya")
        part_msgs = find(responses, "PART")
        assert part_msgs
        assert "see ya" in part_msgs[0].params

    def test_kick_destroys_empty_channel(self) -> None:
        """Kicking the last non-operator from a channel destroys it."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        bob = register(server, 2, nick="bob")
        send(server, alice, "JOIN #general")
        send(server, bob, "JOIN #general")
        send(server, alice, "KICK #general bob")
        # alice parts.
        send(server, alice, "PART #general")

        responses = send(server, alice, "LIST")
        list_rows = find(responses, "322")
        assert list_rows == []

    def test_nick_change_old_nick_freed(self) -> None:
        """After a nick change the old nick is removed from the index."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        send(server, alice, "NICK alicia")
        # "alice" should now be free.
        cid2 = connect(server, 2)
        responses = send(server, cid2, "NICK alice")
        assert "433" not in commands(responses)

    def test_privmsg_channel_message_has_correct_prefix(self) -> None:
        """Channel PRIVMSG is relayed with the sender's full nick!user@host mask."""
        server = make_server()
        alice = register(server, 1, nick="alice", user="auser", host="h1")
        bob = register(server, 2, nick="bob")
        send(server, alice, "JOIN #general")
        send(server, bob, "JOIN #general")
        responses = send(server, alice, "PRIVMSG #general :hi")
        priv_msgs = [
            msg for cid, msg in responses if msg.command == "PRIVMSG" and cid == bob
        ]
        assert priv_msgs
        assert priv_msgs[0].prefix == "alice!auser@h1"

    def test_whois_no_channels_no_319(self) -> None:
        """WHOIS for a user in no channels does not include 319."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        register(server, 2, nick="bob")
        responses = send(server, alice, "WHOIS bob")
        assert "319" not in commands(responses)

    def test_mode_set_and_clear(self) -> None:
        """Setting and then clearing a channel mode works correctly."""
        server = make_server()
        alice = register(server, 1, nick="alice")
        send(server, alice, "JOIN #general")
        send(server, alice, "MODE #general +n")
        # Query should show +n
        resp1 = send(server, alice, "MODE #general")
        mode_is = find(resp1, "324")
        assert "n" in mode_is[0].params[-1]
        # Clear it.
        send(server, alice, "MODE #general -n")
        resp2 = send(server, alice, "MODE #general")
        mode_is2 = find(resp2, "324")
        assert "n" not in mode_is2[0].params[-1]
