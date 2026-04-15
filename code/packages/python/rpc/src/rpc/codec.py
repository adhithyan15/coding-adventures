"""RpcCodec — the interface between RPC messages and raw bytes.

Overview
--------

A codec translates between the *abstract* :class:`~rpc.message.RpcMessage`
world and the *concrete* bytes-on-the-wire world.  The RPC layer never touches
serialization details; the codec encapsulates all of them.

Think of it like a translator at a diplomatic summit:

- The diplomats (RPC server/client) speak a common "diplomatic" language:
  "I am sending a Request with id=1 for method 'add' with params {a:1, b:2}."
- The translator (codec) converts that to the actual spoken language
  (JSON, MessagePack, Protobuf) for transmission.
- The reverse translation happens on receipt.

Architecture
------------

.. code-block:: text

                    ┌─────────────────────────────────────────┐
                    │  RpcServer / RpcClient                  │
                    │  (speaks in RpcMessage objects)         │
                    └──────────────┬──────────────────────────┘
                                   │ encode / decode
                                   ▼
                    ┌─────────────────────────────────────────┐
                    │  RpcCodec  (Protocol)                   │
                    │  encode(RpcMessage) → bytes             │
                    │  decode(bytes) → RpcMessage             │
                    └──────────────┬──────────────────────────┘
                                   │ bytes
                                   ▼
                    ┌─────────────────────────────────────────┐
                    │  RpcFramer                              │
                    │  (splits byte stream into chunks)       │
                    └─────────────────────────────────────────┘

The codec does NOT handle framing.  It receives exactly the payload bytes
(no Content-Length header, no WebSocket envelope) and returns exactly the
payload bytes.  The framer handles all the byte-stream boundary concerns.

Statefulness
-------------

Codec implementations SHOULD be stateless — a single instance should be
safe to use for encoding and decoding multiple messages in sequence.
Whether a codec instance is thread-safe depends on the implementation; the
:class:`RpcCodec` protocol itself makes no thread-safety guarantee.

Implementing a codec
---------------------

To implement a new codec, create a class with ``encode`` and ``decode`` methods
that match the signatures below.  No inheritance or registration is needed —
Python's ``typing.Protocol`` uses structural subtyping (duck typing).

Example skeleton::

    class MyCodec:
        def encode(self, msg: RpcMessage) -> bytes:
            # serialize msg to bytes however you like
            ...

        def decode(self, data: bytes) -> RpcMessage:
            # parse bytes into an RpcMessage
            # raise RpcDecodeError on failure
            ...
"""

from __future__ import annotations

from typing import Any, Optional, Protocol, TypeVar

from rpc.errors import RpcDecodeError  # noqa: F401 — re-exported for convenience
from rpc.message import RpcMessage

V = TypeVar("V", covariant=True)  # type: ignore[misc]


class RpcCodec(Protocol[V]):  # type: ignore[misc]
    """Structural protocol for RPC codec implementations.

    Any object with ``encode`` and ``decode`` methods matching these signatures
    satisfies the protocol — no explicit ``class MyCodec(RpcCodec)`` declaration
    is required.

    Type parameter ``V`` is the codec's native *value* type — the Python type
    that ``params`` and ``result`` fields will hold at runtime.  For a JSON
    codec ``V`` would be ``Any`` (the natural output of ``json.loads``).  For a
    MessagePack codec it might be a ``msgpack.Value`` type.

    In practice, because Python's runtime does not enforce generic type
    parameters, ``V`` is mainly useful for static type checkers (mypy/pyright)
    that read the type annotations.

    Example (minimal implementation)::

        class EchoCodec:
            \"\"\"Trivial codec that wraps bytes in an RpcRequest with method='echo'.\"\"\"

            def encode(self, msg: RpcMessage) -> bytes:
                import json
                # Very simplified — a real codec must handle all message types.
                return json.dumps({"method": msg.method}).encode()

            def decode(self, data: bytes) -> RpcMessage:
                import json
                from rpc.message import RpcRequest
                from rpc.errors import RpcDecodeError, PARSE_ERROR
                try:
                    obj = json.loads(data)
                except json.JSONDecodeError as exc:
                    raise RpcDecodeError(PARSE_ERROR, str(exc)) from exc
                return RpcRequest(id=1, method=obj["method"])
    """

    def encode(self, msg: "RpcMessage[Any]") -> bytes:
        """Encode an RpcMessage into bytes ready for the framer.

        The returned bytes are the *payload* only — no framing envelope
        (Content-Length header, length prefix, etc.).  The framer will add
        the envelope before writing to the transport.

        Args:
            msg: The message to encode.  Must be one of
                :class:`~rpc.message.RpcRequest`,
                :class:`~rpc.message.RpcResponse`,
                :class:`~rpc.message.RpcErrorResponse`, or
                :class:`~rpc.message.RpcNotification`.

        Returns:
            The encoded bytes.

        Raises:
            Any exception the implementation deems appropriate for encoding
            failures (e.g., ``TypeError`` if ``params`` contains
            non-serializable values).
        """
        ...

    def decode(self, data: bytes) -> "RpcMessage[Any]":
        """Decode bytes from the framer into an RpcMessage.

        Args:
            data: Raw payload bytes (no framing envelope).

        Returns:
            A parsed :class:`~rpc.message.RpcMessage` instance.

        Raises:
            :class:`~rpc.errors.RpcDecodeError`: If the bytes cannot be
                decoded (parse error) or do not represent a valid RPC message
                (invalid request).

        Example::

            try:
                msg = codec.decode(frame_bytes)
            except RpcDecodeError as exc:
                # exc.code  → -32700 or -32600
                # exc.message → human-readable description
                ...
        """
        ...


# ---------------------------------------------------------------------------
# Type alias used in server/client for annotation only
# ---------------------------------------------------------------------------

AnyCodec = RpcCodec  # type: ignore[type-arg]
"""Convenience alias for ``RpcCodec[Any]`` in unannotated contexts."""


# ---------------------------------------------------------------------------
# Optional helper: assert codec conformance at runtime (for debugging)
# ---------------------------------------------------------------------------


def check_codec(obj: object) -> Optional[str]:
    """Check whether *obj* looks like a valid :class:`RpcCodec`.

    This is a lightweight duck-type check — it does not perform an actual
    :func:`isinstance` against the ``Protocol`` class (which requires
    ``runtime_checkable``).  Instead it checks that ``encode`` and ``decode``
    are callable attributes.

    Args:
        obj: Any Python object.

    Returns:
        ``None`` if *obj* appears to be a valid codec; a human-readable
        error string otherwise.

    Example::

        assert check_codec(my_codec) is None, "Bad codec!"
    """
    for attr in ("encode", "decode"):
        if not callable(getattr(obj, attr, None)):
            return f"codec missing callable attribute '{attr}'"
    return None
