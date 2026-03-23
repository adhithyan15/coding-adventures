"""Message -- the atom of actor communication.

=== What Is a Message? ===

A Message is a sealed letter. Once created, its contents are fixed forever.
You cannot change who sent it, what it says, or when it was created. If you
need a different message, you create a new one. The original stays untouched.

This immutability is not a restriction -- it is the foundation of everything
the Actor model guarantees. If messages could be modified after creation,
then two actors reading the same message might see different things at
different times. Immutability eliminates that entire class of bugs.

=== Anatomy of a Message ===

Every message has six parts:

    id           -- A unique string that identifies this exact message.
                    No two messages in the universe share an id.
    timestamp    -- A monotonic nanosecond counter. Not wall-clock time
                    (which can jump backwards during daylight saving or
                    NTP adjustments), but a steadily increasing number
                    that tells you "this message was created after that one."
    sender_id    -- Who sent it. Set at creation time. An actor cannot
                    forge another actor's sender_id.
    content_type -- How to interpret the payload bytes. "text/plain" means
                    UTF-8 text. "application/json" means JSON. "image/png"
                    means a PNG image. The actor system never looks at this
                    field -- it is for the receiving actor to decide how to
                    decode the payload.
    payload      -- The actual data, always stored as raw bytes. A text
                    message stores UTF-8 bytes. A JSON message stores JSON
                    serialized to UTF-8. An image stores raw image bytes.
    metadata     -- A dictionary of string-to-string pairs for anything
                    extra: correlation IDs, trace IDs, priority hints.
                    The actor system passes this through without inspection.

=== Wire Format ===

Messages are serialized to a binary format for persistence and network
transport. The format has three sections:

    HEADER (17 bytes, fixed)
    +---------+--------+----------------+-----------------+
    | magic   | version| envelope_length| payload_length  |
    | 4 bytes | 1 byte | 4 bytes (u32)  | 8 bytes (u64)   |
    | "ACTM"  | 0x01   | big-endian     | big-endian      |
    +---------+--------+----------------+-----------------+

    ENVELOPE (variable length, UTF-8 JSON)
    Contains: id, timestamp, sender_id, content_type, metadata.
    Does NOT contain the payload -- that is separate.

    PAYLOAD (variable length, raw bytes)
    The actual data. Could be 0 bytes or 10 gigabytes.

Why separate envelope from payload? Two reasons:
1. No bloat. A 10MB image stays 10MB, not 13.3MB after Base64 encoding.
2. Scannable. To search for "all messages from agent_X," read headers
   and envelopes only -- skip payload bytes entirely.
"""

from __future__ import annotations

import json
import struct
import time
import uuid
from io import BytesIO
from typing import IO, Any

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

WIRE_MAGIC = b"ACTM"
"""Four-byte magic number at the start of every serialized message.

Like a file signature (JPEG starts with FF D8 FF, PNG with 89 50 4E 47),
this lets readers quickly reject data that is not an Actor message.
"""

WIRE_VERSION = 1
"""Current wire format version. Bumped when the format changes.

Readers must handle all versions <= their own. Readers encountering a
version > their own must raise VersionError (not crash silently).
"""

HEADER_FORMAT = ">4sBIQ"
"""struct format string for the 17-byte header.

Breakdown:
    4s  -- 4-byte magic string ("ACTM")
    B   -- 1-byte unsigned integer (version)
    I   -- 4-byte unsigned integer, big-endian (envelope length)
    Q   -- 8-byte unsigned integer, big-endian (payload length)

Big-endian (>) means the most significant byte comes first, which is the
standard network byte order. This ensures the same bytes are read
identically on any machine regardless of its native byte order.
"""

HEADER_SIZE = struct.calcsize(HEADER_FORMAT)
"""Exactly 17 bytes: 4 (magic) + 1 (version) + 4 (envelope_len) + 8 (payload_len)."""


# ---------------------------------------------------------------------------
# Exceptions
# ---------------------------------------------------------------------------


