"""Tests for ActorSystem -- integration tests and system-level behavior.

These tests verify the ActorSystem runtime: ping-pong between actors,
pipelines, fan-out, dynamic topology, channel-based communication,
persistence round-trips, and large-scale processing.
"""

from __future__ import annotations

import random
from pathlib import Path
from typing import Any

import pytest

from actor.actor import ActorResult, ActorSpec
from actor.actor_system import (
    ActorNotFoundError,
    ActorSystem,
    ChannelNotFoundError,
    DuplicateChannelError,
)
from actor.channel import Channel
from actor.message import Message

# ---------------------------------------------------------------------------
# Behavior functions for integration tests
# ---------------------------------------------------------------------------


def echo_behavior(state: Any, message: Message) -> ActorResult:
    """Echo back to sender."""
    reply = Message.text("echo", f"echo: {message.payload_text}")
    return ActorResult(state, [(message.sender_id, reply)])


def counter_behavior(state: Any, message: Message) -> ActorResult:
    """Increment counter."""
    return ActorResult(new_state=state + 1)


def sink_behavior(state: Any, message: Message) -> ActorResult:
    """Collect messages in state (a list)."""
    state.append(message.payload_text)
    return ActorResult(new_state=state)


def ping_behavior(state: Any, message: Message) -> ActorResult:
    """Ping: send pong to sender, increment count. Stop at 10."""
    count = state + 1
    if count >= 10:
        return ActorResult(new_state=count, stop=True)
    reply = Message.text("ping", f"ping_{count}")
    return ActorResult(new_state=count, messages_to_send=[("pong", reply)])


def pong_behavior(state: Any, message: Message) -> ActorResult:
    """Pong: send ping back to sender, increment count. Stop at 10."""
    count = state + 1
    if count >= 10:
        return ActorResult(new_state=count, stop=True)
    reply = Message.text("pong", f"pong_{count}")
    return ActorResult(new_state=count, messages_to_send=[("ping", reply)])


def transformer_behavior(state: Any, message: Message) -> ActorResult:
    """Transform message and forward to the next actor in the pipeline."""
    transformed = f"[transformed] {message.payload_text}"
    forward = Message.text("transformer", transformed)
    return ActorResult(
        new_state=state,
        messages_to_send=[(state, forward)],  # state holds the next actor id
    )


def fanout_behavior(state: Any, message: Message) -> ActorResult:
    """Send the same message to all target actors (stored in state)."""
    targets: list[str] = state
    messages = [
        (target, Message.text("fanout", message.payload_text))
        for target in targets
    ]
    return ActorResult(new_state=state, messages_to_send=messages)


def noop_behavior(state: Any, message: Message) -> ActorResult:
    """Do nothing."""
    return ActorResult(new_state=state)


# ═══════════════════════════════════════════════════════════════
# Test 50: Ping-pong
# ═══════════════════════════════════════════════════════════════


class TestPingPong:
    """Test 50: Two actors send messages back and forth 10 times."""

    def test_ping_pong_reaches_idle(self) -> None:
        """Both actors process 10 messages and system reaches idle."""
        system = ActorSystem()
        system.create_actor("ping", 0, ping_behavior)
        system.create_actor("pong", 0, pong_behavior)

        # Kick off with a message to ping
        system.send("ping", Message.text("pong", "start"))
        stats = system.run_until_done()

        assert stats["messages_processed"] > 0
        # Both actors should have processed messages
        ping_state = system._actors["ping"].state
        pong_state = system._actors["pong"].state
        assert ping_state == 10
        assert pong_state >= 9  # pong processes at least 9


# ═══════════════════════════════════════════════════════════════
# Test 51: Pipeline
# ═══════════════════════════════════════════════════════════════


