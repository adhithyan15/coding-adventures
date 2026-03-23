"""Tests for Actor -- isolated computation with mailbox and behavior.

These tests verify actor creation, message processing, state management,
the stop lifecycle, and error handling.
"""

from __future__ import annotations

from typing import Any

import pytest

from actor.actor import IDLE, STOPPED, Actor, ActorResult, ActorSpec
from actor.actor_system import (
    ActorSystem,
    DuplicateActorError,
)
from actor.message import Message

# ---------------------------------------------------------------------------
# Behavior functions used across tests
# ---------------------------------------------------------------------------


def echo_behavior(state: Any, message: Message) -> ActorResult:
    """Echo: send the same message back to whoever sent it."""
    reply = Message.text(
        sender_id="echo",
        payload=f"echo: {message.payload_text}",
    )
    return ActorResult(
        new_state=state,
        messages_to_send=[(message.sender_id, reply)],
    )


def counter_behavior(state: Any, message: Message) -> ActorResult:
    """Count messages received."""
    return ActorResult(new_state=state + 1)


def stopper_behavior(state: Any, message: Message) -> ActorResult:
    """Stop after receiving any message."""
    return ActorResult(new_state=state, stop=True)


def exploding_behavior(state: Any, message: Message) -> ActorResult:
    """Raise an exception on messages containing 'boom'."""
    if message.payload_text == "boom":
        raise ValueError("BOOM!")
    return ActorResult(new_state=state + 1)


def spawner_behavior(state: Any, message: Message) -> ActorResult:
    """When told to spawn, create a new echo actor."""
    if message.payload_text == "spawn":
        new_id = f"echo_{state}"
        return ActorResult(
            new_state=state + 1,
            actors_to_create=[
                ActorSpec(
                    actor_id=new_id,
                    initial_state=None,
                    behavior=echo_behavior,
                ),
            ],
            messages_to_send=[
                (
                    message.sender_id,
                    Message.text("spawner", f"spawned {new_id}"),
                )
            ],
        )
    return ActorResult(new_state=state)


# ═══════════════════════════════════════════════════════════════
# Test 37: Create actor
# ═══════════════════════════════════════════════════════════════


class TestCreateActor:
    """Test 37: Create actor with initial state, verify status is IDLE."""

    def test_initial_status_idle(self) -> None:
        """Newly created actor starts in IDLE status."""
        system = ActorSystem()
        system.create_actor("counter", 0, counter_behavior)
        assert system.get_actor_status("counter") == IDLE

    def test_initial_mailbox_empty(self) -> None:
        """Newly created actor has empty mailbox."""
        system = ActorSystem()
        system.create_actor("counter", 0, counter_behavior)
        assert system.mailbox_size("counter") == 0


# ═══════════════════════════════════════════════════════════════
# Test 38: Send message
# ═══════════════════════════════════════════════════════════════


class TestSendMessage:
    """Test 38: Send message to actor, verify mailbox_size is 1."""

    def test_message_enqueued(self) -> None:
        """Sending a message increases the mailbox size."""
        system = ActorSystem()
        system.create_actor("counter", 0, counter_behavior)
        system.send("counter", Message.text("user", "hello"))
        assert system.mailbox_size("counter") == 1


# ═══════════════════════════════════════════════════════════════
# Test 39: Process message
# ═══════════════════════════════════════════════════════════════


class TestProcessMessage:
    """Test 39: Send message, call process_next, verify behavior was called."""

    def test_process_returns_true(self) -> None:
        """process_next returns True when a message was processed."""
        system = ActorSystem()
        system.create_actor("counter", 0, counter_behavior)
        system.send("counter", Message.text("user", "hello"))
        assert system.process_next("counter") is True

    def test_process_empty_returns_false(self) -> None:
        """process_next returns False when mailbox is empty."""
        system = ActorSystem()
        system.create_actor("counter", 0, counter_behavior)
        assert system.process_next("counter") is False


# ═══════════════════════════════════════════════════════════════
# Test 40: State update
# ═══════════════════════════════════════════════════════════════


class TestStateUpdate:
    """Test 40: Counter actor counts 3 messages correctly."""

    def test_state_increments(self) -> None:
        """State is updated after each message processing."""
        system = ActorSystem()
        system.create_actor("counter", 0, counter_behavior)
        for _ in range(3):
            system.send("counter", Message.text("user", "tick"))
        for _ in range(3):
            system.process_next("counter")
        # Access internal state via the actor object
        actor = system._actors["counter"]
        assert actor.state == 3


# ═══════════════════════════════════════════════════════════════
# Test 41: Messages to send
# ═══════════════════════════════════════════════════════════════


