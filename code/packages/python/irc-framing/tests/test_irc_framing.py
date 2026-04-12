"""Tests for the irc-framing package.

Every test exercises one specific behaviour of the Framer class, named so
that a failure message alone tells you what broke.  The test groupings
mirror the spec's test-strategy sections:

1. Core frame extraction
2. Maximum line-length enforcement (RFC 1459 §2.3)
3. LF-only client leniency
4. Reset behaviour
5. buffer_size property
6. Version sanity-check
"""

from __future__ import annotations

from collections.abc import Iterator

import pytest

from irc_framing import Framer, __version__

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def drain(framer: Framer) -> list[bytes]:
    """Collect all frames currently available from *framer* into a list."""
    return list(framer.frames())


# ---------------------------------------------------------------------------
# 1. Core frame extraction
# ---------------------------------------------------------------------------


class TestSingleFrame:
    """A single CRLF-terminated message in one feed."""

    def test_yields_stripped_line(self) -> None:
        """feed(b"NICK alice\\r\\n") → frames() yields b"NICK alice"."""
        f = Framer()
        f.feed(b"NICK alice\r\n")
        assert drain(f) == [b"NICK alice"]

    def test_yields_nothing_before_crlf(self) -> None:
        """feed(b"NICK alice") with no terminator → frames() yields nothing."""
        f = Framer()
        f.feed(b"NICK alice")
        assert drain(f) == []

    def test_empty_feed_yields_nothing(self) -> None:
        """feed(b"") is a harmless no-op."""
        f = Framer()
        f.feed(b"")
        assert drain(f) == []


class TestMultipleFramesInOneFeed:
    """More than one complete frame arrives in a single feed call."""

    def test_two_frames(self) -> None:
        """b"NICK a\\r\\nUSER a 0 * :A\\r\\n" → both lines yielded in order."""
        f = Framer()
        f.feed(b"NICK a\r\nUSER a 0 * :A\r\n")
        assert drain(f) == [b"NICK a", b"USER a 0 * :A"]

    def test_three_frames(self) -> None:
        """Three back-to-back messages → three frames in order."""
        f = Framer()
        f.feed(b"PING :server\r\nNICK bob\r\nJOIN #test\r\n")
        assert drain(f) == [b"PING :server", b"NICK bob", b"JOIN #test"]

    def test_frames_then_partial(self) -> None:
        """b"A\\r\\nB\\r\\nC" → yields A and B; C is held in the buffer."""
        f = Framer()
        f.feed(b"A\r\nB\r\nC")
        frames = drain(f)
        assert frames == [b"A", b"B"]
        # C is still pending — it should emerge once its terminator arrives.
        f.feed(b"\r\n")
        assert drain(f) == [b"C"]


class TestSplitAcrossFeeds:
    """A single message arrives in two or more separate feed calls."""

    def test_split_message_body(self) -> None:
        """feed(b"NICK al") then feed(b"ice\\r\\n") → b"NICK alice"."""
        f = Framer()
        f.feed(b"NICK al")
        assert drain(f) == []  # no complete frame yet
        f.feed(b"ice\r\n")
        assert drain(f) == [b"NICK alice"]

    def test_split_crlf_across_two_feeds(self) -> None:
        """CR in one feed, LF in the next — the pair must still be recognised."""
        f = Framer()
        f.feed(b"NICK alice\r")
        assert drain(f) == []  # CR alone is not a frame boundary
        f.feed(b"\n")
        assert drain(f) == [b"NICK alice"]

    def test_three_feeds_one_frame(self) -> None:
        """Three tiny feeds accumulate into one complete frame."""
        f = Framer()
        f.feed(b"NI")
        f.feed(b"CK")
        f.feed(b" a\r\n")
        assert drain(f) == [b"NICK a"]

    def test_two_feeds_second_feed_has_two_frames(self) -> None:
        """Partial in first feed; second feed completes it and adds another."""
        f = Framer()
        f.feed(b"NICK al")
        f.feed(b"ice\r\nUSER alice 0 * :Alice\r\n")
        assert drain(f) == [b"NICK alice", b"USER alice 0 * :Alice"]


