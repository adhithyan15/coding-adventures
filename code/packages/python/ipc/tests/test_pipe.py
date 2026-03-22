"""Tests for Pipe -- the circular-buffer byte stream.

These tests verify:
1. Basic write/read cycle
2. Circular buffer wrapping (data wraps around the end of the array)
3. Partial reads (read fewer bytes than available)
4. EOF detection when all writers close
5. BrokenPipeError when all readers close
6. Capacity limits (full buffer)
7. Empty pipe behavior
8. Properties (is_empty, is_full, available, space, is_eof, capacity)
"""

import pytest

from ipc import Pipe

# ========================================================================
# Basic write/read
# ========================================================================


class TestPipeBasicIO:
    """Write bytes in, read them out -- the fundamental pipe operation."""

    def test_write_and_read(self) -> None:
        """Write 'hello', read 5 bytes, get 'hello' back."""
        pipe = Pipe(capacity=64)
        written = pipe.write(b"hello")
        assert written == 5
        data = pipe.read(5)
        assert data == b"hello"

    def test_write_returns_byte_count(self) -> None:
        """write() returns the number of bytes actually written."""
        pipe = Pipe(capacity=64)
        assert pipe.write(b"abc") == 3

    def test_read_returns_bytes_object(self) -> None:
        """read() returns a bytes object, not bytearray."""
        pipe = Pipe(capacity=64)
        pipe.write(b"test")
        result = pipe.read(4)
        assert isinstance(result, bytes)

    def test_fifo_ordering(self) -> None:
        """Data comes out in the same order it went in (FIFO).

        Write 'abc' then 'def', read 6 bytes => 'abcdef'.
        """
        pipe = Pipe(capacity=64)
        pipe.write(b"abc")
        pipe.write(b"def")
        data = pipe.read(6)
        assert data == b"abcdef"

    def test_multiple_reads(self) -> None:
        """Multiple small reads drain the buffer incrementally."""
        pipe = Pipe(capacity=64)
        pipe.write(b"hello world")
        assert pipe.read(5) == b"hello"
        assert pipe.read(1) == b" "
        assert pipe.read(5) == b"world"


# ========================================================================
# Partial reads and writes
# ========================================================================


class TestPipePartialIO:
    """Test behavior when reads/writes don't consume/fill everything."""

    def test_partial_read(self) -> None:
        """Read fewer bytes than available."""
        pipe = Pipe(capacity=64)
        pipe.write(b"hello world")
        data = pipe.read(5)
        assert data == b"hello"
        assert pipe.available == 6  # " world" remains

    def test_read_more_than_available(self) -> None:
        """Reading more bytes than available returns only what's there."""
        pipe = Pipe(capacity=64)
        pipe.write(b"hi")
        data = pipe.read(100)
        assert data == b"hi"

    def test_write_more_than_space(self) -> None:
        """Writing more bytes than space writes only what fits."""
        pipe = Pipe(capacity=8)
        written = pipe.write(b"0123456789")  # 10 bytes, only 8 fit
        assert written == 8
        assert pipe.is_full

    def test_read_empty_pipe(self) -> None:
        """Reading from an empty pipe returns empty bytes."""
        pipe = Pipe(capacity=64)
        data = pipe.read(10)
        assert data == b""


# ========================================================================
# Circular buffer wrapping
# ========================================================================


class TestPipeCircularWrap:
    """Verify the circular buffer correctly wraps around the array boundary.

    This is the trickiest part of the pipe implementation. We use a small
    buffer (capacity=8) so we can force wrapping with small writes.
    """

    def test_wrap_around(self) -> None:
        """Write, read, then write again to force wrap-around.

        Buffer state (capacity=8):
          1. Write 'abcde' (5 bytes): [a b c d e . . .]
          2. Read 3 bytes ('abc'):    [. . . d e . . .]
          3. Write 'fghij' (5 bytes): [i j . d e f g h]  -- wraps!
          4. Read 7 bytes:            should get 'defghij'
        """
        pipe = Pipe(capacity=8)

        pipe.write(b"abcde")
        assert pipe.read(3) == b"abc"

        # Now write_pos=5, read_pos=3, count=2.
        # Writing 5 more bytes wraps: positions 5,6,7,0,1.
        written = pipe.write(b"fghij")
        assert written == 5
        assert pipe.available == 7

        data = pipe.read(7)
        assert data == b"defghij"

    def test_fill_and_drain_repeatedly(self) -> None:
        """Fill the buffer, drain it, and repeat -- exercises wrap-around."""
        pipe = Pipe(capacity=4)

        for _ in range(10):  # 10 full cycles
            pipe.write(b"abcd")
            assert pipe.is_full
            data = pipe.read(4)
            assert data == b"abcd"
            assert pipe.is_empty


