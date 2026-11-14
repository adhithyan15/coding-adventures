"""irc-framing — Stateful byte-stream-to-line-frame converter.

The Problem: TCP delivers a byte stream, not messages
=====================================================

When you call ``recv()`` on a TCP socket, the operating system hands you
however many bytes happen to be available in the kernel's receive buffer.
That may be:

- Half a message  (``b"NICK ali"``)
- Exactly one message (``b"NICK alice\\r\\n"``)
- Three messages and the start of a fourth

IRC solves this with a simple framing convention: every message ends with
``\\r\\n`` (carriage return + line feed, ASCII 13 + 10).  The framer's job is
to absorb raw byte chunks and emit complete, ``\\r\\n``-stripped lines to the
layer above.

This package is **pure**.  It touches no sockets, threads, or I/O.
It is a single stateful buffer transformer.

Layer diagram::

    irc-proto   ← receives complete \\r\\n-stripped bytes; calls parse()
         ↑
    irc-framing ← THIS PACKAGE: feed(raw_bytes) / frames() → Iterator[bytes]
         ↑
    irc-net     ← calls conn.read() and feeds raw bytes upward

RFC 1459 maximum line length
============================

RFC 1459 §2.3 states that a single IRC message MUST NOT exceed 512 bytes
**including** the trailing ``\\r\\n``.  That leaves at most 510 bytes of
content.  RFC 2812 §2.3 keeps the same 512-byte limit.  Lines that exceed this
limit are silently discarded to prevent memory exhaustion from malformed or
malicious clients.

That limit alone is not enough: it is only applied to lines that already ended
in LF.  A peer that never sends LF would grow the buffer without bound
(SEC-011, CWE-400/770).  So a line still lacking its LF after 511 bytes
(510 plus a CR) is dropped early and the framer skips input up to the next LF.

Usage example::

    framer = Framer()
    while True:
        data = conn.read(4096)
        if not data:
            break
        framer.feed(data)
        for line in framer.frames():
            msg = parse(line.decode("utf-8", errors="replace"))
            handle(msg)
"""

from __future__ import annotations

from collections.abc import Iterator

__version__ = "0.1.0"

# RFC 1459 §2.3: maximum line length is 512 bytes including CRLF.
# Content beyond 510 bytes must be discarded.
_MAX_CONTENT_BYTES: int = 510

# The most bytes we will hold for one line that has not yet seen its LF.
#
# A legal line can be 510 content bytes plus a CR whose LF has not arrived
# yet, i.e. 511 bytes.  One more byte with no LF and the line can no longer be
# legal (SEC-011), so we stop remembering it.
_MAX_PENDING_BYTES: int = _MAX_CONTENT_BYTES + 1


