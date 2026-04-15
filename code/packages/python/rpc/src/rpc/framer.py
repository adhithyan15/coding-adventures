"""RpcFramer — the interface between a raw byte stream and discrete chunks.

Overview
--------

A *framer* solves one fundamental problem in streaming I/O:

    **How do we know where one message ends and the next begins?**

TCP (and stdin/stdout) is a stream of bytes with no inherent message
boundaries.  If we send three messages back-to-back, the reader sees one long
stream.  The framer adds (write) and strips (read) the *envelope* that marks
boundaries.

Analogy: envelopes in postal mail
-----------------------------------

Imagine sending three letters in separate envelopes.  The envelope is the
framing:

- Without envelopes, all three letters merge into one pile of paper — you
  cannot tell where the first letter ends and the second begins.
- With envelopes, each letter is a discrete, self-contained unit.

The codec's role is like the *language* the letter is written in.  The framer's
role is the *envelope*.

Common framing schemes
-----------------------

+---------------------------+-----------------------------------------------+
| Framer                    | Envelope format                               |
+---------------------------+-----------------------------------------------+
| ContentLengthFramer       | ``Content-Length: N\\r\\n\\r\\n<N bytes>``      |
| LengthPrefixFramer        | ``<4-byte big-endian N><N bytes>``            |
| NewlineFramer             | ``<message bytes>\\n``                          |
| WebSocketFramer           | WebSocket data frame header + payload         |
| PassthroughFramer         | No envelope; used when HTTP handles framing   |
+---------------------------+-----------------------------------------------+

Architecture
------------

.. code-block:: text

    Transport (stdin/stdout / TCP socket / pipe)
         │  raw byte stream
         ▼
    ┌─────────────────────────────────────────────┐
    │  RpcFramer  (Protocol)                      │
    │  read_frame()  → bytes or None (EOF)        │
    │  write_frame(bytes) → None                  │
    └──────────────────────────────────────────────┘
         │  discrete payload bytes
         ▼
    RpcCodec.decode / encode

The framer knows nothing about the *content* of the payload.  It only cares
about byte boundaries.

Implementing a framer
----------------------

Any class with ``read_frame`` and ``write_frame`` methods satisfying the
signatures below is a valid :class:`RpcFramer`.  No inheritance is required.

Example — newline framer skeleton::

    import io

    class NewlineFramer:
        def __init__(self, stream: io.RawIOBase) -> None:
            self._stream = stream

        def read_frame(self) -> bytes | None:
            line = self._stream.readline()
            if not line:
                return None   # clean EOF
            return line.rstrip(b'\\n')

        def write_frame(self, data: bytes) -> None:
            self._stream.write(data + b'\\n')
            self._stream.flush()
"""

from __future__ import annotations

from typing import Optional, Protocol


class RpcFramer(Protocol):
    """Structural protocol for RPC framer implementations.

    Any object with ``read_frame`` and ``write_frame`` methods matching these
    signatures satisfies the protocol.

    Framers are *stateful* — they hold a reference to the underlying transport
    stream (stdin/stdout, a TCP socket, a BytesIO buffer, etc.) and advance
    their internal read position with each call to ``read_frame``.

    Usage::

        framer = MyFramer(stream)
        while True:
            data = framer.read_frame()
            if data is None:
                break   # EOF — remote side closed the connection
            msg = codec.decode(data)
            ...

    Error handling
    ---------------

    If the framing envelope is malformed (e.g., a negative Content-Length, a
    truncated length-prefix), the implementation may:

    1. Raise an exception immediately, or
    2. Return ``None`` (treating framing errors as EOF).

    Option 1 is preferred — the server can then send an error response with
    ``id=None`` and keep the connection alive.
    """

    def read_frame(self) -> Optional[bytes]:
        """Read the next payload frame from the transport.

        Blocks until a complete frame is available, EOF is reached, or an
        error occurs.

        Returns:
            The raw payload bytes (the framing envelope stripped away), or
            ``None`` on a clean EOF (the remote side closed the connection).

        Raises:
            Any exception the implementation uses to signal a framing error
            (e.g., ``ValueError`` for a malformed Content-Length header,
            ``OSError`` for an I/O failure).

        Example::

            data = framer.read_frame()
            if data is None:
                break  # connection closed cleanly
            msg = codec.decode(data)
        """
        ...

    def write_frame(self, data: bytes) -> None:
        """Write a payload frame to the transport.

        Wraps *data* in the framing envelope appropriate for this framer
        and writes the result to the underlying transport.  Implementations
        SHOULD flush the transport after writing so the receiver gets the
        frame immediately (important for interactive request-response flows).

        Args:
            data: The raw payload bytes to frame and write.  These are the
                bytes returned by ``codec.encode(msg)`` — no additional
                envelope is present.

        Raises:
            ``OSError`` or similar if the transport write fails.

        Example::

            encoded = codec.encode(response)
            framer.write_frame(encoded)
        """
        ...


def check_framer(obj: object) -> Optional[str]:
    """Check whether *obj* looks like a valid :class:`RpcFramer`.

    Lightweight duck-type check: verifies that ``read_frame`` and
    ``write_frame`` are callable.

    Args:
        obj: Any Python object.

    Returns:
        ``None`` if *obj* appears to be a valid framer; a human-readable
        error string otherwise.

    Example::

        assert check_framer(my_framer) is None, "Bad framer!"
    """
    for attr in ("read_frame", "write_frame"):
        if not callable(getattr(obj, attr, None)):
            return f"framer missing callable attribute '{attr}'"
    return None
