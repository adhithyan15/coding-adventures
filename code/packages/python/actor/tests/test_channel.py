"""Tests for Channel -- one-way, append-only, ordered message log.

These tests verify channel operations (append, read, slice), binary
persistence to disk, and crash recovery from truncated writes.
"""

from __future__ import annotations

import struct
from pathlib import Path

from actor.channel import Channel
from actor.message import HEADER_FORMAT, HEADER_SIZE, WIRE_MAGIC, Message

# ═══════════════════════════════════════════════════════════════
# Test 20: Create channel
# ═══════════════════════════════════════════════════════════════


class TestCreateChannel:
    """Test 20: Create a Channel, verify id and name."""

    def test_id_and_name(self) -> None:
        """Channel stores id and name correctly."""
        ch = Channel("ch_001", "greetings")
        assert ch.id == "ch_001"
        assert ch.name == "greetings"

    def test_created_at(self) -> None:
        """Channel has a positive created_at timestamp."""
        ch = Channel("ch_001", "greetings")
        assert isinstance(ch.created_at, int)
        assert ch.created_at > 0


# ═══════════════════════════════════════════════════════════════
# Test 21: Append and length
# ═══════════════════════════════════════════════════════════════


class TestAppendAndLength:
    """Test 21: Append 3 messages, verify length() returns 3."""

    def test_length_after_appends(self) -> None:
        """Length increases by 1 for each append."""
        ch = Channel("ch1", "test")
        assert ch.length() == 0
        ch.append(Message.text("a", "msg1"))
        ch.append(Message.text("a", "msg2"))
        ch.append(Message.text("a", "msg3"))
        assert ch.length() == 3


# ═══════════════════════════════════════════════════════════════
# Test 22: Append returns sequence number
# ═══════════════════════════════════════════════════════════════


class TestAppendSequenceNumber:
    """Test 22: Verify append returns 0, 1, 2 for successive appends."""

    def test_sequence_numbers(self) -> None:
        """Sequence numbers are 0-indexed and monotonically increasing."""
        ch = Channel("ch1", "test")
        assert ch.append(Message.text("a", "first")) == 0
        assert ch.append(Message.text("a", "second")) == 1
        assert ch.append(Message.text("a", "third")) == 2


# ═══════════════════════════════════════════════════════════════
# Test 23: Read from beginning
# ═══════════════════════════════════════════════════════════════


class TestReadFromBeginning:
    """Test 23: Append 5 messages, read(0, 5), verify all 5 in order."""

    def test_read_all(self) -> None:
        """Reading from offset 0 with sufficient limit returns all messages."""
        ch = Channel("ch1", "test")
        originals = []
        for i in range(5):
            msg = Message.text("a", f"msg_{i}")
            ch.append(msg)
            originals.append(msg)

        result = ch.read(0, 5)
        assert len(result) == 5
        for i, msg in enumerate(result):
            assert msg.payload_text == f"msg_{i}"


# ═══════════════════════════════════════════════════════════════
# Test 24: Read with offset
# ═══════════════════════════════════════════════════════════════


class TestReadWithOffset:
    """Test 24: Append 5 messages, read(2, 3), verify messages 2, 3, 4."""

    def test_offset_read(self) -> None:
        """Reading from offset 2 with limit 3 returns messages 2, 3, 4."""
        ch = Channel("ch1", "test")
        for i in range(5):
            ch.append(Message.text("a", f"msg_{i}"))

        result = ch.read(2, 3)
        assert len(result) == 3
        assert result[0].payload_text == "msg_2"
        assert result[1].payload_text == "msg_3"
        assert result[2].payload_text == "msg_4"


# ═══════════════════════════════════════════════════════════════
# Test 25: Read past end
# ═══════════════════════════════════════════════════════════════


class TestReadPastEnd:
    """Test 25: Append 3 messages, read(5, 10), verify empty list."""

    def test_past_end_returns_empty(self) -> None:
        """Reading past the end of the log returns an empty list."""
        ch = Channel("ch1", "test")
        for i in range(3):
            ch.append(Message.text("a", f"msg_{i}"))

        result = ch.read(5, 10)
        assert result == []


# ═══════════════════════════════════════════════════════════════
# Test 26: Read with limit
# ═══════════════════════════════════════════════════════════════


class TestReadWithLimit:
    """Test 26: Append 10 messages, read(0, 3), verify only 3 returned."""

    def test_limit_respected(self) -> None:
        """Limit caps the number of messages returned."""
        ch = Channel("ch1", "test")
        for i in range(10):
            ch.append(Message.text("a", f"msg_{i}"))

        result = ch.read(0, 3)
        assert len(result) == 3
        assert result[0].payload_text == "msg_0"
        assert result[2].payload_text == "msg_2"


# ═══════════════════════════════════════════════════════════════
# Test 27: Slice
# ═══════════════════════════════════════════════════════════════


