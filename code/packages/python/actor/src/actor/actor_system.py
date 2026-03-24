"""ActorSystem -- the runtime that manages actors, message delivery, and channels.

=== What Is an ActorSystem? ===

The ActorSystem is the office building. It has:

- A directory (which actors exist and their addresses)
- A mail room (message routing)
- A building manager (supervision -- restart actors that crash)

Actors are tenants. They register with the building, get an address, and the
building delivers their mail. But the building manager does not read the mail.

=== Operations ===

The ActorSystem provides five categories of operations:

1. Actor lifecycle: create_actor, stop_actor, get_actor_status
2. Messaging: send (enqueue a message in an actor's mailbox)
3. Processing: process_next (run one message through one actor's behavior),
   run_until_idle (process all actors round-robin until no work remains),
   run_until_done (like run_until_idle but keeps going until fully quiet)
4. Channels: create_channel, get_channel
5. Inspection: dead_letters, actor_ids, mailbox_size

=== Processing Model ===

In V1, actors are processed sequentially in round-robin order. The system
picks an actor with a non-empty mailbox, processes one message, then moves
to the next actor. This is simpler to test and debug than true parallelism.

    Round-robin processing:

    Actors:   [A: 3 msgs] [B: 1 msg] [C: 2 msgs]

    Step 1: Process A's first message  -> [A: 2] [B: 1] [C: 2]
    Step 2: Process B's first message  -> [A: 2] [B: 0] [C: 2]
    Step 3: Process C's first message  -> [A: 2] [B: 0] [C: 1]
    Step 4: Process A's second message -> [A: 1] [B: 0] [C: 1]
    Step 5: Process C's second message -> [A: 1] [B: 0] [C: 0]
    Step 6: Process A's third message  -> [A: 0] [B: 0] [C: 0]
    Done: all mailboxes empty.

=== Dead Letters ===

When a message cannot be delivered (the target actor does not exist or is
stopped), the message goes to the dead_letters list. This is a debugging
aid -- it tells you "these messages were sent but never processed."

=== Error Handling ===

If an actor's behavior function throws an exception during processing:

1. The error is logged
2. The message that caused the error goes to dead_letters
3. The actor's state is NOT changed (no partial updates)
4. The actor returns to IDLE status (continues processing next messages)
5. The system does NOT crash -- one misbehaving actor does not bring down
   the entire system
"""

from __future__ import annotations

import logging
from typing import TYPE_CHECKING, Any

from actor.actor import IDLE, STOPPED, Actor, ActorResult
from actor.channel import Channel

if TYPE_CHECKING:
    from collections.abc import Callable

    from actor.message import Message

logger = logging.getLogger(__name__)


# ---------------------------------------------------------------------------
# Exceptions
# ---------------------------------------------------------------------------


class ActorNotFoundError(Exception):
    """Raised when an operation targets a non-existent actor."""


class DuplicateActorError(Exception):
    """Raised when creating an actor with an id that already exists."""


class DuplicateChannelError(Exception):
    """Raised when creating a channel with an id that already exists."""


class ChannelNotFoundError(Exception):
    """Raised when getting a channel that does not exist."""


# ---------------------------------------------------------------------------
# ActorSystem
# ---------------------------------------------------------------------------


