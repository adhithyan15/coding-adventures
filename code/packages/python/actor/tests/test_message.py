"""Tests for Message -- the atom of actor communication.

These tests verify that messages are immutable, self-describing, and
correctly serialize to and from the binary wire format.
"""

from __future__ import annotations

import json
from io import BytesIO

import pytest

from actor.message import (
    HEADER_SIZE,
    WIRE_VERSION,
    InvalidFormatError,
    Message,
    VersionError,
)

# ═══════════════════════════════════════════════════════════════
# Test 1: Create message
# ═══════════════════════════════════════════════════════════════


class TestCreateMessage:
    """Test 1: Create a Message with all fields, verify all properties."""

    def test_all_fields_accessible(self) -> None:
        """All properties return correct values after creation."""
        msg = Message(
            sender_id="agent_1",
            content_type="text/plain",
            payload=b"hello world",
            metadata={"priority": "high", "trace_id": "abc123"},
        )
        assert msg.sender_id == "agent_1"
        assert msg.content_type == "text/plain"
        assert msg.payload == b"hello world"
        assert msg.metadata == {"priority": "high", "trace_id": "abc123"}
        assert msg.id.startswith("msg_")
        assert isinstance(msg.timestamp, int)
        assert msg.timestamp > 0


# ═══════════════════════════════════════════════════════════════
# Test 2: Immutability
# ═══════════════════════════════════════════════════════════════


class TestImmutability:
    """Test 2: Verify Message has no setter methods and cannot be modified."""

    def test_cannot_set_sender_id(self) -> None:
        """Attempting to set sender_id raises AttributeError."""
        msg = Message.text("agent", "hello")
        with pytest.raises(AttributeError, match="immutable"):
            msg.sender_id = "hacker"  # type: ignore[misc]

    def test_cannot_set_payload(self) -> None:
        """Attempting to set payload raises AttributeError."""
        msg = Message.text("agent", "hello")
        with pytest.raises(AttributeError, match="immutable"):
            msg.payload = b"evil"  # type: ignore[misc]

    def test_cannot_set_arbitrary_attribute(self) -> None:
        """Attempting to add a new attribute raises AttributeError."""
        msg = Message.text("agent", "hello")
        with pytest.raises(AttributeError):
            msg.new_field = "oops"  # type: ignore[attr-defined]

    def test_cannot_delete_attribute(self) -> None:
        """Attempting to delete an attribute raises AttributeError."""
        msg = Message.text("agent", "hello")
        with pytest.raises(AttributeError, match="immutable"):
            del msg.sender_id  # type: ignore[misc]

    def test_metadata_copy(self) -> None:
        """Modifying the returned metadata dict does not affect the message."""
        msg = Message.text("agent", "hello", metadata={"key": "value"})
        meta = msg.metadata
        meta["key"] = "changed"
        assert msg.metadata == {"key": "value"}


# ═══════════════════════════════════════════════════════════════
# Test 3: Unique IDs
# ═══════════════════════════════════════════════════════════════


class TestUniqueIDs:
    """Test 3: Create 1000 messages, verify all IDs are unique."""

    def test_thousand_unique_ids(self) -> None:
        """1000 messages all have distinct IDs."""
        ids = {Message.text("agent", f"msg_{i}").id for i in range(1000)}
        assert len(ids) == 1000


# ═══════════════════════════════════════════════════════════════
# Test 4: Timestamp ordering
# ═══════════════════════════════════════════════════════════════


class TestTimestampOrdering:
    """Test 4: Messages created sequentially have strictly increasing timestamps."""

    def test_strictly_increasing(self) -> None:
        """Each message has a timestamp greater than the previous."""
        msgs = [Message.text("agent", f"msg_{i}") for i in range(100)]
        for i in range(1, len(msgs)):
            assert msgs[i].timestamp > msgs[i - 1].timestamp


# ═══════════════════════════════════════════════════════════════
# Test 5: Wire format round-trip (text)
# ═══════════════════════════════════════════════════════════════