class Framer:
    """Stateful byte-stream-to-line-frame converter.

    Call :meth:`feed` with raw bytes from the socket.
    Call :meth:`frames` to iterate over complete CRLF-stripped lines.

    The Framer is **not thread-safe**.  Each connection should own its own
    ``Framer`` instance.

    How it works internally
    -----------------------
    The framer owns a single ``bytearray`` that accumulates incoming bytes.
    ``bytearray.extend()`` is O(1) amortized — the same cost as
    ``list.append()``.  The CRLF scan is O(n) in the frame length, which is
    bounded by 512 bytes, so it is effectively O(1) per message.

    A ring-buffer would be more memory-efficient at very high message rates
    (avoiding copies when removing bytes from the front), but for IRC traffic
    volumes a ``bytearray`` is fast enough and far simpler to reason about.
    """

    def __init__(self) -> None:
        # bytearray is a mutable sequence of bytes.  We use it instead of
        # bytes (which is immutable) so we can append to it cheaply and slice
        # out frames in-place without creating intermediate copies each time.
        self._buf: bytearray = bytearray()
        # Length of the unterminated tail of _buf (bytes after the last LF).
        # Invariant: _tail_len <= _MAX_PENDING_BYTES after every feed().
        self._tail_len: int = 0
        # True while we are skipping the rest of an overlong line: everything
        # up to and including the next LF is thrown away, unread.
        self._discarding: bool = False

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def feed(self, data: bytes) -> None:
        """Append *data* to the internal buffer.

        This should be called immediately after each socket ``recv()``.
        Passing an empty bytes object is a safe no-op.

        :param data: Raw bytes received from the network.  May be any length,
            including zero.

        Memory bound (SEC-011)
        ----------------------
        The unterminated tail (bytes after the last LF) never exceeds 511
        bytes.  When it would, the partial line is dropped and the framer
        *discards input up to the next LF* -- fail closed, the same silent
        discard that already applies to a complete overlong line, so the API
        is unchanged.  Complete lines already buffered are kept; the caller
        bounds those by calling :meth:`frames` after each read::

            data = [ ...complete lines... LF ][ unterminated tail ]
                                              ^ only this part can be flooded
        """
        # Step 1: while discarding an overlong line, skip to just past its LF.
        if self._discarding:
            pos = data.find(b"\n")
            if pos == -1:
                return  # still inside the overlong line
            self._discarding = False
            data = data[pos + 1 :]

        # Step 2: split data at its last LF.  The head (through that LF)
        # completes lines and is stored; only the tail can be flooded.
        pos = data.rfind(b"\n")
        head, tail = data[: pos + 1], data[pos + 1 :]
        if head:
            self._buf.extend(head)
            self._tail_len = 0

        # Step 3: store the tail only if the pending line still fits.  We
        # check *before* extending so a huge single feed is never copied.
        if self._tail_len + len(tail) > _MAX_PENDING_BYTES:
            del self._buf[len(self._buf) - self._tail_len :]  # drop partial line
            self._tail_len = 0
            self._discarding = True
        else:
            self._buf.extend(tail)
            self._tail_len += len(tail)

    def frames(self) -> Iterator[bytes]:
        """Yield complete lines from the buffer, with ``\\r\\n`` stripped.

        This is a **generator** — it produces values lazily.  Each call
        scans from the beginning of the internal buffer and yields every
        complete line it finds.  Partial data (no ``\\n`` yet) is left in
        the buffer until the next :meth:`feed`.

        Lines exceeding 510 bytes of content are **discarded** silently.

        Yields
        ------
        bytes
            Each yielded value is one complete IRC line with ``\\r\\n``
            (or bare ``\\n``) stripped.
        """
        # We loop as long as there is at least one newline character
        # somewhere in the buffer.  The find() call returns -1 when there
        # is no newline, which terminates the loop — meaning we hold the
        # remaining partial data for the next feed().
        while True:
            # Locate the first newline (LF) byte in the buffer.
            # IRC mandates CRLF but many clients only send LF.  We handle
            # both by scanning for LF and then peeking at the byte before
            # it to check for a preceding CR.
            lf_pos = self._buf.find(b"\n")

            if lf_pos == -1:
                # No complete line yet.  Stop iterating — the caller will
                # feed more bytes before calling frames() again.
                break

            # --- Extract the raw line (without the LF) ---
            # If there is a CR immediately before the LF we want to strip
            # that too.  We check lf_pos > 0 to avoid an index error when
            # the very first byte in the buffer is \n.
            if lf_pos > 0 and self._buf[lf_pos - 1] == ord(b"\r"):
                # CRLF terminator: the content ends one byte before the CR.
                content_end = lf_pos - 1
            else:
                # LF-only terminator: the content ends at lf_pos.
                content_end = lf_pos

            # The raw frame content (bytes before any CR/LF).
            line: bytes = bytes(self._buf[:content_end])

            # --- Advance the buffer past the consumed line + terminator ---
            # We remove everything up to and including the LF byte.
            # del buf[0:n] on a bytearray is O(n) — it shifts the remaining
            # bytes left.  This is acceptable because IRC lines are short
            # (≤ 512 bytes), so n is always small.
            del self._buf[: lf_pos + 1]

            # --- Enforce the RFC 1459 maximum line length ---
            # The RFC allows at most 512 bytes per message including CRLF,
            # leaving 510 bytes of actual content.  Lines longer than this
            # are discarded (not yielded) to prevent a client from growing
            # our buffer without bound.
            if len(line) > _MAX_CONTENT_BYTES:
                # Silently drop the overlong frame and continue scanning
                # for the next line.  A real server would disconnect the
                # offending client; the framer layer is not responsible for
                # that policy.
                continue

            yield line

        # Only the unterminated tail remains in the buffer.
        self._tail_len = len(self._buf)

    def reset(self) -> None:
        """Discard all buffered data.

        Call this when a connection is closed or restarted so that stale
        bytes from the old connection cannot bleed into a new one.
        """
        # Replacing _buf with a fresh bytearray is the clearest way to
        # express "all data is gone".  Alternatively we could do
        # ``del self._buf[:]`` (in-place clear), but creating a new object
        # makes the intent explicit and allows the old allocation to be
        # garbage-collected.
        self._buf = bytearray()
        self._tail_len = 0
        self._discarding = False

    @property
    def buffer_size(self) -> int:
        """Number of bytes currently held in the internal buffer.

        Useful for monitoring buffer growth and writing precise unit tests.
        A value of 0 means the buffer is empty (no partial data pending).
        """
        return len(self._buf)