class TestPipeline:
    """Test 51: Three actors in a chain: A -> B -> C."""

    def test_pipeline_transforms(self) -> None:
        """Message flows through pipeline and C receives transformed data."""
        system = ActorSystem()

        # B transforms and forwards to C
        system.create_actor("B", "C", transformer_behavior)
        # C collects messages
        system.create_actor("C", [], sink_behavior)
        # A sends to B
        system.create_actor("A", "B", transformer_behavior)

        system.send("A", Message.text("user", "hello"))
        system.run_until_done()

        c_state = system._actors["C"].state
        assert len(c_state) == 1
        assert "transformed" in c_state[0]
        assert "hello" in c_state[0]


# ═══════════════════════════════════════════════════════════════
# Test 52: Channel-based pipeline
# ═══════════════════════════════════════════════════════════════


class TestChannelPipeline:
    """Test 52: Producer writes to channel, consumer reads from it."""

    def test_channel_communication(self) -> None:
        """Consumer reads all messages from channel in order."""
        system = ActorSystem()
        channel = system.create_channel("ch_001", "greetings")

        # Producer writes to channel
        msg1 = Message.text("producer", "hello")
        msg2 = Message.text("producer", "world")
        channel.append(msg1)
        channel.append(msg2)

        # Consumer reads from channel
        offset = 0
        batch = channel.read(offset, 10)
        assert len(batch) == 2
        assert batch[0].payload_text == "hello"
        assert batch[1].payload_text == "world"
        offset += len(batch)

        # More messages arrive
        channel.append(Message.text("producer", "!"))
        batch2 = channel.read(offset, 10)
        assert len(batch2) == 1
        assert batch2[0].payload_text == "!"


# ═══════════════════════════════════════════════════════════════
# Test 53: Fan-out
# ═══════════════════════════════════════════════════════════════


class TestFanOut:
    """Test 53: One actor sends the same message to 5 actors."""

    def test_all_targets_receive(self) -> None:
        """All 5 target actors receive and process the message."""
        system = ActorSystem()

        targets = [f"worker_{i}" for i in range(5)]
        for target in targets:
            system.create_actor(target, [], sink_behavior)

        system.create_actor("fanout", targets, fanout_behavior)
        system.send("fanout", Message.text("user", "broadcast"))
        system.run_until_done()

        for target in targets:
            state = system._actors[target].state
            assert len(state) == 1
            assert state[0] == "broadcast"


# ═══════════════════════════════════════════════════════════════
# Test 54: Dynamic topology
# ═══════════════════════════════════════════════════════════════


class TestDynamicTopology:
    """Test 54: Actor A spawns Actor B, sends B a message, B responds."""

    def test_spawn_and_communicate(self) -> None:
        """Full round-trip with a dynamically created actor."""

        def spawner(state: Any, message: Message) -> ActorResult:
            if message.payload_text == "spawn":
                return ActorResult(
                    new_state="spawned",
                    actors_to_create=[
                        ActorSpec("dynamic_echo", None, echo_behavior)
                    ],
                    messages_to_send=[
                        (
                            "dynamic_echo",
                            Message.text("spawner_actor", "hello dynamic"),
                        )
                    ],
                )
            return ActorResult(new_state=state)

        system = ActorSystem()
        system.create_actor("spawner_actor", None, spawner)
        system.send("spawner_actor", Message.text("user", "spawn"))
        system.run_until_done()

        assert "dynamic_echo" in system.actor_ids()
        # The echo should have sent a reply to spawner_actor
        assert system._actors["spawner_actor"].state == "spawned"


# ═══════════════════════════════════════════════════════════════
# Test 55: Run until idle
# ═══════════════════════════════════════════════════════════════


class TestRunUntilIdle:
    """Test 55: Complex network of 5 actors with interconnected messaging."""

    def test_all_messages_processed(self) -> None:
        """All messages are processed and system becomes quiet."""
        system = ActorSystem()

        # Create 5 counter actors
        for i in range(5):
            system.create_actor(f"actor_{i}", 0, counter_behavior)

        # Send messages to each actor
        for i in range(5):
            for j in range(3):
                system.send(
                    f"actor_{i}", Message.text("user", f"msg_{j}")
                )

        stats = system.run_until_idle()
        assert stats["messages_processed"] == 15

        # All mailboxes should be empty
        for i in range(5):
            assert system.mailbox_size(f"actor_{i}") == 0