class TestWireFormatRoundTripText:
    """Test 5: Text message survives to_bytes/from_bytes round-trip."""

    def test_text_round_trip(self) -> None:
        """All fields match after serialization and deserialization."""
        original = Message.text(
            sender_id="agent",
            payload="hello world",
            metadata={"key": "value"},
        )
        data = original.to_bytes()
        restored = Message.from_bytes(data)

        assert restored.id == original.id
        assert restored.timestamp == original.timestamp
        assert restored.sender_id == original.sender_id
        assert restored.content_type == original.content_type
        assert restored.payload == original.payload
        assert restored.metadata == original.metadata
        assert restored.payload_text == "hello world"


# ═══════════════════════════════════════════════════════════════
# Test 6: Wire format round-trip (binary)
# ═══════════════════════════════════════════════════════════════


class TestWireFormatRoundTripBinary:
    """Test 6: Binary message (PNG header) survives round-trip."""

    def test_binary_round_trip(self) -> None:
        """Binary payload bytes are identical after round-trip."""
        png_header = b"\x89PNG\r\n\x1a\n" + b"\x00" * 100
        original = Message.binary("browser", "image/png", png_header)
        data = original.to_bytes()
        restored = Message.from_bytes(data)

        assert restored.payload == png_header
        assert restored.content_type == "image/png"


# ═══════════════════════════════════════════════════════════════
# Test 7: Metadata passthrough
# ═══════════════════════════════════════════════════════════════


class TestMetadataPassthrough:
    """Test 7: Metadata survives serialization/deserialization."""

    def test_metadata_preserved(self) -> None:
        """All metadata key-value pairs survive the round-trip."""
        metadata = {
            "correlation_id": "req_abc123",
            "priority": "high",
            "trace_id": "trace_xyz",
        }
        original = Message.text("agent", "hello", metadata=metadata)
        restored = Message.from_bytes(original.to_bytes())
        assert restored.metadata == metadata


# ═══════════════════════════════════════════════════════════════
# Test 8: Empty payload
# ═══════════════════════════════════════════════════════════════


class TestEmptyPayload:
    """Test 8: Message with zero-length payload works correctly."""

    def test_empty_payload(self) -> None:
        """Zero-byte payload serializes and deserializes correctly."""
        msg = Message(
            sender_id="agent",
            content_type="application/octet-stream",
            payload=b"",
        )
        assert msg.payload == b""
        assert len(msg.payload) == 0

        restored = Message.from_bytes(msg.to_bytes())
        assert restored.payload == b""


# ═══════════════════════════════════════════════════════════════
# Test 9: Large payload
# ═══════════════════════════════════════════════════════════════


class TestLargePayload:
    """Test 9: 1MB binary payload serializes correctly."""

    def test_one_megabyte_payload(self) -> None:
        """1MB payload survives round-trip with correct size."""
        payload = b"\xab" * (1024 * 1024)  # 1MB
        msg = Message.binary("agent", "application/octet-stream", payload)
        restored = Message.from_bytes(msg.to_bytes())
        assert restored.payload == payload
        assert len(restored.payload) == 1024 * 1024


# ═══════════════════════════════════════════════════════════════
# Test 10: Content type preservation
# ═══════════════════════════════════════════════════════════════


class TestContentType:
    """Test 10: Content type is preserved across serialization."""

    def test_custom_content_type(self) -> None:
        """Custom content types survive round-trip."""
        msg = Message.binary("agent", "video/mp4", b"\x00\x00\x00\x1c")
        restored = Message.from_bytes(msg.to_bytes())
        assert restored.content_type == "video/mp4"


# ═══════════════════════════════════════════════════════════════
# Test 11: Convenience constructors
# ═══════════════════════════════════════════════════════════════


