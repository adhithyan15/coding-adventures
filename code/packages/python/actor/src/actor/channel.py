"""Channel -- a one-way, append-only, ordered message log.

=== What Is a Channel? ===

A Channel is a pneumatic tube in an office building. Documents go in one end
and come out the other. You cannot send documents backwards. The tube keeps a
copy of every document that has ever passed through it (the log), and each
office at the receiving end has a bookmark showing which documents they have
already read (the offset).

=== Why One-Way? ===

Bidirectional channels create ambiguity: "who sent this message?" and "can a
receiver inject messages that look like they came from the sender?" One-way
channels eliminate both questions. If you need bidirectional communication,
use two channels -- one in each direction. This is not a limitation. It is
the core security property that the Chief of Staff system (D18) builds on.

=== Why Append-Only? ===

If messages could be deleted or modified, crash recovery becomes impossible.
After a crash, the system asks: "what happened before the crash?" If the log
is mutable, the answer is "we don't know -- someone might have changed it."
If the log is append-only, the answer is definitive: "here is exactly what
happened, in order, immutably recorded."

=== Persistence ===

Channels persist to disk as a binary append log. Each message is written in
the Message wire format (17-byte header + JSON envelope + raw payload bytes),
concatenated end-to-end. This format is:

    Binary-native  -- images, videos stored as raw bytes, zero bloat
    Appendable     -- just write the next message at the end of the file
    Replayable     -- read from the beginning, parse header by header
    Scannable      -- skip payload bytes to index without loading data

=== Recovery ===

After a crash, recovery reads the log file from the beginning. Each message
is parsed from its header. If a crash happened mid-write, the last message
will have a truncated header, envelope, or payload. Recovery discards only
that incomplete message and restores everything before it.

=== Offset Tracking ===

Each consumer independently tracks how far it has read. This is NOT managed
by the channel -- it is the consumer's responsibility. The channel is a dumb
log; consumers are smart readers.

    Channel log:   [m0] [m1] [m2] [m3] [m4] [m5]
                                         ^
    Consumer A:    offset = 4 -----------+
                         ^
    Consumer B:    offset = 1 (behind -- maybe processing slowly)

Consumer A has read messages 0-3 and will read m4 next.
Consumer B has read message 0 and will read m1 next.
They are independent. A being ahead does not affect B.
"""

from __future__ import annotations

import os
import time
from pathlib import Path

from actor.message import Message