class ActorSystem:
    """Runtime for managing actors, message delivery, and channels.

    The ActorSystem is the "world" that actors live in. It manages:

    - Actor registration and lifecycle (create, stop, status)
    - Message delivery (send to an actor's mailbox)
    - Processing (run behavior functions, one message at a time)
    - Channels (create and retrieve append-only message logs)
    - Dead letters (messages that could not be delivered)

    === Quick Start ===

        >>> from actor.message import Message
        >>> from actor.actor import ActorResult
        >>>
        >>> def echo(state, message):
        ...     reply = Message.text("echo", f"echo: {message.payload_text}")
        ...     return ActorResult(state, [(message.sender_id, reply)])
        ...
        >>> system = ActorSystem()
        >>> system.create_actor("echo", None, echo)
        'echo'
        >>> system.send("echo", Message.text("user", "hello"))
        >>> system.process_next("echo")
        True
    """

    def __init__(self) -> None:
        """Create a new ActorSystem with no actors, no channels, and no dead letters."""
        self._actors: dict[str, Actor] = {}
        self._channels: dict[str, Channel] = {}
        self._dead_letters: list[Message] = []
        self._clock: int = 0

    # ------------------------------------------------------------------
    # Actor lifecycle
    # ------------------------------------------------------------------

    def create_actor(
        self,
        actor_id: str,
        initial_state: Any,
        behavior: Callable[[Any, Message], ActorResult],
    ) -> str:
        """Create and register a new actor.

        Args:
            actor_id: Unique identifier for the actor. Must not conflict
                with any existing actor.
            initial_state: The actor's starting state. Can be any type.
            behavior: Function that processes messages. Signature:
                (state, message) -> ActorResult.

        Returns:
            The actor_id (for convenience in chaining).

        Raises:
            DuplicateActorError: If an actor with this id already exists.

        Example:
            >>> system = ActorSystem()
            >>> system.create_actor("counter", 0, counter_behavior)
            'counter'
        """
        if actor_id in self._actors:
            raise DuplicateActorError(
                f"Actor with id '{actor_id}' already exists."
            )
        actor = Actor(actor_id, state=initial_state, behavior=behavior)
        self._actors[actor_id] = actor
        return actor_id

    def stop_actor(self, actor_id: str) -> None:
        """Stop an actor permanently.

        Sets the actor's status to STOPPED and drains its mailbox to
        dead_letters. A stopped actor cannot receive or process any
        more messages.

        Args:
            actor_id: The id of the actor to stop.

        Raises:
            ActorNotFoundError: If no actor with this id exists.
        """
        actor = self._actors.get(actor_id)
        if actor is None:
            raise ActorNotFoundError(
                f"Actor with id '{actor_id}' not found."
            )
        actor.status = STOPPED
        # Drain remaining mailbox messages to dead_letters
        drained = actor.drain_mailbox()
        self._dead_letters.extend(drained)

    def get_actor_status(self, actor_id: str) -> str:
        """Get an actor's current status.

        Args:
            actor_id: The id of the actor.

        Returns:
            One of: "idle", "processing", "stopped".

        Raises:
            ActorNotFoundError: If no actor with this id exists.
        """
        actor = self._actors.get(actor_id)
        if actor is None:
            raise ActorNotFoundError(
                f"Actor with id '{actor_id}' not found."
            )
        return actor.status

    # ------------------------------------------------------------------
    # Messaging
    # ------------------------------------------------------------------

    def send(self, target_id: str, message: Message) -> None:
        """Deliver a message to an actor's mailbox.

        If the target actor does not exist or is stopped, the message
        goes to dead_letters instead. This is a deliberate design choice:
        the sender should not crash because a recipient is unavailable.

        Args:
            target_id: The id of the actor to receive the message.
            message: The message to deliver.

        Behavior:
            - Target exists and is IDLE/PROCESSING: message enqueued
            - Target exists but is STOPPED: message goes to dead_letters
            - Target does not exist: message goes to dead_letters
        """
        actor = self._actors.get(target_id)
        if actor is None or actor.status == STOPPED:
            self._dead_letters.append(message)
            return
        actor.enqueue(message)

    # ------------------------------------------------------------------
    # Processing
    # ------------------------------------------------------------------

    def process_next(self, actor_id: str) -> bool:
        """Process one message from an actor's mailbox.

        Dequeues the front message, calls the actor's behavior function,
        applies the result (state update, message sends, actor creation),
        and updates the actor's status.

        If the behavior function raises an exception:
        - The actor's state is NOT changed
        - The failed message goes to dead_letters
        - The actor returns to IDLE (continues processing future messages)
        - The exception is logged but does NOT propagate

        Args:
            actor_id: The id of the actor to process.

        Returns:
            True if a message was processed, False if the mailbox was empty.

        Raises:
            ActorNotFoundError: If no actor with this id exists.
        """
        actor = self._actors.get(actor_id)
        if actor is None or actor.status == STOPPED:
            raise ActorNotFoundError(
                f"Actor with id '{actor_id}' not found or is stopped."
            )

        message = actor.dequeue()
        if message is None:
            return False

        # Set status to PROCESSING while the behavior runs
        actor.status = "processing"

        try:
            result = actor.behavior(actor.state, message)
        except Exception:
            # Behavior threw an exception. Per the spec:
            # 1. State is unchanged
            # 2. Message goes to dead_letters
            # 3. Actor returns to IDLE
            logger.exception(
                "Actor '%s' behavior raised an exception while processing "
                "message '%s'. State unchanged, message moved to dead_letters.",
                actor_id,
                message.id,
            )
            self._dead_letters.append(message)
            actor.status = IDLE
            return True

        # Apply the result
        actor.state = result.new_state

        # Deliver outgoing messages
        for target_id, msg in result.messages_to_send:
            self.send(target_id, msg)

        # Create new actors
        for spec in result.actors_to_create:
            # If create_actor fails (duplicate id), log and continue
            try:
                self.create_actor(spec.actor_id, spec.initial_state, spec.behavior)
            except DuplicateActorError:
                logger.warning(
                    "Actor '%s' tried to create actor '%s' which already exists.",
                    actor_id,
                    spec.actor_id,
                )

        # Update status based on stop flag
        if result.stop:
            actor.status = STOPPED
            # Drain remaining mailbox to dead_letters
            drained = actor.drain_mailbox()
            self._dead_letters.extend(drained)
        else:
            actor.status = IDLE

        return True

    def run_until_idle(self) -> dict[str, int]:
        """Process all actors round-robin until no work remains.

        Repeatedly finds an actor with a non-empty mailbox and IDLE status,
        processes one message, then moves to the next actor. Stops when
        no actor has pending messages.

        Returns:
            Statistics dict with keys:
                messages_processed: Total messages processed in this run
                actors_created: Total new actors created

        Example:
            >>> system = ActorSystem()
            >>> system.create_actor("echo", None, echo_behavior)
            'echo'
            >>> system.send("echo", Message.text("user", "hi"))
            >>> stats = system.run_until_idle()
            >>> stats["messages_processed"]
            1
        """
        messages_processed = 0
        actors_before = len(self._actors)

        while True:
            # Find any IDLE actor with messages
            found = False
            for actor_id, actor in list(self._actors.items()):
                if actor.status == IDLE and actor.mailbox_size > 0:
                    self.process_next(actor_id)
                    messages_processed += 1
                    found = True
                    break  # Restart the scan for round-robin fairness

            if not found:
                break

        actors_created = len(self._actors) - actors_before
        return {
            "messages_processed": messages_processed,
            "actors_created": max(0, actors_created),
        }

    def run_until_done(self) -> dict[str, int]:
        """Process all actors until the system is completely quiet.

        Like run_until_idle(), but keeps running until no new messages
        are being generated. This handles cases where processing a
        message generates new messages that need further processing.

        Returns:
            Statistics dict with the same keys as run_until_idle().
        """
        total_processed = 0
        actors_before = len(self._actors)

        while True:
            stats = self.run_until_idle()
            total_processed += stats["messages_processed"]
            if stats["messages_processed"] == 0:
                break

        actors_created = len(self._actors) - actors_before
        return {
            "messages_processed": total_processed,
            "actors_created": max(0, actors_created),
        }

    # ------------------------------------------------------------------
    # Channels
    # ------------------------------------------------------------------

    def create_channel(self, channel_id: str, name: str) -> Channel:
        """Create and register a new channel.

        Args:
            channel_id: Unique identifier for the channel.
            name: Human-readable name for the channel.

        Returns:
            The newly created Channel.

        Raises:
            DuplicateChannelError: If a channel with this id already exists.
        """
        if channel_id in self._channels:
            raise DuplicateChannelError(
                f"Channel with id '{channel_id}' already exists."
            )
        channel = Channel(channel_id, name)
        self._channels[channel_id] = channel
        return channel

    def get_channel(self, channel_id: str) -> Channel:
        """Retrieve a channel by its id.

        Args:
            channel_id: The id of the channel.

        Returns:
            The Channel object.

        Raises:
            ChannelNotFoundError: If no channel with this id exists.
        """
        channel = self._channels.get(channel_id)
        if channel is None:
            raise ChannelNotFoundError(
                f"Channel with id '{channel_id}' not found."
            )
        return channel

    # ------------------------------------------------------------------
    # Inspection
    # ------------------------------------------------------------------

    @property
    def dead_letters(self) -> list[Message]:
        """Messages that could not be delivered.

        Returns a copy to prevent external modification.
        """
        return list(self._dead_letters)

    def actor_ids(self) -> list[str]:
        """List all registered actor IDs.

        Returns:
            A list of actor id strings.
        """
        return list(self._actors.keys())

    def mailbox_size(self, actor_id: str) -> int:
        """Get the number of pending messages for an actor.

        Args:
            actor_id: The id of the actor.

        Returns:
            The number of messages in the actor's mailbox.

        Raises:
            ActorNotFoundError: If no actor with this id exists.
        """
        actor = self._actors.get(actor_id)
        if actor is None:
            raise ActorNotFoundError(
                f"Actor with id '{actor_id}' not found."
            )
        return actor.mailbox_size

    # ------------------------------------------------------------------
    # Shutdown
    # ------------------------------------------------------------------

    def shutdown(self) -> None:
        """Stop all actors and drain all mailboxes to dead_letters.

        This is a clean shutdown: every actor is stopped, every unprocessed
        message is preserved in dead_letters for debugging.
        """
        for actor_id in list(self._actors.keys()):
            actor = self._actors[actor_id]
            if actor.status != STOPPED:
                actor.status = STOPPED
                drained = actor.drain_mailbox()
                self._dead_letters.extend(drained)

    def __repr__(self) -> str:
        """Human-readable representation showing actor and channel counts."""
        return (
            f"ActorSystem(actors={len(self._actors)}, "
            f"channels={len(self._channels)}, "
            f"dead_letters={len(self._dead_letters)})"
        )
