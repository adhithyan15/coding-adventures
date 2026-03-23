"""Actor -- an isolated unit of computation with a mailbox and behavior.

=== What Is an Actor? ===

An actor is a person sitting alone in a soundproofed room with a mail slot
in the door. Letters (messages) come in through the slot and pile up in a
tray (mailbox). The person reads one letter at a time, thinks about it,
possibly writes reply letters and slides them out through their own mail
slot, and possibly rearranges things on their desk (state). They never leave
the room. They never look into anyone else's room. They only know about
other rooms by their mail slot addresses.

=== The Four Things an Actor Can Do ===

When an actor receives a message, it can:

1. Send messages to other actors it knows about
2. Create new actors
3. Change its own internal state
4. Choose to stop (halt permanently)

An actor CANNOT:

1. Access another actor's internal state
2. Share memory with another actor
3. Communicate except through messages

=== The Behavior Function ===

The behavior is the heart of an actor. It takes the current state and one
message, and returns an ActorResult:

    def my_behavior(state, message) -> ActorResult:
        # Process the message, return new state + side effects
        return ActorResult(
            new_state=state + 1,
            messages_to_send=[("other_actor", reply_msg)],
        )

The behavior function is called once per message, sequentially. While
processing message N, messages N+1, N+2, etc. accumulate in the mailbox
but are not touched. This eliminates all concurrency hazards -- no races,
no deadlocks, no need for locks.

=== ActorResult ===

The return value of a behavior function. Contains:

    new_state        -- The actor's state after processing this message
    messages_to_send -- List of (target_id, message) pairs to deliver
    actors_to_create -- List of ActorSpec objects to spawn new actors
    stop             -- If True, the actor halts permanently

=== ActorSpec ===

A specification for creating a new actor. Contains:

    actor_id       -- The unique id for the new actor
    initial_state  -- The starting state
    behavior       -- The behavior function

=== Status Lifecycle ===

An actor has three possible statuses:

    IDLE       -- Waiting for messages. Can receive and process messages.
    PROCESSING -- Currently handling a message. Still receives messages
                  (they queue up), but does not process them until the
                  current message is done.
    STOPPED    -- Permanently halted. Cannot receive or process messages.
                  Any messages sent to a stopped actor go to dead_letters.

    State transitions:
        IDLE --> PROCESSING  (when process_next dequeues a message)
        PROCESSING --> IDLE  (when behavior returns without stop=True)
        PROCESSING --> STOPPED (when behavior returns with stop=True)
        IDLE --> STOPPED     (when stop_actor is called externally)
"""

from __future__ import annotations

from collections import deque
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from collections.abc import Callable

    from actor.message import Message


# ---------------------------------------------------------------------------
# Status constants
# ---------------------------------------------------------------------------

IDLE = "idle"
"""Actor is waiting for messages. Ready to process."""

PROCESSING = "processing"
"""Actor is currently handling a message."""

STOPPED = "stopped"
"""Actor has permanently halted. No further message processing."""


# ---------------------------------------------------------------------------
# ActorResult -- the return type of behavior functions
# ---------------------------------------------------------------------------


class ActorResult:
    """Return value from an actor's behavior function.

    When an actor processes a message, its behavior function returns an
    ActorResult that describes what should happen next:

    - new_state: What the actor's internal state should become
    - messages_to_send: Messages to deliver to other actors
    - actors_to_create: New actors to spawn
    - stop: Whether this actor should halt permanently

    Example -- echo actor that sends back what it received:

        >>> result = ActorResult(
        ...     new_state=None,
        ...     messages_to_send=[("sender", reply_msg)],
        ... )

    Example -- counter that increments and stops at 10:

        >>> result = ActorResult(
        ...     new_state=count + 1,
        ...     stop=(count + 1 >= 10),
        ... )
    """

    __slots__ = ("new_state", "messages_to_send", "actors_to_create", "stop")

    def __init__(
        self,
        new_state: Any,
        messages_to_send: list[tuple[str, Message]] | None = None,
        actors_to_create: list[ActorSpec] | None = None,
        stop: bool = False,
    ) -> None:
        """Create an ActorResult.

        Args:
            new_state: The actor's state after processing this message.
                Can be any type -- the actor system does not inspect it.
            messages_to_send: List of (target_actor_id, message) pairs.
                Each message will be delivered to the target actor's
                mailbox. Can be None or empty if the actor has nothing
                to say.
            actors_to_create: List of ActorSpec objects describing new
                actors to spawn. Can be None or empty.
            stop: If True, the actor halts permanently after this
                message. Default is False (keep processing).
        """
        self.new_state = new_state
        self.messages_to_send = messages_to_send or []
        self.actors_to_create = actors_to_create or []
        self.stop = stop


