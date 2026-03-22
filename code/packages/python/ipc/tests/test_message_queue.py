"""Tests for MessageQueue -- the typed message FIFO.

These tests verify:
1. Send/receive in FIFO order
2. Typed receive (filter by message type)
3. Full queue rejection
4. Oversized message rejection
5. Invalid message type rejection
6. Empty queue behavior
7. Properties (message_count, is_empty, is_full, max_messages, max_message_size)
"""

from ipc import MessageQueue

# ========================================================================
# Basic send/receive
# ========================================================================


class TestMessageQueueBasicIO:
    """Send messages in, receive them out -- FIFO order."""

    def test_send_and_receive(self) -> None:
        """Send a message, receive it back."""
        mq = MessageQueue()
        assert mq.send(1, b"hello") is True
        result = mq.receive()
        assert result == (1, b"hello")

    def test_fifo_order(self) -> None:
        """Messages come out in the order they went in."""
        mq = MessageQueue()
        mq.send(1, b"first")
        mq.send(1, b"second")
        mq.send(1, b"third")
        assert mq.receive() == (1, b"first")
        assert mq.receive() == (1, b"second")
        assert mq.receive() == (1, b"third")

    def test_receive_returns_none_when_empty(self) -> None:
        """Receiving from an empty queue returns None."""
        mq = MessageQueue()
        assert mq.receive() is None

    def test_send_returns_true_on_success(self) -> None:
        mq = MessageQueue()
        assert mq.send(1, b"data") is True


# ========================================================================
# Typed receive (filtering)
# ========================================================================


class TestMessageQueueTypedReceive:
    """Receive messages filtered by type -- the key differentiator from pipes."""

    def test_receive_specific_type(self) -> None:
        """Receive only messages of a specific type, skipping others.

        Queue: (1, 'req1'), (2, 'status'), (1, 'req2')
        Receive type=2 => (2, 'status')
        Queue is now: (1, 'req1'), (1, 'req2')
        """
        mq = MessageQueue()
        mq.send(1, b"req1")
        mq.send(2, b"status")
        mq.send(1, b"req2")

        result = mq.receive(msg_type=2)
        assert result == (2, b"status")
        assert mq.message_count == 2

    def test_receive_type_zero_means_any(self) -> None:
        """Type 0 means 'give me the oldest message of any type.'"""
        mq = MessageQueue()
        mq.send(3, b"three")
        mq.send(1, b"one")
        result = mq.receive(msg_type=0)
        assert result == (3, b"three")

    def test_receive_nonexistent_type(self) -> None:
        """Requesting a type with no messages returns None."""
        mq = MessageQueue()
        mq.send(1, b"data")
        assert mq.receive(msg_type=99) is None
        assert mq.message_count == 1  # original message untouched

    def test_typed_receive_preserves_order(self) -> None:
        """Plucking a typed message leaves other messages in order."""
        mq = MessageQueue()
        mq.send(1, b"a")
        mq.send(2, b"b")
        mq.send(1, b"c")
        mq.send(3, b"d")

        # Remove the type-2 message
        mq.receive(msg_type=2)

        # Remaining: (1, a), (1, c), (3, d)
        assert mq.receive() == (1, b"a")
        assert mq.receive() == (1, b"c")
        assert mq.receive() == (3, b"d")

    def test_receive_oldest_of_matching_type(self) -> None:
        """When multiple messages match, the oldest one is returned."""
        mq = MessageQueue()
        mq.send(1, b"old")
        mq.send(1, b"new")
        result = mq.receive(msg_type=1)
        assert result == (1, b"old")


# ========================================================================
# Capacity and size limits
# ========================================================================


class TestMessageQueueLimits:
    """Test queue full, oversized message, and invalid type rejection."""

    def test_full_queue_rejects_send(self) -> None:
        """When queue is at max_messages, send() returns False."""
        mq = MessageQueue(max_messages=3)
        assert mq.send(1, b"a") is True
        assert mq.send(1, b"b") is True
        assert mq.send(1, b"c") is True
        assert mq.send(1, b"d") is False  # queue full
        assert mq.message_count == 3

    def test_oversized_message_rejected(self) -> None:
        """Messages larger than max_message_size are rejected."""
        mq = MessageQueue(max_message_size=8)
        assert mq.send(1, b"short") is True
        assert mq.send(1, b"this is way too long") is False
        assert mq.message_count == 1

    def test_exactly_max_size_accepted(self) -> None:
        """A message exactly at max_message_size is accepted."""
        mq = MessageQueue(max_message_size=5)
        assert mq.send(1, b"exact") is True

    def test_invalid_message_type_zero(self) -> None:
        """msg_type must be positive; 0 is rejected."""
        mq = MessageQueue()
        assert mq.send(0, b"data") is False

    def test_invalid_message_type_negative(self) -> None:
        """Negative msg_type is rejected."""
        mq = MessageQueue()
        assert mq.send(-1, b"data") is False


# ========================================================================
# Properties
# ========================================================================


class TestMessageQueueProperties:
    """Test state-query properties."""

    def test_message_count(self) -> None:
        mq = MessageQueue()
        assert mq.message_count == 0
        mq.send(1, b"a")
        assert mq.message_count == 1
        mq.send(2, b"b")
        assert mq.message_count == 2
        mq.receive()
        assert mq.message_count == 1

    def test_is_empty(self) -> None:
        mq = MessageQueue()
        assert mq.is_empty
        mq.send(1, b"x")
        assert not mq.is_empty

    def test_is_full(self) -> None:
        mq = MessageQueue(max_messages=2)
        assert not mq.is_full
        mq.send(1, b"a")
        assert not mq.is_full
        mq.send(1, b"b")
        assert mq.is_full

    def test_max_messages_property(self) -> None:
        mq = MessageQueue(max_messages=100)
        assert mq.max_messages == 100

    def test_max_message_size_property(self) -> None:
        mq = MessageQueue(max_message_size=512)
        assert mq.max_message_size == 512

    def test_default_limits(self) -> None:
        mq = MessageQueue()
        assert mq.max_messages == 256
        assert mq.max_message_size == 4096