class Channel:
    """One-way, append-only, ordered message log.

    A Channel stores an ordered list of Messages. Messages can only be
    appended -- never deleted, modified, or reordered. This makes the
    channel a reliable audit trail that survives crashes.

    === Creating a Channel ===

        >>> ch = Channel("ch_001", "greetings")
        >>> ch.name
        'greetings'
        >>> ch.length()
        0

    === Appending Messages ===

        >>> msg = Message.text("agent", "hello")
        >>> seq = ch.append(msg)  # Returns sequence number 0
        >>> ch.length()
        1

    === Reading Messages ===

        >>> messages = ch.read(offset=0, limit=10)
        >>> len(messages)
        1
        >>> messages[0].payload_text
        'hello'

    === Persistence and Recovery ===

        >>> ch.persist("/tmp/channels")       # Write to disk
        >>> ch2 = Channel.recover("/tmp/channels", "greetings")
        >>> ch2.length()
        1
    """

    __slots__ = ("_id", "_name", "_log", "_created_at")

    def __init__(self, channel_id: str, name: str) -> None:
        """Create a new empty Channel.

        Args:
            channel_id: Unique identifier for this channel.
            name: Human-readable name (e.g., "email-summaries").
                Used for file naming during persistence and for
                debugging output.
        """
        self._id = channel_id
        self._name = name
        self._log: list[Message] = []
        self._created_at = time.monotonic_ns()

    # ------------------------------------------------------------------
    # Properties
    # ------------------------------------------------------------------

    @property
    def id(self) -> str:
        """Unique identifier for this channel."""
        return self._id

    @property
    def name(self) -> str:
        """Human-readable name for this channel.

        Used as the filename stem during persistence (e.g., name="greetings"
        produces "greetings.log" on disk).
        """
        return self._name

    @property
    def created_at(self) -> int:
        """Monotonic nanosecond timestamp of when the channel was created."""
        return self._created_at

    # ------------------------------------------------------------------
    # Core operations
    # ------------------------------------------------------------------

    def append(self, message: Message) -> int:
        """Append a message to the end of the log.

        This is the ONLY write operation on a Channel. There is no delete,
        no update, no insert-at-position. Append-only means the log is an
        immutable history that only grows.

        Args:
            message: The message to append.

        Returns:
            The sequence number of the appended message (0-indexed).
            The first message appended gets sequence number 0, the second
            gets 1, and so on.

        Example:
            >>> ch = Channel("ch1", "test")
            >>> ch.append(Message.text("a", "first"))
            0
            >>> ch.append(Message.text("a", "second"))
            1
        """
        sequence_number = len(self._log)
        self._log.append(message)
        return sequence_number

    def read(self, offset: int = 0, limit: int = 100) -> list[Message]:
        """Read messages from the log starting at an offset.

        This does NOT consume messages -- they remain in the log. Another
        reader can read the same messages independently. This is unlike a
        traditional queue where reading removes the item.

        Args:
            offset: Index of the first message to read (0-based).
            limit: Maximum number of messages to return.

        Returns:
            A list of messages from offset to offset+limit (or end of log,
            whichever comes first). Returns an empty list if offset is
            beyond the end of the log (caller is caught up).

        Example:
            >>> ch = Channel("ch1", "test")
            >>> for i in range(5):
            ...     ch.append(Message.text("a", f"msg_{i}"))
            0
            1
            2
            3
            4
            >>> len(ch.read(offset=2, limit=2))
            2
        """
        if offset >= len(self._log):
            return []
        end = min(offset + limit, len(self._log))
        # Return a copy of the slice so the caller cannot modify our log
        return list(self._log[offset:end])

    def length(self) -> int:
        """Return the number of messages in the log.

        Returns:
            The count of messages appended so far.
        """
        return len(self._log)

    def slice(self, start: int, end: int) -> list[Message]:
        """Return messages from index start to end (exclusive).

        Equivalent to read(start, end - start), but with a more Pythonic
        interface that mirrors list slicing.

        Args:
            start: Index of the first message (inclusive).
            end: Index of the last message (exclusive).

        Returns:
            A list of messages in the range [start, end).

        Example:
            >>> ch = Channel("ch1", "test")
            >>> for i in range(5):
            ...     ch.append(Message.text("a", f"msg_{i}"))
            0
            1
            2
            3
            4
            >>> len(ch.slice(1, 4))
            3
        """
        return list(self._log[start:end])

    # ------------------------------------------------------------------
    # Persistence
    # ------------------------------------------------------------------

    def persist(self, directory: str) -> None:
        """Write the entire channel log to disk as a binary append log.

        Each message is serialized using the Message wire format (17-byte
        header + JSON envelope + raw payload bytes) and concatenated
        end-to-end in a single file.

        The file is named {name}.log inside the given directory.

        Args:
            directory: Path to the directory where the log file will be
                written. The directory is created if it does not exist.

        File layout:
            [ACTM][v1][env_len][pay_len][envelope JSON][payload bytes]
            [ACTM][v1][env_len][pay_len][envelope JSON][payload bytes]
            ...
        """
        path = Path(directory)
        path.mkdir(parents=True, exist_ok=True)

        log_file = path / f"{self._name}.log"
        with open(log_file, "wb") as f:
            for message in self._log:
                f.write(message.to_bytes())
            f.flush()
            os.fsync(f.fileno())

    @classmethod
    def recover(cls, directory: str, name: str) -> Channel:
        """Reconstruct a Channel from a persisted log file on disk.

        Reads the binary log file message by message. If the file ends
        with a truncated message (from a crash mid-write), the incomplete
        message is silently discarded. All complete messages before the
        truncation point are restored.

        Args:
            directory: Path to the directory containing the log file.
            name: The channel name (used to find {name}.log).

        Returns:
            A reconstructed Channel containing all complete messages
            from the log file. If the file does not exist, returns an
            empty Channel.

        Example:
            >>> ch = Channel("ch1", "greetings")
            >>> ch.append(Message.text("a", "hello"))
            0
            >>> ch.persist("/tmp/channels")
            >>> recovered = Channel.recover("/tmp/channels", "greetings")
            >>> recovered.length()
            1
        """
        log_file = Path(directory) / f"{name}.log"

        # Generate a channel_id for the recovered channel
        channel = cls(channel_id=f"recovered_{name}", name=name)

        if not log_file.exists():
            return channel

        with open(log_file, "rb") as f:
            while True:
                message = Message.from_stream(f)
                if message is None:
                    break  # EOF or truncated message
                channel._log.append(message)

        return channel

    # ------------------------------------------------------------------
    # String representation
    # ------------------------------------------------------------------

    def __repr__(self) -> str:
        """Human-readable representation showing id, name, and message count."""
        return (
            f"Channel(id={self._id!r}, name={self._name!r}, "
            f"messages={len(self._log)})"
        )