class VersionError(Exception):
    """Raised when a message uses a wire format version we cannot parse.

    This is NOT a bug -- it means the sender is running newer software.
    The fix is to upgrade the reader, not to crash.
    """

    def __init__(self, encountered: int, max_supported: int) -> None:
        self.encountered = encountered
        self.max_supported = max_supported
        super().__init__(
            f"Wire format version {encountered} is not supported. "
            f"Maximum supported version is {max_supported}. "
            f"Please upgrade to a newer version of the actor package."
        )


class InvalidFormatError(Exception):
    """Raised when the magic bytes do not match "ACTM".

    This means the data is not an Actor message at all -- it might be
    a different file format, corrupted data, or random bytes.
    """


# ---------------------------------------------------------------------------
# Monotonic clock for timestamps
# ---------------------------------------------------------------------------

# We use a module-level counter to ensure strictly increasing timestamps
# even when two messages are created in the same nanosecond. The counter
# combines time.monotonic_ns() with a sequence number.
_last_timestamp = 0


def _next_timestamp() -> int:
    """Return a strictly increasing monotonic nanosecond timestamp.

    time.monotonic_ns() gives nanosecond-resolution monotonic time, but
    two calls in rapid succession might return the same value. We track
    the last timestamp and ensure each new one is strictly greater.

    This is safe for single-threaded use. For multi-threaded use, you
    would need a lock -- but this V1 actor system is single-threaded.
    """
    global _last_timestamp  # noqa: PLW0603
    now = time.monotonic_ns()
    if now <= _last_timestamp:
        now = _last_timestamp + 1
    _last_timestamp = now
    return now


# ---------------------------------------------------------------------------
# Message class
# ---------------------------------------------------------------------------