class TestSlice:
    """Test 27: Append 5 messages, slice(1, 4), verify messages 1, 2, 3."""

    def test_slice(self) -> None:
        """Slice returns messages in the [start, end) range."""
        ch = Channel("ch1", "test")
        for i in range(5):
            ch.append(Message.text("a", f"msg_{i}"))

        result = ch.slice(1, 4)
        assert len(result) == 3
        assert result[0].payload_text == "msg_1"
        assert result[1].payload_text == "msg_2"
        assert result[2].payload_text == "msg_3"


# ═══════════════════════════════════════════════════════════════
# Test 28: Independent readers
# ═══════════════════════════════════════════════════════════════


class TestIndependentReaders:
    """Test 28: Two consumers read the same channel at different offsets."""

    def test_independent_offsets(self) -> None:
        """Two readers see correct messages independently."""
        ch = Channel("ch1", "test")
        for i in range(5):
            ch.append(Message.text("a", f"msg_{i}"))

        # Reader A is at offset 0
        batch_a = ch.read(0, 2)
        assert len(batch_a) == 2
        assert batch_a[0].payload_text == "msg_0"

        # Reader B is at offset 3
        batch_b = ch.read(3, 2)
        assert len(batch_b) == 2
        assert batch_b[0].payload_text == "msg_3"

        # Reader A reads again from offset 2 -- unaffected by B
        batch_a2 = ch.read(2, 2)
        assert len(batch_a2) == 2
        assert batch_a2[0].payload_text == "msg_2"


# ═══════════════════════════════════════════════════════════════
# Test 29: Append-only
# ═══════════════════════════════════════════════════════════════


class TestAppendOnly:
    """Test 29: Verify there is no method to delete or modify messages."""

    def test_no_delete_method(self) -> None:
        """Channel has no delete or remove method."""
        ch = Channel("ch1", "test")
        assert not hasattr(ch, "delete")
        assert not hasattr(ch, "remove")
        assert not hasattr(ch, "pop")
        assert not hasattr(ch, "clear")
        assert not hasattr(ch, "update")
        assert not hasattr(ch, "insert")


# ═══════════════════════════════════════════════════════════════
# Test 30: Binary persistence
# ═══════════════════════════════════════════════════════════════


class TestBinaryPersistence:
    """Test 30: Persist messages (text + binary), verify file format."""

    def test_file_starts_with_magic(self, tmp_path: Path) -> None:
        """Persisted file starts with ACTM magic bytes."""
        ch = Channel("ch1", "test")
        ch.append(Message.text("a", "hello"))
        ch.append(Message.binary("a", "image/png", b"\x89PNG\r\n\x1a\n"))
        ch.persist(str(tmp_path))

        log_file = tmp_path / "test.log"
        assert log_file.exists()
        with open(log_file, "rb") as f:
            assert f.read(4) == b"ACTM"

    def test_file_contains_correct_headers(self, tmp_path: Path) -> None:
        """Each message in the file has a valid header."""
        ch = Channel("ch1", "test")
        ch.append(Message.text("a", "hello"))
        ch.persist(str(tmp_path))

        log_file = tmp_path / "test.log"
        with open(log_file, "rb") as f:
            header = f.read(HEADER_SIZE)
            magic, version, env_len, pay_len = struct.unpack(
                HEADER_FORMAT, header
            )
            assert magic == WIRE_MAGIC
            assert version == 1
            assert env_len > 0
            assert pay_len == 5  # len(b"hello")


# ═══════════════════════════════════════════════════════════════
# Test 31: Recovery
# ═══════════════════════════════════════════════════════════════


class TestRecovery:
    """Test 31: Persist a channel, recover from disk, verify messages."""

    def test_full_recovery(self, tmp_path: Path) -> None:
        """All messages are restored including binary payloads."""
        ch = Channel("ch1", "test")
        ch.append(Message.text("a", "hello"))
        ch.append(Message.binary("a", "image/png", b"\x89PNG"))
        ch.persist(str(tmp_path))

        recovered = Channel.recover(str(tmp_path), "test")
        assert recovered.length() == 2
        assert recovered.read(0, 1)[0].payload_text == "hello"
        assert recovered.read(1, 1)[0].payload == b"\x89PNG"


# ═══════════════════════════════════════════════════════════════
# Test 32: Recovery preserves order
# ═══════════════════════════════════════════════════════════════


class TestRecoveryOrder:
    """Test 32: Persist 100 messages, recover, verify order matches."""

    def test_order_preserved(self, tmp_path: Path) -> None:
        """100 messages are recovered in the exact same order."""
        ch = Channel("ch1", "test")
        for i in range(100):
            ch.append(Message.text("a", f"msg_{i}"))
        ch.persist(str(tmp_path))

        recovered = Channel.recover(str(tmp_path), "test")
        assert recovered.length() == 100
        for i in range(100):
            assert recovered.read(i, 1)[0].payload_text == f"msg_{i}"


# ═══════════════════════════════════════════════════════════════
# Test 33: Empty channel recovery
# ═══════════════════════════════════════════════════════════════