# ═══════════════════════════════════════════════════════════════
# Test 56: Persistence round-trip
# ═══════════════════════════════════════════════════════════════


class TestPersistenceRoundTrip:
    """Test 56: Create actors, send messages through channels, persist, recover."""

    def test_channel_persistence_with_binary(self, tmp_path: Path) -> None:
        """Channel with binary payloads survives persistence round-trip."""
        system = ActorSystem()
        channel = system.create_channel("ch_001", "media")

        # Add various message types
        channel.append(Message.text("agent", "hello text"))
        channel.append(Message.json("agent", {"key": "value", "count": 42}))
        channel.append(
            Message.binary(
                "agent", "image/png", b"\x89PNG\r\n\x1a\n" + b"\x00" * 256
            )
        )

        channel.persist(str(tmp_path))

        # Recover in a fresh context (simulating new system startup)
        recovered = Channel.recover(str(tmp_path), "media")

        assert recovered.length() == 3
        msgs = recovered.read(0, 3)
        assert msgs[0].payload_text == "hello text"
        assert msgs[1].payload_json == {"key": "value", "count": 42}
        assert msgs[2].payload[:4] == b"\x89PNG"
        assert len(msgs[2].payload) == 8 + 256


# ═══════════════════════════════════════════════════════════════
# Test 57: Large-scale
# ═══════════════════════════════════════════════════════════════


class TestLargeScale:
    """Test 57: 100 actors, 1000 messages, verify nothing is lost."""

    def test_no_messages_lost(self) -> None:
        """All messages are either delivered or in dead_letters."""
        system = ActorSystem()

        # Create 100 counter actors
        for i in range(100):
            system.create_actor(f"actor_{i}", 0, counter_behavior)

        # Send 1000 messages randomly
        random.seed(42)  # Deterministic for reproducibility
        for _ in range(1000):
            target = f"actor_{random.randint(0, 99)}"
            system.send(target, Message.text("user", "tick"))

        stats = system.run_until_done()
        assert stats["messages_processed"] == 1000

        # Verify total: all messages processed + dead letters = 1000
        total_processed = sum(
            system._actors[f"actor_{i}"].state for i in range(100)
        )
        assert total_processed == 1000
        assert len(system.dead_letters) == 0


# ═══════════════════════════════════════════════════════════════
# Test 58: Binary message pipeline
# ═══════════════════════════════════════════════════════════════


class TestBinaryMessagePipeline:
    """Test 58: Actor A sends PNG image to Actor B via channel."""

    def test_binary_through_channel(self, tmp_path: Path) -> None:
        """Binary payload is identical after channel round-trip."""
        system = ActorSystem()
        channel = system.create_channel("ch_img", "images")

        # Simulate PNG image data
        png_data = b"\x89PNG\r\n\x1a\n" + bytes(range(256)) * 10

        # Actor A writes to channel
        msg = Message.binary("actor_a", "image/png", png_data)
        channel.append(msg)

        # Persist and recover
        channel.persist(str(tmp_path))
        recovered = Channel.recover(str(tmp_path), "images")

        # Actor B reads from recovered channel
        messages = recovered.read(0, 1)
        assert len(messages) == 1
        assert messages[0].payload == png_data
        assert messages[0].content_type == "image/png"


# ═══════════════════════════════════════════════════════════════
# Additional ActorSystem tests
# ═══════════════════════════════════════════════════════════════


