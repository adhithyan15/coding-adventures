"""Tests for irc-proto — RFC 1459 message parsing and serialization.

These tests are organized to mirror the public API surface:
  - Parsing happy-path cases (valid lines → correct Message values)
  - Parsing edge cases and error paths
  - Serialization tests (Message values → correct wire bytes)
  - Round-trip tests (parse → serialize → parse yields the same Message)

Coverage target: 95%+.  Every branch in parse() and serialize() is exercised.
"""

from __future__ import annotations

import pytest

from irc_proto import (
    Message,
    ParseError,
    __version__,
    parse,
    serialize,
)

# ---------------------------------------------------------------------------
# Sanity
# ---------------------------------------------------------------------------

class TestVersion:
    """The package is importable and has the expected version string."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.1.0"


# ---------------------------------------------------------------------------
# Parsing — happy path
# ---------------------------------------------------------------------------

class TestParseHappyPath:
    """Each test exercises one well-formed IRC line from the spec examples."""

    def test_nick_no_prefix(self) -> None:
        # Client sends NICK with no prefix — simplest possible message.
        msg = parse("NICK alice")
        assert msg == Message(prefix=None, command="NICK", params=["alice"])

    def test_user_with_trailing(self) -> None:
        # USER has a colon-prefixed trailing param that includes a space.
        msg = parse("USER alice 0 * :Alice Smith")
        assert msg == Message(
            prefix=None,
            command="USER",
            params=["alice", "0", "*", "Alice Smith"],
        )

    def test_join_no_prefix(self) -> None:
        msg = parse("JOIN #general")
        assert msg == Message(prefix=None, command="JOIN", params=["#general"])

    def test_privmsg_with_trailing(self) -> None:
        # The trailing "Hello everyone!" becomes a single param without the colon.
        msg = parse("PRIVMSG #general :Hello everyone!")
        assert msg == Message(
            prefix=None,
            command="PRIVMSG",
            params=["#general", "Hello everyone!"],
        )

    def test_server_welcome(self) -> None:
        # Server-originated 001 numeric with a server-name prefix.
        msg = parse(":irc.local 001 alice :Welcome!")
        assert msg == Message(
            prefix="irc.local",
            command="001",
            params=["alice", "Welcome!"],
        )

    def test_nick_mask_prefix_privmsg(self) -> None:
        # Full nick!user@host mask as prefix.
        msg = parse(":alice!alice@host PRIVMSG #general :hello world")
        assert msg == Message(
            prefix="alice!alice@host",
            command="PRIVMSG",
            params=["#general", "hello world"],
        )

    def test_ping_no_params(self) -> None:
        # PING with no arguments at all.
        msg = parse("PING")
        assert msg == Message(prefix=None, command="PING", params=[])

    def test_quit_trailing(self) -> None:
        msg = parse("QUIT :Goodbye")
        assert msg == Message(prefix=None, command="QUIT", params=["Goodbye"])

    def test_ping_with_trailing(self) -> None:
        msg = parse("PING :server1")
        assert msg == Message(prefix=None, command="PING", params=["server1"])


# ---------------------------------------------------------------------------
# Parsing — prefix variants
# ---------------------------------------------------------------------------

class TestParsePrefix:
    """Verify that all three prefix forms are handled correctly."""

    def test_no_prefix(self) -> None:
        # A line without a leading colon has no prefix.
        msg = parse("NICK alice")
        assert msg.prefix is None

    def test_server_name_prefix(self) -> None:
        # irc.local — a plain dotted hostname as prefix.
        msg = parse(":irc.local 001 alice :Welcome")
        assert msg.prefix == "irc.local"

    def test_full_nick_mask_prefix(self) -> None:
        # nick!user@host — all three components present.
        msg = parse(":alice!alice@127.0.0.1 QUIT :bye")
        assert msg.prefix == "alice!alice@127.0.0.1"
        assert msg.command == "QUIT"
        assert msg.params == ["bye"]

    def test_nick_only_prefix(self) -> None:
        # Some servers omit the user/host portion of the mask.
        msg = parse(":alice JOIN #foo")
        assert msg.prefix == "alice"

    def test_server_prefix_with_no_params(self) -> None:
        msg = parse(":server PING")
        assert msg.prefix == "server"
        assert msg.command == "PING"
        assert msg.params == []


# ---------------------------------------------------------------------------
# Parsing — command handling
# ---------------------------------------------------------------------------

class TestParseCommand:
    """Command tokens are normalised and numeric commands are preserved."""

    def test_command_uppercased(self) -> None:
        # "join" (lowercase) → "JOIN" (uppercase).
        msg = parse("join #foo")
        assert msg.command == "JOIN"

    def test_command_mixed_case(self) -> None:
        msg = parse("PrIvMsG #c :hi")
        assert msg.command == "PRIVMSG"

    def test_numeric_command_001(self) -> None:
        # Three-digit numerics are stored as strings, not integers.
        msg = parse(":srv 001 alice :Welcome")
        assert msg.command == "001"

    def test_numeric_command_433(self) -> None:
        msg = parse(":srv 433 * nick :Nick in use")
        assert msg.command == "433"
        assert msg.params == ["*", "nick", "Nick in use"]

    def test_numeric_command_353(self) -> None:
        msg = parse(":irc.local 353 alice = #general :alice bob carol")
        assert msg.command == "353"
        assert msg.params == ["alice", "=", "#general", "alice bob carol"]


# ---------------------------------------------------------------------------
# Parsing — parameter handling
# ---------------------------------------------------------------------------

class TestParseParams:
    """Param extraction, trailing param detection, and the 15-param cap."""

    def test_single_param(self) -> None:
        msg = parse("NICK alice")
        assert msg.params == ["alice"]

    def test_multiple_params_no_trailing(self) -> None:
        # MODE with three regular params — none starts with ":".
        msg = parse("MODE #foo +o alice")
        assert msg.params == ["#foo", "+o", "alice"]

    def test_trailing_with_spaces(self) -> None:
        # The trailing param absorbs spaces verbatim.
        msg = parse("PRIVMSG #c :hello world")
        assert msg.params == ["#c", "hello world"]

    def test_empty_trailing(self) -> None:
        # A lone ":" with nothing after it is a valid empty trailing param.
        msg = parse("PRIVMSG #c :")
        assert msg.params == ["#c", ""]

    def test_no_params(self) -> None:
        msg = parse("QUIT")
        assert msg.params == []

    def test_trailing_only(self) -> None:
        # Some commands have only a trailing param (e.g., PING :server).
        msg = parse("PING :irc.local")
        assert msg.params == ["irc.local"]

    def test_15_param_cap_keeps_15(self) -> None:
        # Build a line with exactly 15 regular params — all should be kept.
        params = " ".join(str(i) for i in range(15))
        msg = parse(f"CMD {params}")
        assert len(msg.params) == 15
        assert msg.params == [str(i) for i in range(15)]

    def test_16_params_drops_16th(self) -> None:
        # Build a line with 16 params — the 16th must be silently dropped.
        params = " ".join(str(i) for i in range(16))
        msg = parse(f"CMD {params}")
        assert len(msg.params) == 15
        assert msg.params == [str(i) for i in range(15)]

    def test_16_params_with_trailing_drops_trailing(self) -> None:
        # 15 regular params followed by a trailing param: trailing is dropped.
        regular = " ".join(str(i) for i in range(15))
        line = f"CMD {regular} :this should be dropped"
        msg = parse(line)
        assert len(msg.params) == 15

    def test_trailing_colon_in_middle_of_params(self) -> None:
        # A colon at the *start* of a token, mid-params, still triggers trailing.
        msg = parse("CMD a b :rest of it")
        assert msg.params == ["a", "b", "rest of it"]


# ---------------------------------------------------------------------------
# Parsing — error paths
# ---------------------------------------------------------------------------

class TestParseErrors:
    """Lines that cannot be parsed must raise ParseError."""

    def test_empty_line_raises(self) -> None:
        with pytest.raises(ParseError):
            parse("")

    def test_whitespace_only_raises(self) -> None:
        with pytest.raises(ParseError):
            parse("   ")

    def test_single_space_raises(self) -> None:
        with pytest.raises(ParseError):
            parse(" ")

    def test_prefix_only_no_command_raises(self) -> None:
        # A line with only a prefix and nothing after is malformed.
        with pytest.raises(ParseError):
            parse(":irc.local")


# ---------------------------------------------------------------------------
# Serialization
# ---------------------------------------------------------------------------

class TestSerialize:
    """Every serialize() call produces well-formed IRC wire bytes."""

    def test_crlf_terminated(self) -> None:
        # The wire format always ends with CRLF (0x0D 0x0A), not just LF.
        result = serialize(Message(None, "PING", []))
        assert result.endswith(b"\r\n")

    def test_no_prefix_no_params(self) -> None:
        result = serialize(Message(None, "PING", []))
        assert result == b"PING\r\n"

    def test_no_prefix_with_params(self) -> None:
        result = serialize(Message(None, "NICK", ["alice"]))
        assert result == b"NICK alice\r\n"

    def test_prefix_with_params(self) -> None:
        # The prefix is wrapped in ":" and separated from the command by a space.
        # "Welcome!" contains no space so it is emitted as a plain param (no colon).
        result = serialize(Message("irc.local", "001", ["alice", "Welcome!"]))
        assert result == b":irc.local 001 alice Welcome!\r\n"

    def test_prefix_with_trailing_param(self) -> None:
        # When the last param contains a space it must get a leading colon.
        result = serialize(Message("irc.local", "001", ["alice", "Welcome to IRC!"]))
        assert result == b":irc.local 001 alice :Welcome to IRC!\r\n"

    def test_trailing_param_gets_colon(self) -> None:
        # A param containing a space must be serialized as a trailing param.
        result = serialize(Message(None, "PRIVMSG", ["#chan", "hello world"]))
        assert result == b"PRIVMSG #chan :hello world\r\n"

    def test_empty_trailing_param(self) -> None:
        # An empty string as last param — must get the colon only if it's last
        # AND... it has no space, so it is written without colon.
        # Per spec: the colon is only required when the param contains a space.
        result = serialize(Message(None, "PRIVMSG", ["#c", ""]))
        # "" has no space, so it is emitted as-is.
        assert result == b"PRIVMSG #c \r\n"

    def test_multiple_params_no_trailing(self) -> None:
        result = serialize(Message(None, "MODE", ["#foo", "+o", "alice"]))
        assert result == b"MODE #foo +o alice\r\n"

    def test_nick_mask_prefix(self) -> None:
        result = serialize(
            Message("alice!alice@host", "PRIVMSG", ["#general", "hi"])
        )
        assert result == b":alice!alice@host PRIVMSG #general hi\r\n"

    def test_server_prefix_no_params(self) -> None:
        result = serialize(Message("server", "PING", []))
        assert result == b":server PING\r\n"

    def test_returns_bytes(self) -> None:
        # serialize() must always return bytes, never str.
        result = serialize(Message(None, "NICK", ["alice"]))
        assert isinstance(result, bytes)

    def test_non_ascii_trailing(self) -> None:
        # UTF-8 is widely accepted in IRC; the encoder must handle it.
        result = serialize(Message(None, "PRIVMSG", ["#ch", "Héllo"]))
        assert b"H\xc3\xa9llo" in result


# ---------------------------------------------------------------------------
# Round-trip tests
# ---------------------------------------------------------------------------

class TestRoundTrip:
    """parse → serialize → parse must yield an equal Message for well-formed input."""

    def _roundtrip(self, line: str) -> Message:
        """Parse a line, serialize it, strip CRLF, then parse again."""
        msg1 = parse(line)
        wire = serialize(msg1)
        line2 = wire.decode("utf-8").rstrip("\r\n")
        return parse(line2)

    def test_roundtrip_nick(self) -> None:
        msg = self._roundtrip("NICK alice")
        assert msg == Message(None, "NICK", ["alice"])

    def test_roundtrip_privmsg_trailing(self) -> None:
        msg = self._roundtrip("PRIVMSG #general :Hello everyone!")
        assert msg == Message(None, "PRIVMSG", ["#general", "Hello everyone!"])

    def test_roundtrip_server_welcome(self) -> None:
        msg = self._roundtrip(":irc.local 001 alice :Welcome!")
        assert msg == Message("irc.local", "001", ["alice", "Welcome!"])

    def test_roundtrip_nick_mask_privmsg(self) -> None:
        msg = self._roundtrip(":alice!alice@host PRIVMSG #general :hello world")
        assert msg == Message(
            "alice!alice@host", "PRIVMSG", ["#general", "hello world"]
        )

    def test_roundtrip_mode(self) -> None:
        msg = self._roundtrip("MODE #foo +o alice")
        assert msg == Message(None, "MODE", ["#foo", "+o", "alice"])

    def test_roundtrip_quit(self) -> None:
        msg = self._roundtrip("QUIT :Goodbye cruel world")
        assert msg == Message(None, "QUIT", ["Goodbye cruel world"])

    def test_roundtrip_numeric(self) -> None:
        msg = self._roundtrip(":srv 433 * nick :Nick in use")
        assert msg == Message("srv", "433", ["*", "nick", "Nick in use"])

    def test_roundtrip_ping(self) -> None:
        msg = self._roundtrip("PING")
        assert msg == Message(None, "PING", [])

    def test_roundtrip_join(self) -> None:
        msg = self._roundtrip("JOIN #general")
        assert msg == Message(None, "JOIN", ["#general"])