class TestConvenienceConstructors:
    """Test 11: Message.text(), Message.json(), Message.binary() all work."""

    def test_text_constructor(self) -> None:
        """Message.text() sets content_type and encodes payload."""
        msg = Message.text("agent", "hello")
        assert msg.content_type == "text/plain"
        assert msg.payload == b"hello"

    def test_json_constructor(self) -> None:
        """Message.json() sets content_type and serializes payload."""
        msg = Message.json("agent", {"key": "value"})
        assert msg.content_type == "application/json"
        assert json.loads(msg.payload) == {"key": "value"}

    def test_json_constructor_list(self) -> None:
        """Message.json() works with lists too."""
        msg = Message.json("agent", [1, 2, 3])
        assert msg.payload_json == [1, 2, 3]

    def test_binary_constructor(self) -> None:
        """Message.binary() sets content_type and uses raw bytes."""
        data = b"\x89PNG\r\n\x1a\n"
        msg = Message.binary("agent", "image/png", data)
        assert msg.content_type == "image/png"
        assert msg.payload == data


# ═══════════════════════════════════════════════════════════════
# Test 12: payload_text
# ═══════════════════════════════════════════════════════════════


class TestPayloadText:
    """Test 12: Text message payload_text returns decoded string."""

    def test_payload_text(self) -> None:
        """payload_text decodes UTF-8 bytes back to string."""
        msg = Message.text("agent", "hello world")
        assert msg.payload_text == "hello world"

    def test_payload_text_unicode(self) -> None:
        """payload_text handles unicode correctly."""
        msg = Message.text("agent", "Hello, world! 42")
        assert msg.payload_text == "Hello, world! 42"


# ═══════════════════════════════════════════════════════════════
# Test 13: payload_json
# ═══════════════════════════════════════════════════════════════


class TestPayloadJson:
    """Test 13: JSON message payload_json returns parsed dict/list."""

    def test_payload_json_dict(self) -> None:
        """payload_json returns parsed dictionary."""
        msg = Message.json("agent", {"name": "test", "count": 42})
        result = msg.payload_json
        assert result == {"name": "test", "count": 42}

    def test_payload_json_list(self) -> None:
        """payload_json returns parsed list."""
        msg = Message.json("agent", [1, "two", 3.0])
        result = msg.payload_json
        assert result == [1, "two", 3.0]


# ═══════════════════════════════════════════════════════════════
# Test 14: Envelope-only serialization
# ═══════════════════════════════════════════════════════════════


class TestEnvelopeSerialization:
    """Test 14: envelope_to_json() produces JSON without payload."""

    def test_envelope_contains_all_fields(self) -> None:
        """Envelope JSON has id, timestamp, sender_id, content_type, metadata."""
        msg = Message.text("agent", "hello", metadata={"key": "value"})
        envelope_str = msg.envelope_to_json()
        envelope = json.loads(envelope_str)

        assert envelope["id"] == msg.id
        assert envelope["timestamp"] == msg.timestamp
        assert envelope["sender_id"] == "agent"
        assert envelope["content_type"] == "text/plain"
        assert envelope["metadata"] == {"key": "value"}

    def test_envelope_does_not_contain_payload(self) -> None:
        """Payload is not in the envelope JSON."""
        msg = Message.text("agent", "secret data")
        envelope = json.loads(msg.envelope_to_json())
        assert "payload" not in envelope


# ═══════════════════════════════════════════════════════════════
# Test 15: Wire format magic
# ═══════════════════════════════════════════════════════════════


class TestWireFormatMagic:
    """Test 15: to_bytes() starts with 'ACTM' magic bytes."""

    def test_starts_with_magic(self) -> None:
        """First 4 bytes of serialized message are 'ACTM'."""
        msg = Message.text("agent", "hello")
        data = msg.to_bytes()
        assert data[:4] == b"ACTM"


# ═══════════════════════════════════════════════════════════════
# Test 16: Wire format version
# ═══════════════════════════════════════════════════════════════


class TestWireFormatVersion:
    """Test 16: to_bytes() contains correct version byte."""

    def test_version_byte(self) -> None:
        """Version byte (5th byte) matches WIRE_VERSION."""
        msg = Message.text("agent", "hello")
        data = msg.to_bytes()
        assert data[4] == WIRE_VERSION


# ═══════════════════════════════════════════════════════════════
# Test 17: Future version rejection
# ═══════════════════════════════════════════════════════════════