class TestActorSystemErrors:
    """Test error handling in ActorSystem."""

    def test_stop_nonexistent_actor(self) -> None:
        """Stopping a non-existent actor raises ActorNotFoundError."""
        system = ActorSystem()
        with pytest.raises(ActorNotFoundError):
            system.stop_actor("nonexistent")

    def test_get_status_nonexistent(self) -> None:
        """Getting status of non-existent actor raises ActorNotFoundError."""
        system = ActorSystem()
        with pytest.raises(ActorNotFoundError):
            system.get_actor_status("nonexistent")

    def test_process_nonexistent(self) -> None:
        """Processing non-existent actor raises ActorNotFoundError."""
        system = ActorSystem()
        with pytest.raises(ActorNotFoundError):
            system.process_next("nonexistent")

    def test_process_stopped_actor(self) -> None:
        """Processing a stopped actor raises ActorNotFoundError."""
        system = ActorSystem()
        system.create_actor("actor1", None, noop_behavior)
        system.stop_actor("actor1")
        with pytest.raises(ActorNotFoundError):
            system.process_next("actor1")

    def test_mailbox_size_nonexistent(self) -> None:
        """Getting mailbox size of non-existent actor raises error."""
        system = ActorSystem()
        with pytest.raises(ActorNotFoundError):
            system.mailbox_size("nonexistent")


class TestActorSystemChannels:
    """Test channel management in ActorSystem."""

    def test_create_and_get_channel(self) -> None:
        """Created channel is retrievable by ID."""
        system = ActorSystem()
        ch = system.create_channel("ch_001", "test")
        assert ch.name == "test"
        retrieved = system.get_channel("ch_001")
        assert retrieved.name == "test"

    def test_duplicate_channel_raises(self) -> None:
        """Creating a channel with duplicate ID raises error."""
        system = ActorSystem()
        system.create_channel("ch_001", "test")
        with pytest.raises(DuplicateChannelError):
            system.create_channel("ch_001", "test2")

    def test_get_nonexistent_channel(self) -> None:
        """Getting a non-existent channel raises error."""
        system = ActorSystem()
        with pytest.raises(ChannelNotFoundError):
            system.get_channel("nonexistent")


class TestActorSystemShutdown:
    """Test shutdown behavior."""

    def test_shutdown_stops_all(self) -> None:
        """Shutdown stops all actors and drains mailboxes."""
        system = ActorSystem()
        system.create_actor("a1", None, noop_behavior)
        system.create_actor("a2", None, noop_behavior)
        system.send("a1", Message.text("user", "msg1"))
        system.send("a2", Message.text("user", "msg2"))

        system.shutdown()

        assert system.get_actor_status("a1") == "stopped"
        assert system.get_actor_status("a2") == "stopped"
        assert len(system.dead_letters) == 2

    def test_shutdown_idempotent(self) -> None:
        """Shutting down twice does not cause errors."""
        system = ActorSystem()
        system.create_actor("a1", None, noop_behavior)
        system.shutdown()
        system.shutdown()  # Should not raise


class TestActorSystemRepr:
    """ActorSystem repr shows useful info."""

    def test_repr(self) -> None:
        """repr includes actor, channel, and dead_letters counts."""
        system = ActorSystem()
        system.create_actor("a1", None, noop_behavior)
        system.create_channel("ch1", "test")
        r = repr(system)
        assert "actors=1" in r
        assert "channels=1" in r
        assert "dead_letters=0" in r


class TestActorSystemActorIds:
    """Test actor_ids listing."""

    def test_lists_all_actors(self) -> None:
        """actor_ids returns all registered actor IDs."""
        system = ActorSystem()
        system.create_actor("a", None, noop_behavior)
        system.create_actor("b", None, noop_behavior)
        ids = system.actor_ids()
        assert "a" in ids
        assert "b" in ids
        assert len(ids) == 2


class TestStopDrainsMailboxOnBehaviorStop:
    """When behavior returns stop=True, remaining mailbox drains."""

    def test_drain_on_behavior_stop(self) -> None:
        """Remaining messages go to dead_letters when behavior stops."""
        system = ActorSystem()

        def stop_on_stop(state: Any, message: Message) -> ActorResult:
            if message.payload_text == "stop":
                return ActorResult(new_state=state, stop=True)
            return ActorResult(new_state=state)

        system.create_actor("actor1", None, stop_on_stop)
        system.send("actor1", Message.text("user", "stop"))
        system.send("actor1", Message.text("user", "after_stop"))

        system.process_next("actor1")
        # The second message should be in dead_letters
        assert len(system.dead_letters) == 1
        assert system.dead_letters[0].payload_text == "after_stop"
