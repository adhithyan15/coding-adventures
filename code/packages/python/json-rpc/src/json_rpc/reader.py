"""JSON-RPC 2.0 Message Reader — framed stream → typed Message.

The Problem: Byte Streams Have No Message Boundaries
------------------------------------------------------

A TCP connection (or stdin) is a continuous stream of bytes. JSON has no
self-delimiting syntax at the stream level: you cannot tell where one JSON
object ends and the next begins without fully parsing every character.

Consider two back-to-back messages on the wire::

    {"jsonrpc":"2.0","id":1,"method":"foo"}{"jsonrpc":"2.0","method":"bar"}

Where does the first message end? At the ``}`` after ``"foo"}``? But nested
objects also end with ``}``! You would need to count matching braces — which
requires a full JSON parser just to find the message boundary.

The Solution: Content-Length Framing
--------------------------------------

The LSP specification borrows HTTP's ``Content-Length`` header to pre-announce
the byte length of each message. The wire format is::

    Content-Length: <n>\r\n
    \r\n
    <UTF-8 JSON payload, exactly n bytes>

The reader first reads the header block line by line, extracts the
``Content-Length`` value, then reads exactly that many bytes. No brace
counting, no buffering ambiguity.

Example (the ``\\`` sequences represent literal bytes on the wire)::

    "Content-Length: 47\\r\\nContent-Type: application/vscode-jsonrpc; charset=utf-8\\r\\n\\r\\n"
    followed by 47 bytes of JSON.

EOF Handling
------------

If the underlying stream reaches EOF while the reader is waiting for the next
message header (i.e., between messages), it returns ``None`` — the server's
``serve()`` loop uses this as the clean shutdown signal.

If EOF occurs mid-message (after the header but before all payload bytes are
read), that is an error because the message is incomplete.
"""

from __future__ import annotations

import json
from io import RawIOBase
from typing import IO

from json_rpc.errors import INVALID_REQUEST, PARSE_ERROR
from json_rpc.message import JsonRpcError, Message, parse_message


class MessageReader:
    """Reads Content-Length-framed JSON-RPC messages from a binary stream.

    Each call to :meth:`read_message` reads exactly one message from the
    underlying stream. The reader is not thread-safe; use one reader per
    thread if concurrent reading is required (though the spec's single-
    threaded server model makes this unnecessary).

    Example usage::

        import sys
        reader = MessageReader(sys.stdin.buffer)
        while True:
            msg = reader.read_message()
            if msg is None:
                break  # EOF — client disconnected
            handle(msg)

    Args:
        stream: A binary-mode readable stream (``IO[bytes]``). In production
            this is ``sys.stdin.buffer``; in tests it can be a ``BytesIO``.
    """

    def __init__(self, stream: IO[bytes]) -> None:
        self._stream = stream

    def read_raw(self) -> str | None:
        """Read one framed message and return the raw JSON string.

        This low-level method reads the Content-Length header, then reads
        exactly that many bytes, and returns the decoded UTF-8 string.

        Returns ``None`` on EOF (clean end of stream between messages).

        Returns:
            The raw JSON payload as a string, or ``None`` on EOF.

        Raises:
            JsonRpcError: With ``PARSE_ERROR`` if the header is malformed
                (no Content-Length found, or non-integer value).
            EOFError: If the stream closes mid-message (incomplete payload).
        """
        # --- Phase 1: Read headers -------------------------------------------
        # Headers are ASCII lines terminated by \r\n. The header block ends
        # with a blank line (\r\n alone). We read lines until we see that
        # blank line, accumulating any headers we find.

        content_length: int | None = None

        while True:
            line_bytes = self._stream.readline()

            # readline() returns b'' on EOF. If we get EOF before reading any
            # header, the stream closed cleanly between messages → return None.
            if line_bytes == b"":
                if content_length is None:
                    return None  # Clean EOF between messages
                # EOF in the middle of the header block is an error.
                raise JsonRpcError(
                    PARSE_ERROR, "Parse error: unexpected EOF in header block"
                )

            # Decode the header line. The LSP spec mandates ASCII for headers.
            line = line_bytes.decode("ascii", errors="replace").rstrip("\r\n")

            # A blank line signals the end of the header block.
            if line == "":
                break

            # Parse the header field. We only care about Content-Length.
            # Other headers (like Content-Type) are valid but ignored.
            if ":" in line:
                name, _, value = line.partition(":")
                if name.strip().lower() == "content-length":
                    try:
                        content_length = int(value.strip())
                    except ValueError as exc:
                        raise JsonRpcError(
                            PARSE_ERROR,
                            f"Parse error: invalid Content-Length value: {value.strip()!r}",
                        ) from exc

        # --- Phase 2: Validate we got a Content-Length -----------------------

        if content_length is None:
            raise JsonRpcError(
                PARSE_ERROR,
                "Parse error: no Content-Length header found",
            )

        if content_length < 0:
            raise JsonRpcError(
                PARSE_ERROR,
                f"Parse error: Content-Length must be non-negative, got {content_length}",
            )

        # --- Phase 3: Read exactly content_length bytes of payload -----------
        # We must read EXACTLY this many bytes — not more, not less. Reading
        # more would consume bytes belonging to the next message.

        payload_bytes = self._stream.read(content_length)

        if len(payload_bytes) < content_length:
            raise JsonRpcError(
                PARSE_ERROR,
                f"Parse error: expected {content_length} bytes but stream ended after {len(payload_bytes)}",
            )

        # Decode as UTF-8. The LSP spec mandates UTF-8 for the JSON payload.
        try:
            return payload_bytes.decode("utf-8")
        except UnicodeDecodeError as exc:
            raise JsonRpcError(
                PARSE_ERROR, f"Parse error: payload is not valid UTF-8: {exc}"
            ) from exc

    def read_message(self) -> Message | None:
        """Read one framed message and return a typed Message.

        Calls :meth:`read_raw` to get the JSON string, then calls
        ``parse_message`` to convert it to a typed dataclass.

        Returns ``None`` on clean EOF (between messages).

        Returns:
            A ``Request``, ``Response``, or ``Notification``, or ``None`` on EOF.

        Raises:
            JsonRpcError: With ``PARSE_ERROR`` for malformed JSON or bad framing.
            JsonRpcError: With ``INVALID_REQUEST`` for valid JSON that is not a
                well-formed JSON-RPC message.
        """
        raw = self.read_raw()
        if raw is None:
            return None
        return parse_message(raw)