class Message:
    """Immutable message -- the atom of actor communication.

    A Message is like a sealed letter: once created, its contents cannot be
    changed. The envelope (id, timestamp, sender_id, content_type, metadata)
    describes the letter. The payload is the letter itself -- always raw bytes.

    === Creating Messages ===

    For common cases, use the convenience constructors:

        >>> msg = Message.text(sender_id="agent", payload="hello world")
        >>> msg.content_type
        'text/plain'
        >>> msg.payload_text
        'hello world'

        >>> msg = Message.json(sender_id="agent", payload={"key": "value"})
        >>> msg.content_type
        'application/json'
        >>> msg.payload_json
        {'key': 'value'}

        >>> msg = Message.binary(
        ...     sender_id="browser",
        ...     content_type="image/png",
        ...     payload=b"\\x89PNG...",
        ... )

    For full control, use the constructor directly:

        >>> msg = Message(
        ...     sender_id="agent",
        ...     content_type="text/plain",
        ...     payload=b"hello",
        ...     metadata={"priority": "high"},
        ... )

    === Serialization ===

    Messages serialize to a compact binary wire format:

        >>> data = msg.to_bytes()
        >>> restored = Message.from_bytes(data)
        >>> restored.payload == msg.payload
        True

    The envelope (everything except payload) can be serialized separately
    for indexing and debugging:

        >>> print(msg.envelope_to_json())
        {"id": "...", "timestamp": ..., ...}

    Attributes:
        WIRE_VERSION: Current wire format version (class-level constant).
    """

    # __slots__ prevents adding arbitrary attributes to instances, which:
    # 1. Enforces immutability -- you cannot do msg.new_field = "oops"
    # 2. Saves memory -- no per-instance __dict__
    # 3. Is slightly faster for attribute access
    __slots__ = (
        "_id",
        "_timestamp",
        "_sender_id",
        "_content_type",
        "_payload",
        "_metadata",
    )

    WIRE_VERSION = WIRE_VERSION

    def __init__(
        self,
        sender_id: str,
        content_type: str,
        payload: bytes,
        metadata: dict[str, str] | None = None,
        *,
        _id: str | None = None,
        _timestamp: int | None = None,
    ) -> None:
        """Create an immutable Message.

        Args:
            sender_id: The actor that created this message.
            content_type: MIME type describing how to interpret the payload.
                Common values: "text/plain", "application/json", "image/png".
            payload: The message body as raw bytes. For text, encode with
                UTF-8 first. Or use Message.text() which handles encoding.
            metadata: Optional string-to-string key-value pairs for
                extensibility. Passed through without interpretation.
            _id: Internal -- used by deserialization to restore the original
                message ID. Do not set this manually.
            _timestamp: Internal -- used by deserialization to restore the
                original timestamp. Do not set this manually.
        """
        # object.__setattr__ is required because __slots__ + property
        # decorators without setters means normal self._x = y would raise
        # AttributeError after the first set (since there is no setter).
        # We bypass this restriction during __init__ only.
        object.__setattr__(self, "_id", _id or f"msg_{uuid.uuid4().hex}")
        object.__setattr__(self, "_timestamp", _timestamp or _next_timestamp())
        object.__setattr__(self, "_sender_id", sender_id)
        object.__setattr__(self, "_content_type", content_type)
        object.__setattr__(self, "_payload", payload)
        object.__setattr__(
            self, "_metadata", dict(metadata) if metadata else {}
        )

    # ------------------------------------------------------------------
    # Block attribute mutation to enforce immutability
    # ------------------------------------------------------------------

    def __setattr__(self, name: str, value: Any) -> None:
        """Prevent modification of any attribute after creation.

        This makes Message truly immutable. Any attempt to do:
            msg.sender_id = "hacker"
        will raise AttributeError.
        """
        raise AttributeError(
            f"Message is immutable. Cannot set attribute '{name}'."
        )

    def __delattr__(self, name: str) -> None:
        """Prevent deletion of any attribute after creation."""
        raise AttributeError(
            f"Message is immutable. Cannot delete attribute '{name}'."
        )

    # ------------------------------------------------------------------
    # Read-only properties
    # ------------------------------------------------------------------

    @property
    def id(self) -> str:
        """Unique identifier for this message.

        Auto-generated at creation time using UUID4. Two messages with
        the same id are considered identical -- they represent the same
        logical message (perhaps delivered twice due to retry logic).
        """
        return self._id

    @property
    def timestamp(self) -> int:
        """Monotonic nanosecond timestamp.

        Strictly increasing within a single process. Used for ordering
        messages, not for wall-clock time. Two messages from different
        machines may have unrelated timestamps -- ordering across
        machines requires a distributed clock (not implemented in V1).
        """
        return self._timestamp

    @property
    def sender_id(self) -> str:
        """The actor that created this message.

        Set at creation time and cannot be changed. An actor cannot
        forge another actor's sender_id (in this implementation, because
        we trust the creator; in a distributed system, you would add
        cryptographic signatures).
        """
        return self._sender_id

    @property
    def content_type(self) -> str:
        """MIME type describing how to interpret the payload.

        Common values:
            "text/plain"              -- UTF-8 text
            "application/json"        -- JSON document
            "application/octet-stream" -- opaque binary
            "image/png"               -- PNG image
        """
        return self._content_type

    @property
    def payload(self) -> bytes:
        """The message body as raw bytes.

        The content_type property tells the receiver how to decode these
        bytes. The actor system never inspects the payload -- it is
        completely opaque to the infrastructure.
        """
        return self._payload

    @property
    def payload_text(self) -> str:
        """Decode the payload as UTF-8 text.

        Convenience property for text/plain messages. Raises UnicodeDecodeError
        if the payload is not valid UTF-8.

        Example:
            >>> msg = Message.text(sender_id="a", payload="hello")
            >>> msg.payload_text
            'hello'
        """
        return self._payload.decode("utf-8")

    @property
    def payload_json(self) -> dict[str, Any] | list[Any]:
        """Parse the payload as JSON.

        Convenience property for application/json messages. Raises
        json.JSONDecodeError if the payload is not valid JSON.

        Example:
            >>> msg = Message.json(sender_id="a", payload={"x": 1})
            >>> msg.payload_json
            {'x': 1}
        """
        return json.loads(self._payload)

    @property
    def metadata(self) -> dict[str, str]:
        """Key-value pairs for extensibility.

        Returns a copy to preserve immutability -- modifying the returned
        dict does not affect the message.
        """
        return dict(self._metadata)

    # ------------------------------------------------------------------
    # Convenience constructors
    # ------------------------------------------------------------------

    @classmethod
    def text(
        cls,
        sender_id: str,
        payload: str,
        metadata: dict[str, str] | None = None,
    ) -> Message:
        """Create a text/plain message.

        Encodes the string as UTF-8 bytes automatically.

        Args:
            sender_id: The actor creating this message.
            payload: The text content (a Python string, not bytes).
            metadata: Optional key-value pairs.

        Returns:
            A new Message with content_type="text/plain".

        Example:
            >>> msg = Message.text("agent", "hello world")
            >>> msg.payload_text
            'hello world'
        """
        return cls(
            sender_id=sender_id,
            content_type="text/plain",
            payload=payload.encode("utf-8"),
            metadata=metadata,
        )

    @classmethod
    def json(
        cls,
        sender_id: str,
        payload: dict[str, Any] | list[Any],
        metadata: dict[str, str] | None = None,
    ) -> Message:
        """Create an application/json message.

        Serializes the dict or list to JSON, then encodes as UTF-8 bytes.

        Args:
            sender_id: The actor creating this message.
            payload: A JSON-serializable dict or list.
            metadata: Optional key-value pairs.

        Returns:
            A new Message with content_type="application/json".

        Example:
            >>> msg = Message.json("agent", {"key": "value"})
            >>> msg.payload_json
            {'key': 'value'}
        """
        json_str = json.dumps(payload, separators=(",", ":"))
        return cls(
            sender_id=sender_id,
            content_type="application/json",
            payload=json_str.encode("utf-8"),
            metadata=metadata,
        )

    @classmethod
    def binary(
        cls,
        sender_id: str,
        content_type: str,
        payload: bytes,
        metadata: dict[str, str] | None = None,
    ) -> Message:
        """Create a binary message with an explicit content type.

        Use this for images, videos, or any non-text payload.

        Args:
            sender_id: The actor creating this message.
            content_type: MIME type (e.g., "image/png", "video/mp4").
            payload: Raw binary data.
            metadata: Optional key-value pairs.

        Returns:
            A new Message with the specified content_type.

        Example:
            >>> png_header = b"\\x89PNG\\r\\n\\x1a\\n"
            >>> msg = Message.binary("browser", "image/png", png_header)
            >>> msg.payload
            b'\\x89PNG\\r\\n\\x1a\\n'
        """
        return cls(
            sender_id=sender_id,
            content_type=content_type,
            payload=payload,
            metadata=metadata,
        )

    # ------------------------------------------------------------------
    # Serialization
    # ------------------------------------------------------------------

    def envelope_to_json(self) -> str:
        """Serialize the envelope (everything except payload) to JSON.

        The envelope contains: id, timestamp, sender_id, content_type,
        and metadata. The payload is excluded because it may be large
        binary data that does not belong in a JSON string.

        This is useful for:
        - Logging: print the envelope without dumping a 10MB image
        - Indexing: build a searchable index of message metadata
        - Debugging: inspect message routing without touching payloads

        Returns:
            A JSON string containing the envelope fields.
        """
        envelope = {
            "id": self._id,
            "timestamp": self._timestamp,
            "sender_id": self._sender_id,
            "content_type": self._content_type,
            "metadata": self._metadata,
        }
        return json.dumps(envelope, separators=(",", ":"))

    def to_bytes(self) -> bytes:
        """Serialize to the binary wire format.

        Layout:
            [4 bytes magic "ACTM"]
            [1 byte version]
            [4 bytes envelope length, big-endian u32]
            [8 bytes payload length, big-endian u64]
            [envelope bytes (JSON)]
            [payload bytes (raw)]

        The envelope is JSON-encoded UTF-8. The payload is raw bytes,
        never Base64-encoded. This keeps binary data at its original
        size with zero bloat.

        Returns:
            The complete serialized message as bytes.
        """
        envelope_bytes = self.envelope_to_json().encode("utf-8")
        header = struct.pack(
            HEADER_FORMAT,
            WIRE_MAGIC,
            WIRE_VERSION,
            len(envelope_bytes),
            len(self._payload),
        )
        return header + envelope_bytes + self._payload

    @classmethod
    def from_bytes(cls, data: bytes) -> Message:
        """Deserialize a Message from the binary wire format.

        Validates the magic bytes and version before parsing. Raises
        InvalidFormatError if the magic is wrong, VersionError if the
        version is too new.

        Args:
            data: The complete serialized message bytes.

        Returns:
            The deserialized Message.

        Raises:
            InvalidFormatError: If the magic bytes are not "ACTM".
            VersionError: If the version is greater than WIRE_VERSION.
        """
        stream = BytesIO(data)
        return cls.from_stream(stream)

    @classmethod
    def from_stream(cls, stream: IO[bytes]) -> Message | None:
        """Read exactly one Message from a byte stream.

        This is used for reading messages from channel log files and
        network sockets. After reading, the stream is positioned at the
        start of the next message (or at EOF).

        The reading process:
        1. Read 17-byte header (magic + version + lengths)
        2. Read envelope_length bytes and parse as JSON
        3. Read payload_length bytes as raw binary

        Args:
            stream: A readable byte stream (file, BytesIO, socket).

        Returns:
            The deserialized Message, or None if the stream is at EOF
            or has insufficient data for a complete header.

        Raises:
            InvalidFormatError: If the magic bytes are not "ACTM".
            VersionError: If the version is greater than WIRE_VERSION.
        """
        # Step 1: Read the 17-byte header
        header_data = stream.read(HEADER_SIZE)
        if not header_data:
            return None
        if len(header_data) < HEADER_SIZE:
            # Truncated header -- this can happen after a crash mid-write.
            # Return None to signal "no complete message available."
            return None

        magic, version, envelope_length, payload_length = struct.unpack(
            HEADER_FORMAT, header_data
        )

        # Step 2: Validate magic bytes
        if magic != WIRE_MAGIC:
            raise InvalidFormatError(
                f"Expected magic bytes 'ACTM', got {magic!r}. "
                f"This data is not an Actor message."
            )

        # Step 3: Validate version
        if version > WIRE_VERSION:
            raise VersionError(version, WIRE_VERSION)

        # Step 4: Read envelope JSON
        envelope_data = stream.read(envelope_length)
        if len(envelope_data) < envelope_length:
            return None  # Truncated envelope
        envelope = json.loads(envelope_data)

        # Step 5: Read payload bytes
        payload_data = stream.read(payload_length)
        if len(payload_data) < payload_length:
            return None  # Truncated payload

        # Step 6: Reconstruct the Message with original id and timestamp
        return cls(
            sender_id=envelope["sender_id"],
            content_type=envelope["content_type"],
            payload=payload_data,
            metadata=envelope.get("metadata"),
            _id=envelope["id"],
            _timestamp=envelope["timestamp"],
        )

    # ------------------------------------------------------------------
    # String representation
    # ------------------------------------------------------------------

    def __repr__(self) -> str:
        """Human-readable representation for debugging.

        Shows the id, sender, content type, and payload size.
        Does NOT show the payload itself (it might be megabytes of binary).
        """
        return (
            f"Message(id={self._id!r}, sender_id={self._sender_id!r}, "
            f"content_type={self._content_type!r}, "
            f"payload_size={len(self._payload)})"
        )

    def __eq__(self, other: object) -> bool:
        """Two messages are equal if they have the same id.

        This follows the spec: "Two messages with the same id are the
        same message." Content comparison is not needed -- the id is
        the unique identity.
        """
        if not isinstance(other, Message):
            return NotImplemented
        return self._id == other._id