class TestMessagesToSend:
    """Test 41: Echo actor sends reply to sender's mailbox."""

    def test_reply_delivered(self) -> None:
        """Echo actor's reply ends up in sender's mailbox."""
        system = ActorSystem()
        system.create_actor("echo", None, echo_behavior)
        system.create_actor("user", None, counter_behavior)
        system.send("echo", Message.text("user", "hello"))
        system.process_next("echo")
        assert system.mailbox_size("user") == 1


# ═══════════════════════════════════════════════════════════════
# Test 42: Actor creation
# ═══════════════════════════════════════════════════════════════


class TestActorCreation:
    """Test 42: Spawner actor creates new actor in the system."""

    def test_spawned_actor_exists(self) -> None:
        """New actor is registered in the system after spawning."""
        system = ActorSystem()
        system.create_actor("spawner", 0, spawner_behavior)
        system.create_actor("user", None, counter_behavior)
        system.send("spawner", Message.text("user", "spawn"))
        system.process_next("spawner")
        assert "echo_0" in system.actor_ids()
        assert system.get_actor_status("echo_0") == IDLE


# ═══════════════════════════════════════════════════════════════
# Test 43: Stop actor
# ═══════════════════════════════════════════════════════════════


class TestStopActor:
    """Test 43: Send stop message, verify status is STOPPED."""

    def test_stop_via_behavior(self) -> None:
        """Actor that returns stop=True becomes STOPPED."""
        system = ActorSystem()
        system.create_actor("stopper", None, stopper_behavior)
        system.send("stopper", Message.text("user", "stop"))
        system.process_next("stopper")
        assert system.get_actor_status("stopper") == STOPPED

    def test_stop_via_system(self) -> None:
        """stop_actor sets status to STOPPED."""
        system = ActorSystem()
        system.create_actor("actor1", None, counter_behavior)
        system.stop_actor("actor1")
        assert system.get_actor_status("actor1") == STOPPED


# ═══════════════════════════════════════════════════════════════
# Test 44: Stopped actor rejects messages
# ═══════════════════════════════════════════════════════════════


class TestStoppedActorRejectsMessages:
    """Test 44: Messages to stopped actors go to dead_letters."""

    def test_dead_letter_on_stopped(self) -> None:
        """Message sent to stopped actor ends up in dead_letters."""
        system = ActorSystem()
        system.create_actor("actor1", None, counter_behavior)
        system.stop_actor("actor1")
        msg = Message.text("user", "hello")
        system.send("actor1", msg)
        assert len(system.dead_letters) > 0
        # The last dead letter should be our message
        # (there may be drained mailbox messages before it)
        assert any(dl.id == msg.id for dl in system.dead_letters)


# ═══════════════════════════════════════════════════════════════
# Test 45: Dead letters
# ═══════════════════════════════════════════════════════════════


class TestDeadLetters:
    """Test 45: Messages to non-existent actors go to dead_letters."""

    def test_nonexistent_target(self) -> None:
        """Message to non-existent actor goes to dead_letters."""
        system = ActorSystem()
        msg = Message.text("user", "hello")
        system.send("nonexistent", msg)
        assert len(system.dead_letters) == 1
        assert system.dead_letters[0].id == msg.id


# ═══════════════════════════════════════════════════════════════
# Test 46: Sequential processing
# ═══════════════════════════════════════════════════════════════


class TestSequentialProcessing:
    """Test 46: 3 messages processed in FIFO order."""

    def test_fifo_order(self) -> None:
        """Messages are processed in the order they were sent."""

        def tracking_behavior(
            state: list[str], message: Message
        ) -> ActorResult:
            state.append(message.payload_text)
            return ActorResult(new_state=state)

        system = ActorSystem()
        system.create_actor("tracker", [], tracking_behavior)
        system.send("tracker", Message.text("user", "first"))
        system.send("tracker", Message.text("user", "second"))
        system.send("tracker", Message.text("user", "third"))

        system.process_next("tracker")
        system.process_next("tracker")
        system.process_next("tracker")

        actor = system._actors["tracker"]
        assert actor.state == ["first", "second", "third"]


# ═══════════════════════════════════════════════════════════════
# Test 47: Mailbox drains on stop
# ═══════════════════════════════════════════════════════════════


class TestMailboxDrainOnStop:
    """Test 47: Pending messages go to dead_letters when actor stops."""

    def test_drain_to_dead_letters(self) -> None:
        """All pending messages move to dead_letters on stop."""
        system = ActorSystem()
        system.create_actor("actor1", None, counter_behavior)
        system.send("actor1", Message.text("user", "msg1"))
        system.send("actor1", Message.text("user", "msg2"))
        system.send("actor1", Message.text("user", "msg3"))
        assert system.mailbox_size("actor1") == 3

        system.stop_actor("actor1")
        assert system.mailbox_size("actor1") == 0
        assert len(system.dead_letters) == 3