class TestEmptyLine:
    """A line that contains only the CRLF terminator."""

    def test_empty_line_yields_empty_bytes(self) -> None:
        """\\r\\n by itself yields b"" — a zero-length frame."""
        f = Framer()
        f.feed(b"\r\n")
        assert drain(f) == [b""]

    def test_multiple_empty_lines(self) -> None:
        """Several bare CRLF pairs each yield one empty frame."""
        f = Framer()
        f.feed(b"\r\n\r\n\r\n")
        assert drain(f) == [b"", b"", b""]


# ---------------------------------------------------------------------------
# 2. Maximum line-length enforcement (RFC 1459 §2.3)
# ---------------------------------------------------------------------------


class TestMaxLineLength:
    """Lines longer than 510 bytes of content must be discarded."""

    def _make_line(self, n_content_bytes: int) -> bytes:
        """Return a CRLF-terminated line with exactly *n_content_bytes* of b"A"."""
        return b"A" * n_content_bytes + b"\r\n"

    def test_exactly_510_bytes_is_yielded(self) -> None:
        """Content of exactly 510 bytes is at the limit and must be yielded."""
        f = Framer()
        f.feed(self._make_line(510))
        frames = drain(f)
        assert len(frames) == 1
        assert len(frames[0]) == 510

    def test_511_bytes_is_discarded(self) -> None:
        """511 bytes of content exceeds the 510-byte limit; must be discarded."""
        f = Framer()
        f.feed(self._make_line(511))
        assert drain(f) == []

    def test_512_bytes_is_discarded(self) -> None:
        """512 bytes of content (no room for CRLF) is also discarded."""
        f = Framer()
        f.feed(self._make_line(512))
        assert drain(f) == []

    def test_overlong_line_does_not_block_subsequent_valid_lines(self) -> None:
        """Discarding an overlong frame must not prevent subsequent valid frames."""
        f = Framer()
        # An overlong line followed immediately by two short valid lines.
        overlong = b"X" * 511 + b"\r\n"
        f.feed(overlong + b"NICK alice\r\n" + b"USER bob\r\n")
        assert drain(f) == [b"NICK alice", b"USER bob"]

    def test_two_short_frames_after_overlong(self) -> None:
        """Regression: overlong discard must not consume part of the next frame."""
        f = Framer()
        f.feed(b"B" * 511 + b"\r\nPING :x\r\nJOIN #y\r\n")
        assert drain(f) == [b"PING :x", b"JOIN #y"]


# ---------------------------------------------------------------------------
# 3. LF-only client leniency
# ---------------------------------------------------------------------------


class TestLfOnlyClients:
    """Accept bare LF as a frame terminator (no preceding CR)."""

    def test_lf_only_yields_frame(self) -> None:
        """feed(b"NICK alice\\n") → b"NICK alice" (LF stripped)."""
        f = Framer()
        f.feed(b"NICK alice\n")
        assert drain(f) == [b"NICK alice"]

    def test_crlf_strips_both_characters(self) -> None:
        """CRLF-terminated line must not include a trailing \\r in the yielded bytes."""
        f = Framer()
        f.feed(b"NICK alice\r\n")
        result = drain(f)
        assert result == [b"NICK alice"]
        # Explicitly verify no CR remains at the end.
        assert not result[0].endswith(b"\r")

    def test_lf_only_multiple_frames(self) -> None:
        """Multiple LF-only frames in one feed all yielded correctly."""
        f = Framer()
        f.feed(b"NICK a\nUSER b\n")
        assert drain(f) == [b"NICK a", b"USER b"]

    def test_lone_cr_is_not_a_frame_boundary(self) -> None:
        """A lone \\r without a following \\n does not trigger a frame."""
        f = Framer()
        f.feed(b"NICK alice\r")
        assert drain(f) == []
        # Completing with LF should now yield the frame.
        f.feed(b"\n")
        assert drain(f) == [b"NICK alice"]

    def test_mixed_crlf_and_lf_in_one_feed(self) -> None:
        """Buffer may contain both CRLF and LF-only lines."""
        f = Framer()
        f.feed(b"NICK a\r\nUSER b\n")
        assert drain(f) == [b"NICK a", b"USER b"]


# ---------------------------------------------------------------------------
# 4. Reset behaviour
# ---------------------------------------------------------------------------


