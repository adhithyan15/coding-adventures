"""JSON-RPC 2.0 Message Writer — typed Message → framed stream.

The writer is the inverse of the reader. Where the reader peels off the
Content-Length header and hands back raw JSON, the writer takes a typed
Message, serializes it to JSON, measures the byte length, prepends the
header, and writes the whole frame in one shot.

Wire Format (repeated from reader.py for easy reference)
----------------------------------------------------------

    Content-Length: <n>\r\n
    \r\n
    <UTF-8 JSON payload, exactly n bytes>

Why measure bytes, not characters?
------------------------------------

``Content-Length`` is a *byte* count, not a character count. For ASCII-only
JSON the two are equal, but JSON strings can contain any Unicode character.
A single emoji like 🎸 is one character but *four* bytes in UTF-8.

Always encode the JSON string to bytes first, then measure ``len(payload_bytes)``.

Flushing
---------

The writer calls ``flush()`` after every message. Without flushing, the Python
buffer may hold the bytes internally and never send them to the reader's stdin.
This is especially important in test scenarios using ``io.BytesIO`` where the
buffer is checked immediately after the write.
"""

from __future__ import annotations

import json
from typing import IO, Any

from json_rpc.message import Message, message_to_dict


class MessageWriter:
    """Writes Content-Length-framed JSON-RPC messages to a binary stream.

    Each call to :meth:`write_message` produces exactly one framed message
    on the underlying stream.

    Example usage::

        import sys
        writer = MessageWriter(sys.stdout.buffer)
        writer.write_message(Response(id=1, result={"ok": True}))

    Args:
        stream: A binary-mode writable stream (``IO[bytes]``). In production
            this is ``sys.stdout.buffer``; in tests it can be a ``BytesIO``.
    """

    def __init__(self, stream: IO[bytes]) -> None:
        self._stream = stream

    def write_raw(self, json_str: str) -> None:
        """Write a raw JSON string as a Content-Length-framed message.

        This low-level method measures the byte length of the JSON string,
        writes the header, and writes the payload. Use :meth:`write_message`
        if you have a typed ``Message`` object.

        Args:
            json_str: A valid JSON string to send as the message payload.
        """
        # Encode to UTF-8 bytes first so we can measure the true byte length.
        # This must happen before we write the Content-Length header!
        payload: bytes = json_str.encode("utf-8")
        content_length = len(payload)

        # Write the header block:
        #   Content-Length: <n>\r\n
        #   \r\n
        # The \r\n line endings are required by the LSP spec (HTTP convention).
        header = f"Content-Length: {content_length}\r\n\r\n"
        self._stream.write(header.encode("ascii"))

        # Write the payload bytes.
        self._stream.write(payload)

        # Flush so the bytes are visible to the reader immediately.
        # Without this, the OS or Python's buffer layer might hold the data
        # and the reader would block waiting for bytes that have been "sent".
        self._stream.flush()

    def write_message(self, msg: Message) -> None:
        """Serialize a typed Message and write it as a framed message.

        This is the primary API. It serializes the message to a compact JSON
        string (no extra whitespace) and delegates to :meth:`write_raw`.

        The JSON is compact (no trailing whitespace) to keep the payload as
        small as possible. Pretty-printing would waste bytes on the wire and
        add no value since the reader just parses the JSON anyway.

        Args:
            msg: A ``Request``, ``Response``, or ``Notification`` instance.
        """
        # Convert the typed message to a plain dict, then to a compact JSON string.
        d = message_to_dict(msg)
        json_str = json.dumps(d, separators=(",", ":"), ensure_ascii=False)
        self.write_raw(json_str)