# ═══════════════════════════════════════════════════════════════
# Test 48: Behavior exception
# ═══════════════════════════════════════════════════════════════


class TestBehaviorException:
    """Test 48: Exception in behavior: state unchanged, dead letter, continue."""

    def test_state_unchanged_after_error(self) -> None:
        """State is not updated when behavior throws."""
        system = ActorSystem()
        system.create_actor("exploder", 0, exploding_behavior)
        system.send("exploder", Message.text("user", "ok"))
        system.process_next("exploder")
        assert system._actors["exploder"].state == 1

        system.send("exploder", Message.text("user", "boom"))
        system.process_next("exploder")
        # State should still be 1 (error message did not increment)
        assert system._actors["exploder"].state == 1

    def test_message_goes_to_dead_letters(self) -> None:
        """Failed message goes to dead_letters."""
        system = ActorSystem()
        system.create_actor("exploder", 0, exploding_behavior)
        system.send("exploder", Message.text("user", "boom"))
        system.process_next("exploder")
        assert len(system.dead_letters) == 1

    def test_actor_continues_processing(self) -> None:
        """Actor returns to IDLE and can process more messages."""
        system = ActorSystem()
        system.create_actor("exploder", 0, exploding_behavior)
        system.send("exploder", Message.text("user", "boom"))
        system.process_next("exploder")
        assert system.get_actor_status("exploder") == IDLE

        # Can still process normal messages
        system.send("exploder", Message.text("user", "ok"))
        system.process_next("exploder")
        assert system._actors["exploder"].state == 1


# ═══════════════════════════════════════════════════════════════
# Test 49: Duplicate actor ID
# ═══════════════════════════════════════════════════════════════


class TestDuplicateActorId:
    """Test 49: Creating two actors with the same ID raises error."""

    def test_duplicate_raises(self) -> None:
        """DuplicateActorError is raised for duplicate IDs."""
        system = ActorSystem()
        system.create_actor("actor1", None, counter_behavior)
        with pytest.raises(DuplicateActorError):
            system.create_actor("actor1", None, counter_behavior)


# ═══════════════════════════════════════════════════════════════
# Additional Actor unit tests
# ═══════════════════════════════════════════════════════════════


class TestActorDirectCreation:
    """Test Actor class directly (not through ActorSystem)."""

    def test_actor_properties(self) -> None:
        """Actor stores id, state, behavior correctly."""
        actor = Actor("test_1", state=42, behavior=counter_behavior)
        assert actor.id == "test_1"
        assert actor.state == 42
        assert actor.status == IDLE
        assert actor.mailbox_size == 0

    def test_enqueue_dequeue(self) -> None:
        """Messages are enqueued and dequeued in FIFO order."""
        actor = Actor("test_1", state=None, behavior=counter_behavior)
        msg1 = Message.text("a", "first")
        msg2 = Message.text("a", "second")
        actor.enqueue(msg1)
        actor.enqueue(msg2)
        assert actor.mailbox_size == 2
        assert actor.dequeue() == msg1
        assert actor.dequeue() == msg2
        assert actor.dequeue() is None

    def test_drain_mailbox(self) -> None:
        """drain_mailbox returns all messages and empties the mailbox."""
        actor = Actor("test_1", state=None, behavior=counter_behavior)
        for i in range(3):
            actor.enqueue(Message.text("a", f"msg_{i}"))
        drained = actor.drain_mailbox()
        assert len(drained) == 3
        assert actor.mailbox_size == 0

    def test_repr(self) -> None:
        """repr includes id, status, and mailbox size."""
        actor = Actor("test_1", state=None, behavior=counter_behavior)
        r = repr(actor)
        assert "test_1" in r
        assert "idle" in r


class TestActorResultDefaults:
    """ActorResult has sensible defaults."""

    def test_defaults(self) -> None:
        """Default messages_to_send and actors_to_create are empty lists."""
        result = ActorResult(new_state=42)
        assert result.new_state == 42
        assert result.messages_to_send == []
        assert result.actors_to_create == []
        assert result.stop is False


class TestActorSpec:
    """ActorSpec stores all fields."""

    def test_fields(self) -> None:
        """ActorSpec stores actor_id, initial_state, and behavior."""
        spec = ActorSpec("worker", 0, counter_behavior)
        assert spec.actor_id == "worker"
        assert spec.initial_state == 0
        assert spec.behavior is counter_behavior