class TestEmptyChannelRecovery:
    """Test 33: Recover from non-existent file, verify empty channel."""

    def test_missing_file_returns_empty(self, tmp_path: Path) -> None:
        """Recovering from a non-existent file returns an empty channel."""
        recovered = Channel.recover(str(tmp_path), "nonexistent")
        assert recovered.length() == 0
        assert recovered.name == "nonexistent"


# ═══════════════════════════════════════════════════════════════
# Test 34: Mixed content recovery
# ═══════════════════════════════════════════════════════════════


class TestMixedContentRecovery:
    """Test 34: Persist text, JSON, and binary; recover all types."""

    def test_mixed_content_types(self, tmp_path: Path) -> None:
        """All content types and payloads are correctly recovered."""
        ch = Channel("ch1", "mixed")
        ch.append(Message.text("a", "hello text"))
        ch.append(Message.json("b", {"key": "value"}))
        ch.append(Message.binary("c", "image/png", b"\x89PNG\r\n\x1a\n"))
        ch.persist(str(tmp_path))

        recovered = Channel.recover(str(tmp_path), "mixed")
        assert recovered.length() == 3

        msgs = recovered.read(0, 3)
        assert msgs[0].content_type == "text/plain"
        assert msgs[0].payload_text == "hello text"
        assert msgs[1].content_type == "application/json"
        assert msgs[1].payload_json == {"key": "value"}
        assert msgs[2].content_type == "image/png"
        assert msgs[2].payload == b"\x89PNG\r\n\x1a\n"


# ═══════════════════════════════════════════════════════════════
# Test 35: Truncated write recovery
# ═══════════════════════════════════════════════════════════════


class TestTruncatedWriteRecovery:
    """Test 35: Simulate crash mid-write, recover complete messages only."""

    def test_truncated_header(self, tmp_path: Path) -> None:
        """Truncated header at end of file is silently discarded."""
        ch = Channel("ch1", "test")
        ch.append(Message.text("a", "msg_0"))
        ch.append(Message.text("a", "msg_1"))
        ch.persist(str(tmp_path))

        # Append a partial header to simulate crash mid-write
        log_file = tmp_path / "test.log"
        with open(log_file, "ab") as f:
            f.write(b"ACTM\x01")  # Only 5 of 17 header bytes

        recovered = Channel.recover(str(tmp_path), "test")
        assert recovered.length() == 2
        assert recovered.read(0, 1)[0].payload_text == "msg_0"
        assert recovered.read(1, 1)[0].payload_text == "msg_1"

    def test_truncated_payload(self, tmp_path: Path) -> None:
        """Truncated payload at end of file is silently discarded."""
        ch = Channel("ch1", "test")
        ch.append(Message.text("a", "complete"))
        ch.persist(str(tmp_path))

        # Write a complete header but truncated payload for a second message
        log_file = tmp_path / "test.log"
        msg2 = Message.text("a", "this will be truncated")
        full_bytes = msg2.to_bytes()
        with open(log_file, "ab") as f:
            # Write everything except the last 5 bytes of payload
            f.write(full_bytes[:-5])

        recovered = Channel.recover(str(tmp_path), "test")
        assert recovered.length() == 1
        assert recovered.read(0, 1)[0].payload_text == "complete"


# ═══════════════════════════════════════════════════════════════
# Test 36: Mixed version recovery (hypothetical)
# ═══════════════════════════════════════════════════════════════


class TestMixedVersionRecovery:
    """Test 36: Verify reader handles current version correctly.

    Since we only have v1, this test verifies that v1 messages are
    correctly parsed. A true mixed-version test would require a v2
    implementation, which is a future extension.
    """

    def test_v1_messages_recovered(self, tmp_path: Path) -> None:
        """V1 messages are correctly parsed during recovery."""
        ch = Channel("ch1", "test")
        ch.append(Message.text("a", "v1_message_1"))
        ch.append(Message.text("b", "v1_message_2"))
        ch.persist(str(tmp_path))

        recovered = Channel.recover(str(tmp_path), "test")
        assert recovered.length() == 2
        msgs = recovered.read(0, 2)
        assert msgs[0].sender_id == "a"
        assert msgs[1].sender_id == "b"


# ═══════════════════════════════════════════════════════════════
# Additional edge cases
# ═══════════════════════════════════════════════════════════════


class TestChannelRepr:
    """Channel repr shows useful debugging info."""

    def test_repr(self) -> None:
        """repr includes id, name, and message count."""
        ch = Channel("ch_001", "greetings")
        r = repr(ch)
        assert "ch_001" in r
        assert "greetings" in r
        assert "messages=0" in r


class TestChannelPersistCreatesDirectory:
    """Persist creates the directory if it does not exist."""

    def test_creates_directory(self, tmp_path: Path) -> None:
        """Nested directory is created automatically."""
        nested = tmp_path / "a" / "b" / "c"
        ch = Channel("ch1", "test")
        ch.append(Message.text("a", "hello"))
        ch.persist(str(nested))
        assert (nested / "test.log").exists()
