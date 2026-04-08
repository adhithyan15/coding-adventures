"""
RESP2 decoder: parse bytes from a TCP read buffer into typed Python values.

The framing problem
───────────────────
TCP is a byte stream, not a message stream.  When you call recv() on a
socket, you might get:
  - Less than one complete RESP message (fragment)
  - Exactly one message
  - One and a half messages (a full message plus the start of the next)
  - Many complete messages concatenated

The decoder must handle all four cases correctly.  The solution is a
read buffer that accumulates bytes until at least one complete message
can be parsed.

The low-level `decode()` function is stateless: it attempts to parse
one message from the start of a bytes object and returns a (value, consumed)
pair.  If the buffer contains a complete message, consumed > 0.  If not,
it returns (None, 0) to signal "need more data".

The `RespDecoder` class wraps this in a stateful buffer for streaming use.
"""

from __future__ import annotations

from resp_protocol.types import RespError, RespValue


class RespDecodeError(Exception):
    """
    Raised when the decoder encounters malformed RESP data.

    Unlike an incomplete message (which returns (None, 0)), a decode error
    means the bytes are syntactically invalid — the connection should be
    closed and the client should reconnect.

    Examples of invalid data:
      - An unknown type byte (not +, -, :, $, *)
      - A non-integer where a length or count is expected
      - A bulk string whose trailing \\r\\n is missing
    """


def _read_line(buffer: bytes) -> tuple[bytes | None, int]:
    """
    Find the first \\r\\n in the buffer and return what comes before it.

    This is the fundamental building block of the RESP parser: almost every
    type starts with a "header line" terminated by \\r\\n.

    Returns:
        (content, bytes_consumed)  — content does NOT include \\r\\n
        (None, 0)                  — \\r\\n not yet in buffer (need more data)

    Example:
        _read_line(b"+OK\\r\\nrest")   → (b"OK", 5)
        _read_line(b"+OK")            → (None, 0)   ← incomplete
    """
    pos = buffer.find(b"\r\n")
    if pos == -1:
        return None, 0
    # Return the content before \r\n and consume content + 2 bytes for \r\n.
    return buffer[:pos], pos + 2


def decode(buffer: bytes) -> tuple[RespValue, int]:
    """
    Attempt to decode one RESP value from the start of buffer.

    This is a pure function (no side effects, no internal state).  It is
    suitable for one-shot decoding and as the inner loop of the stateful
    `RespDecoder`.

    The parser uses recursive descent: arrays call `decode` recursively
    to parse each element, which handles nested arrays automatically.

    Return convention:
        (value, consumed)  — a complete message was found
        (None, 0)          — buffer does not yet hold a complete message
                             (caller should accumulate more bytes and retry)

    Raises:
        RespDecodeError: on syntactically malformed input (wrong type byte,
                         non-integer length, etc.)

    Inline commands (plain text without a RESP type prefix):
        If the first byte is not one of +, -, :, $, *, we attempt to parse
        the input as an inline command: a \\r\\n-terminated line of
        space-separated tokens.  Each token becomes a bulk-string element
        in an array.  This is how `telnet redis.example.com 6379` works.

        Example: b"PING\\r\\n"  →  ([b"PING"], 6)
                 b"SET foo bar\\r\\n"  →  ([b"SET", b"foo", b"bar"], 12)
    """
    if len(buffer) == 0:
        return None, 0

    type_byte = buffer[0:1]
    rest = buffer[1:]

    if type_byte == b"+":
        # --- Simple String ---
        # Wire: +<text>\r\n
        # Decode: read until \r\n, return the text as a Python str.
        line, n = _read_line(rest)
        if line is None:
            return None, 0
        return line.decode("utf-8"), 1 + n

    if type_byte == b"-":
        # --- Error ---
        # Wire: -<message>\r\n
        # Decode: read until \r\n, wrap in RespError.
        line, n = _read_line(rest)
        if line is None:
            return None, 0
        return RespError(line.decode("utf-8")), 1 + n

    if type_byte == b":":
        # --- Integer ---
        # Wire: :<number>\r\n
        # Decode: read until \r\n, parse as signed integer.
        line, n = _read_line(rest)
        if line is None:
            return None, 0
        try:
            value = int(line)
        except ValueError as exc:
            raise RespDecodeError(
                f"Invalid integer in RESP Integer type: {line!r}"
            ) from exc
        return value, 1 + n

    if type_byte == b"$":
        # --- Bulk String ---
        # Wire: $<length>\r\n<bytes>\r\n   or   $-1\r\n (null)
        #
        # The length field tells us exactly how many bytes to read, making
        # this binary-safe: the body can contain any bytes including \r\n.
        line, n = _read_line(rest)
        if line is None:
            return None, 0
        try:
            length = int(line)
        except ValueError as exc:
            raise RespDecodeError(
                f"Invalid length in RESP Bulk String: {line!r}"
            ) from exc

        if length == -1:
            # Null bulk string — key not found, etc.
            return None, 1 + n

        if length < -1:
            raise RespDecodeError(f"Invalid negative length: {length}")

        # We need: n bytes already consumed (the header \r\n) +
        #          length bytes of data +
        #          2 bytes for the trailing \r\n
        end = n + length + 2
        if len(rest) < end:
            return None, 0  # incomplete: wait for more data

        data = rest[n : n + length]
        # Verify the trailing \r\n is present (should always be true if
        # the sender is spec-compliant, but guard anyway).
        if rest[n + length : n + length + 2] != b"\r\n":
            raise RespDecodeError(
                f"Missing trailing \\r\\n after bulk string of length {length}"
            )
        return data, 1 + end

    if type_byte == b"*":
        # --- Array ---
        # Wire: *<count>\r\n<element1><element2>...   or   *-1\r\n (null)
        #
        # Each element is a full RESP value: we recurse into decode() for
        # each one, tracking the offset into the buffer as we go.
        line, n = _read_line(rest)
        if line is None:
            return None, 0
        try:
            count = int(line)
        except ValueError as exc:
            raise RespDecodeError(
                f"Invalid count in RESP Array: {line!r}"
            ) from exc

        if count == -1:
            # Null array — some commands use this to indicate absence.
            return None, 1 + n

        if count < -1:
            raise RespDecodeError(f"Invalid negative array count: {count}")

        # Parse `count` elements starting from `offset` in the original buffer.
        offset = 1 + n
        elements: list[RespValue] = []
        for _ in range(count):
            elem, consumed = decode(buffer[offset:])
            if consumed == 0:
                # An element was incomplete — the whole array is incomplete.
                return None, 0
            elements.append(elem)
            offset += consumed
        return elements, offset

    # --- Inline command (non-RESP text from a telnet session) ---
    # If the first byte is none of the five RESP type bytes, treat the
    # entire line as an inline command.  Split on spaces to get tokens.
    # This allows interactive use via: echo "PING" | nc localhost 6379
    line, n = _read_line(buffer)
    if line is None:
        return None, 0
    # Split on whitespace; each token becomes a bytes bulk-string element.
    tokens = line.split()
    if not tokens:
        # Empty line — produce an empty array
        return [], n
    return [token for token in tokens], n