class TestFutureVersionRejection:
    """Test 17: from_bytes() with version > WIRE_VERSION raises VersionError."""

    def test_future_version_raises(self) -> None:
        """Version 99 should raise VersionError, not crash."""
        msg = Message.text("agent", "hello")
        data = bytearray(msg.to_bytes())
        # Overwrite version byte (offset 4) with a future version
        data[4] = 99
        with pytest.raises(VersionError) as exc_info:
            Message.from_bytes(bytes(data))
        assert exc_info.value.encountered == 99
        assert exc_info.value.max_supported == WIRE_VERSION


# ═══════════════════════════════════════════════════════════════
# Test 18: Corrupt magic rejection
# ═══════════════════════════════════════════════════════════════


class TestCorruptMagicRejection:
    """Test 18: from_bytes() with wrong magic raises InvalidFormat."""

    def test_wrong_magic_raises(self) -> None:
        """Non-ACTM magic bytes should raise InvalidFormatError."""
        msg = Message.text("agent", "hello")
        data = bytearray(msg.to_bytes())
        data[0:4] = b"NOPE"
        with pytest.raises(InvalidFormatError):
            Message.from_bytes(bytes(data))


# ═══════════════════════════════════════════════════════════════
# Test 19: Stream reading
# ═══════════════════════════════════════════════════════════════


class TestStreamReading:
    """Test 19: from_stream() reads exactly one message from a byte stream."""

    def test_reads_one_message(self) -> None:
        """Stream is positioned at the start of the next message after reading."""
        msg1 = Message.text("agent", "first")
        msg2 = Message.text("agent", "second")
        stream = BytesIO(msg1.to_bytes() + msg2.to_bytes())

        restored1 = Message.from_stream(stream)
        assert restored1 is not None
        assert restored1.payload_text == "first"

        restored2 = Message.from_stream(stream)
        assert restored2 is not None
        assert restored2.payload_text == "second"

    def test_returns_none_at_eof(self) -> None:
        """from_stream() returns None when the stream is at EOF."""
        stream = BytesIO(b"")
        result = Message.from_stream(stream)
        assert result is None

    def test_returns_none_on_truncated_header(self) -> None:
        """from_stream() returns None when header is incomplete."""
        stream = BytesIO(b"ACT")  # Only 3 of 17 header bytes
        result = Message.from_stream(stream)
        assert result is None

    def test_returns_none_on_truncated_envelope(self) -> None:
        """from_stream() returns None when envelope is truncated."""
        msg = Message.text("agent", "hello")
        data = msg.to_bytes()
        # Keep header but truncate envelope
        truncated = data[: HEADER_SIZE + 5]
        stream = BytesIO(truncated)
        result = Message.from_stream(stream)
        assert result is None

    def test_returns_none_on_truncated_payload(self) -> None:
        """from_stream() returns None when payload is truncated."""
        msg = Message.text("agent", "hello world")
        data = msg.to_bytes()
        # Truncate the last few bytes of payload
        truncated = data[:-3]
        stream = BytesIO(truncated)
        result = Message.from_stream(stream)
        assert result is None


# ═══════════════════════════════════════════════════════════════
# Additional edge cases
# ═══════════════════════════════════════════════════════════════


class TestMessageEquality:
    """Messages with the same id are equal."""

    def test_equal_messages(self) -> None:
        """Two messages deserialized from the same bytes are equal."""
        msg = Message.text("agent", "hello")
        restored = Message.from_bytes(msg.to_bytes())
        assert msg == restored

    def test_not_equal_to_non_message(self) -> None:
        """Message is not equal to non-Message objects."""
        msg = Message.text("agent", "hello")
        assert msg != "not a message"


class TestMessageRepr:
    """Message repr shows useful debugging info."""

    def test_repr(self) -> None:
        """repr includes id, sender_id, content_type, and payload_size."""
        msg = Message.text("agent", "hello")
        r = repr(msg)
        assert "agent" in r
        assert "text/plain" in r
        assert "payload_size=5" in r


class TestDefaultMetadata:
    """Message with no metadata gets an empty dict."""

    def test_default_metadata(self) -> None:
        """No metadata argument results in empty dict."""
        msg = Message.text("agent", "hello")
        assert msg.metadata == {}
