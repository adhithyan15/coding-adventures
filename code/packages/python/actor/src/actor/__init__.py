"""Actor -- immutable messages, append-only channels, and isolated actors.

The Actor model is a mathematical framework for concurrent computation.
It defines computation in terms of **actors** -- independent entities that
communicate exclusively through **messages**. No shared memory. No locks.
No mutexes. Just isolated units of computation passing immutable messages
through one-way channels.

This package implements three primitives:

1. **Message** -- the atom of communication. Immutable, typed, serializable.
2. **Channel** -- a one-way, append-only pipe for messages. Persistent
   and replayable.
3. **Actor** -- an isolated unit of computation with a mailbox and
   internal state.
4. **ActorSystem** -- the runtime that manages actor lifecycles, message
   delivery, and channels.

These four types are sufficient to build complete concurrent systems --
from simple request-response patterns to complex multi-agent pipelines.

Quick start:
    >>> from actor import Message, Channel, Actor, ActorResult, ActorSystem
    >>> system = ActorSystem()
    >>> def echo(state, message):
    ...     reply = Message.text("echo", f"echo: {message.payload_text}")
    ...     return ActorResult(state, [(message.sender_id, reply)])
    >>> system.create_actor("echo", None, echo)
    'echo'
"""

from actor.actor import Actor, ActorResult, ActorSpec
from actor.actor_system import (
    ActorNotFoundError,
    ActorSystem,
    ChannelNotFoundError,
    DuplicateActorError,
    DuplicateChannelError,
)
from actor.channel import Channel
from actor.message import InvalidFormatError, Message, VersionError

__all__ = [
    "Actor",
    "ActorNotFoundError",
    "ActorResult",
    "ActorSpec",
    "ActorSystem",
    "Channel",
    "ChannelNotFoundError",
    "DuplicateActorError",
    "DuplicateChannelError",
    "InvalidFormatError",
    "Message",
    "VersionError",
]