def decode_all(buffer: bytes) -> tuple[list[RespValue], int]:
    """
    Decode as many complete RESP messages as possible from buffer.

    Repeatedly calls `decode` until the buffer is exhausted or an
    incomplete message is found.  The total bytes consumed is returned
    so the caller can advance their read buffer:

        messages, consumed = decode_all(buf)
        buf = buf[consumed:]   # keep the incomplete tail

    This is the correct interface for a TCP read loop:

        buf = b""
        while True:
            data = socket.recv(4096)
            buf += data
            msgs, n = decode_all(buf)
            buf = buf[n:]
            for msg in msgs:
                dispatch(msg)

    Args:
        buffer: Accumulated bytes from recv() calls.

    Returns:
        (messages, total_bytes_consumed)
        messages: list of decoded RespValue objects (may be empty)
        total_bytes_consumed: how many bytes from the start of buffer
                              were successfully parsed
    """
    messages: list[RespValue] = []
    offset = 0
    while offset < len(buffer):
        value, consumed = decode(buffer[offset:])
        if consumed == 0:
            # Incomplete message — stop and return what we have.
            break
        messages.append(value)
        offset += consumed
    return messages, offset


class RespDecoder:
    """
    Stateful incremental decoder for a streaming TCP connection.

    Maintains an internal byte buffer.  Bytes are fed in via `feed()`.
    Complete messages are retrieved via `get_message()` / `has_message()`.

    Typical usage in a coroutine/async loop:

        decoder = RespDecoder()
        while True:
            data = await reader.read(4096)
            decoder.feed(data)
            while decoder.has_message():
                msg = decoder.get_message()
                await handle(msg)

    The decoder never raises on incomplete input — it simply buffers
    bytes until a complete message is available.  It does raise
    RespDecodeError on malformed input.
    """

    def __init__(self) -> None:
        # Internal accumulation buffer.  Bytes are appended by feed() and
        # consumed as complete messages are decoded.
        self._buffer: bytes = b""

    def feed(self, data: bytes) -> None:
        """
        Append bytes to the internal buffer.

        Typically called each time data arrives from recv() on a socket.
        There is no size limit on the buffer; callers should limit how
        much data they feed to avoid unbounded memory growth.

        Args:
            data: Raw bytes received from the network.
        """
        self._buffer += data

    def has_message(self) -> bool:
        """
        Return True if the buffer contains at least one complete message.

        This is O(n) in the worst case because it attempts a decode.
        For high-performance applications, keep a flag updated by feed().
        """
        _, consumed = decode(self._buffer)
        return consumed > 0

    def get_message(self) -> RespValue:
        """
        Decode and return the next complete message from the buffer.

        After returning, the consumed bytes are removed from the buffer
        so the next call to get_message() decodes the following message.

        Returns:
            The next decoded RespValue.

        Raises:
            ValueError: If no complete message is available.
            RespDecodeError: If the bytes are malformed.
        """
        value, consumed = decode(self._buffer)
        if consumed == 0:
            raise ValueError(
                "No complete RESP message available yet. "
                "Call has_message() before get_message()."
            )
        # Advance the buffer past the bytes we just consumed.
        self._buffer = self._buffer[consumed:]
        return value

    def decode_all(self, data: bytes) -> list[RespValue]:
        """
        Convenience method: feed data then drain all complete messages.

        Equivalent to:
            decoder.feed(data)
            results = []
            while decoder.has_message():
                results.append(decoder.get_message())
            return results

        Args:
            data: New bytes to append to the buffer.

        Returns:
            List of all completely decoded messages (may be empty).
        """
        self.feed(data)
        messages, consumed = decode_all(self._buffer)
        self._buffer = self._buffer[consumed:]
        return messages