class TestReset:
    """reset() discards all buffered data."""

    def test_reset_clears_partial_data(self) -> None:
        """Partial data accumulated before reset() must not appear after reset."""
        f = Framer()
        f.feed(b"NICK al")
        f.reset()
        f.feed(b"USER a\r\n")
        assert drain(f) == [b"USER a"]

    def test_reset_on_empty_buffer_is_safe(self) -> None:
        """reset() on an already-empty framer must not raise."""
        f = Framer()
        f.reset()  # should not raise
        f.feed(b"PING :x\r\n")
        assert drain(f) == [b"PING :x"]

    def test_reset_between_complete_frames(self) -> None:
        """reset() after successfully extracting one frame discards partial data."""
        f = Framer()
        f.feed(b"NICK a\r\nUSER b")  # two — one complete, one partial
        assert drain(f) == [b"NICK a"]
        f.reset()
        f.feed(b"JOIN #c\r\n")
        assert drain(f) == [b"JOIN #c"]

    def test_reset_then_multiple_frames(self) -> None:
        """After a reset, the framer works normally for subsequent feeds."""
        f = Framer()
        f.feed(b"GARBAGE")
        f.reset()
        f.feed(b"NICK x\r\nUSER y\r\n")
        assert drain(f) == [b"NICK x", b"USER y"]


# ---------------------------------------------------------------------------
# 5. buffer_size property
# ---------------------------------------------------------------------------


class TestBufferSize:
    """buffer_size reflects the number of bytes currently held."""

    def test_buffer_size_zero_initially(self) -> None:
        """A freshly constructed Framer has an empty buffer."""
        f = Framer()
        assert f.buffer_size == 0

    def test_buffer_size_increases_after_feed(self) -> None:
        """After feed(b"hello"), buffer_size is 5."""
        f = Framer()
        f.feed(b"hello")
        assert f.buffer_size == 5

    def test_buffer_size_decreases_after_frames_extracted(self) -> None:
        """Extracting frames removes their bytes from the buffer."""
        f = Framer()
        f.feed(b"NICK alice\r\n")  # 12 bytes
        assert f.buffer_size == 12
        drain(f)
        # Buffer must be empty after all frames have been extracted.
        assert f.buffer_size == 0

    def test_buffer_size_holds_partial_data(self) -> None:
        """Bytes not yet terminated by CRLF remain counted in buffer_size."""
        f = Framer()
        # Feed two complete frames plus three partial bytes.
        f.feed(b"A\r\nB\r\nCDE")
        drain(f)  # extracts A and B
        assert f.buffer_size == 3  # only "CDE" remains

    def test_buffer_size_zero_after_reset(self) -> None:
        """After reset(), buffer_size is 0 regardless of prior state."""
        f = Framer()
        f.feed(b"partial data with no terminator")
        assert f.buffer_size > 0
        f.reset()
        assert f.buffer_size == 0

    def test_buffer_size_after_empty_feed(self) -> None:
        """feed(b"") does not change buffer_size."""
        f = Framer()
        f.feed(b"abc")
        f.feed(b"")
        assert f.buffer_size == 3


# ---------------------------------------------------------------------------
# 6. frames() is a generator
# ---------------------------------------------------------------------------


class TestFramesIsGenerator:
    """frames() must return an iterator, not a list."""

    def test_frames_returns_iterator(self) -> None:
        """frames() must return an Iterator, not a concrete list."""
        f = Framer()
        f.feed(b"NICK x\r\n")
        result = f.frames()
        # Iterator protocol: must have __iter__ and __next__
        assert hasattr(result, "__iter__")
        assert hasattr(result, "__next__")

    def test_frames_can_be_iterated_incrementally(self) -> None:
        """Consuming frames one at a time via next() works correctly."""
        f = Framer()
        f.feed(b"A\r\nB\r\n")
        gen: Iterator[bytes] = f.frames()
        assert next(gen) == b"A"
        assert next(gen) == b"B"
        with pytest.raises(StopIteration):
            next(gen)

    def test_frames_empty_when_no_complete_line(self) -> None:
        """frames() raises StopIteration immediately when nothing is ready."""
        f = Framer()
        f.feed(b"partial")
        gen = f.frames()
        with pytest.raises(StopIteration):
            next(gen)


# ---------------------------------------------------------------------------
# 7. Version
# ---------------------------------------------------------------------------


class TestVersion:
    """Verify the package is importable and has the expected version string."""

    def test_version_exists(self) -> None:
        """__version__ must be present and match pyproject.toml."""
        assert __version__ == "0.1.0"