# ========================================================================
# EOF and BrokenPipe
# ========================================================================


class TestPipeEOFAndBrokenPipe:
    """Test the two terminal conditions of a pipe."""

    def test_eof_when_writers_close(self) -> None:
        """After all writers close and buffer drains, is_eof is True.

        This is how 'cat file | grep hello' terminates: cat closes its
        write end, grep reads remaining data, then sees EOF.
        """
        pipe = Pipe(capacity=64)
        pipe.write(b"last data")
        pipe.close_write()

        # Data still available -- not EOF yet
        assert not pipe.is_eof
        data = pipe.read(9)
        assert data == b"last data"

        # Now buffer is empty AND no writers => EOF
        assert pipe.is_eof

    def test_eof_empty_pipe_no_writers(self) -> None:
        """An empty pipe with no writers is immediately at EOF."""
        pipe = Pipe(capacity=64)
        pipe.close_write()
        assert pipe.is_eof

    def test_broken_pipe_error(self) -> None:
        """Writing to a pipe with no readers raises BrokenPipeError.

        This is EPIPE in Unix. If nobody will ever read the data,
        there is no point in writing it.
        """
        pipe = Pipe(capacity=64)
        pipe.close_read()
        with pytest.raises(BrokenPipeError):
            pipe.write(b"nobody home")

    def test_write_after_close_write(self) -> None:
        """Writing after close_write raises BrokenPipeError."""
        pipe = Pipe(capacity=64)
        pipe.close_write()
        with pytest.raises(BrokenPipeError):
            pipe.write(b"too late")

    def test_read_after_close_read(self) -> None:
        """Reading after close_read returns empty bytes."""
        pipe = Pipe(capacity=64)
        pipe.write(b"data")
        pipe.close_read()
        assert pipe.read(4) == b""


# ========================================================================
# Properties
# ========================================================================


class TestPipeProperties:
    """Test the state-query properties."""

    def test_is_empty(self) -> None:
        pipe = Pipe(capacity=64)
        assert pipe.is_empty
        pipe.write(b"x")
        assert not pipe.is_empty

    def test_is_full(self) -> None:
        pipe = Pipe(capacity=4)
        assert not pipe.is_full
        pipe.write(b"abcd")
        assert pipe.is_full

    def test_available(self) -> None:
        pipe = Pipe(capacity=64)
        assert pipe.available == 0
        pipe.write(b"hello")
        assert pipe.available == 5
        pipe.read(2)
        assert pipe.available == 3

    def test_space(self) -> None:
        pipe = Pipe(capacity=8)
        assert pipe.space == 8
        pipe.write(b"abc")
        assert pipe.space == 5

    def test_capacity(self) -> None:
        pipe = Pipe(capacity=1024)
        assert pipe.capacity == 1024

    def test_default_capacity(self) -> None:
        pipe = Pipe()
        assert pipe.capacity == 4096

    def test_is_eof_with_active_writers(self) -> None:
        """An empty pipe with active writers is NOT at EOF."""
        pipe = Pipe(capacity=64)
        assert not pipe.is_eof  # writers still open, just no data yet


# ========================================================================
# Edge cases
# ========================================================================


class TestPipeEdgeCases:
    """Edge cases and boundary conditions."""

    def test_write_zero_bytes(self) -> None:
        """Writing an empty bytes object writes nothing."""
        pipe = Pipe(capacity=64)
        assert pipe.write(b"") == 0

    def test_read_zero_bytes(self) -> None:
        """Reading zero bytes returns empty bytes."""
        pipe = Pipe(capacity=64)
        pipe.write(b"data")
        assert pipe.read(0) == b""

    def test_capacity_one(self) -> None:
        """A pipe with capacity 1 -- the smallest possible buffer."""
        pipe = Pipe(capacity=1)
        assert pipe.write(b"a") == 1
        assert pipe.is_full
        assert pipe.write(b"b") == 0  # no space
        assert pipe.read(1) == b"a"
        assert pipe.is_empty

    def test_write_full_pipe_returns_zero(self) -> None:
        """Writing to a full pipe returns 0 (would block in real OS)."""
        pipe = Pipe(capacity=4)
        pipe.write(b"abcd")
        assert pipe.write(b"x") == 0