# ---------------------------------------------------------------------------
# ActorSpec -- specification for creating a new actor
# ---------------------------------------------------------------------------


class ActorSpec:
    """Specification for creating a new actor.

    This is a data class that holds everything needed to spawn an actor:
    its id, its starting state, and its behavior function. It is used
    in ActorResult.actors_to_create to tell the ActorSystem to create
    new actors as a side effect of processing a message.

    Example:
        >>> spec = ActorSpec(
        ...     actor_id="worker_1",
        ...     initial_state=0,
        ...     behavior=counter_behavior,
        ... )
    """

    __slots__ = ("actor_id", "initial_state", "behavior")

    def __init__(
        self,
        actor_id: str,
        initial_state: Any,
        behavior: Callable[[Any, Message], ActorResult],
    ) -> None:
        """Create an ActorSpec.

        Args:
            actor_id: Unique identifier for the new actor. Must not
                conflict with any existing actor in the system.
            initial_state: The starting state for the new actor.
            behavior: The function that will process messages. Takes
                (state, message) and returns an ActorResult.
        """
        self.actor_id = actor_id
        self.initial_state = initial_state
        self.behavior = behavior


# ---------------------------------------------------------------------------
# Actor -- the isolated computation unit
# ---------------------------------------------------------------------------


class Actor:
    """An isolated unit of computation with a mailbox and behavior.

    Each Actor has:
    - An id (its address for receiving messages)
    - A mailbox (FIFO queue of incoming messages)
    - A state (private data, any type)
    - A behavior (function that processes one message at a time)
    - A status (IDLE, PROCESSING, or STOPPED)

    The Actor class itself is a container. It does not process messages
    on its own -- the ActorSystem calls its behavior function. This
    separation keeps actors passive and testable.

    Example:
        >>> def echo(state, message):
        ...     return ActorResult(new_state=state)
        >>> actor = Actor("echo_1", state=None, behavior=echo)
        >>> actor.status
        'idle'
        >>> actor.mailbox_size
        0
    """

    __slots__ = ("_id", "_mailbox", "_state", "_behavior", "_status")

    def __init__(
        self,
        actor_id: str,
        state: Any,
        behavior: Callable[[Any, Message], ActorResult],
    ) -> None:
        """Create a new Actor.

        Args:
            actor_id: Unique identifier. Other actors use this to send
                messages to this actor.
            state: Initial internal state. Can be any type -- the actor
                system never inspects it.
            behavior: Function that processes messages. Signature:
                (state, message) -> ActorResult.
        """
        self._id = actor_id
        self._mailbox: deque[Message] = deque()
        self._state = state
        self._behavior = behavior
        self._status = IDLE

    @property
    def id(self) -> str:
        """The actor's unique identifier (its address)."""
        return self._id

    @property
    def status(self) -> str:
        """Current status: 'idle', 'processing', or 'stopped'."""
        return self._status

    @status.setter
    def status(self, value: str) -> None:
        """Set the actor's status. Used by ActorSystem during processing."""
        self._status = value

    @property
    def state(self) -> Any:
        """The actor's private internal state."""
        return self._state

    @state.setter
    def state(self, value: Any) -> None:
        """Update the actor's state. Used by ActorSystem after processing."""
        self._state = value

    @property
    def behavior(self) -> Callable[[Any, Message], ActorResult]:
        """The function that processes messages for this actor."""
        return self._behavior

    @property
    def mailbox_size(self) -> int:
        """Number of messages waiting in the mailbox."""
        return len(self._mailbox)

    def enqueue(self, message: Message) -> None:
        """Add a message to the back of the mailbox.

        Messages are processed in FIFO order -- the first message
        enqueued is the first message processed.

        Args:
            message: The message to add to the mailbox.
        """
        self._mailbox.append(message)

    def dequeue(self) -> Message | None:
        """Remove and return the front message from the mailbox.

        Returns:
            The oldest unprocessed message, or None if the mailbox
            is empty.
        """
        if self._mailbox:
            return self._mailbox.popleft()
        return None

    def drain_mailbox(self) -> list[Message]:
        """Remove and return all messages from the mailbox.

        Used when an actor is stopped -- all pending messages are
        moved to dead_letters so they are not silently lost.

        Returns:
            A list of all messages that were in the mailbox, in FIFO order.
        """
        messages = list(self._mailbox)
        self._mailbox.clear()
        return messages

    def __repr__(self) -> str:
        """Human-readable representation showing id, status, and mailbox size."""
        return (
            f"Actor(id={self._id!r}, status={self._status!r}, "
            f"mailbox={len(self._mailbox)})"
        )
