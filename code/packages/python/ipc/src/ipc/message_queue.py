"""MessageQueue -- a FIFO queue of typed messages.

While pipes transmit raw bytes (the reader must know how to parse them),
message queues transmit discrete, typed **messages**. Each message has a
type tag and a body, and the receiver can filter by type.

Analogy
-------
Think of a message queue as a shared mailbox in a hallway. Anyone can drop
off an envelope with a label ("type 1: request", "type 2: status update"),
and anyone can pick up envelopes. You can ask for "any envelope" or "only
type-2 envelopes."

Message Structure
-----------------
Each message is a tuple of (type, data):

    +----------+---------------------------------------------------+
    | Field    | Description                                       |
    +==========+===================================================+
    | msg_type | Positive integer identifying the message kind.     |
    |          | Receivers filter on this: "give me only type 3."   |
    |          | Type 0 in receive() means "give me any message."   |
    +----------+---------------------------------------------------+
    | data     | The payload -- up to max_message_size bytes of     |
    |          | arbitrary data.                                     |
    +----------+---------------------------------------------------+

Queue Limits
------------
Two limits prevent unbounded resource consumption:

    - **max_messages** (default 256): If the queue has this many messages,
      send() returns False (in a real OS, the sender would block until
      space opens up).
    - **max_message_size** (default 4096): Messages larger than this are
      rejected immediately. This prevents a single enormous message from
      monopolizing kernel memory.

Send/Receive Semantics
----------------------
::

    send(msg_type, data):
      1. Validate len(data) <= max_message_size.
      2. If queue is full: return False (would block in real OS).
      3. Append (msg_type, data) to the back of the FIFO.

    receive(msg_type=0):
      1. If msg_type == 0: dequeue the first message of any type.
      2. If msg_type > 0: find and remove the first message with
         matching type (non-matching messages stay in the queue).
      3. If no matching message: return None (would block in real OS).
"""


class MessageQueue:
    """FIFO queue of typed messages for inter-process communication.

    Parameters
    ----------
    max_messages : int
        Maximum number of messages the queue can hold (default 256).
    max_message_size : int
        Maximum size of a single message body in bytes (default 4096).

    Example
    -------
    >>> mq = MessageQueue(max_messages=10)
    >>> mq.send(1, b"hello")
    True
    >>> mq.send(2, b"status ok")
    True
    >>> mq.receive(msg_type=2)
    (2, b'status ok')
    >>> mq.receive()
    (1, b'hello')
    """

    def __init__(
        self,
        max_messages: int = 256,
        max_message_size: int = 4096,
    ) -> None:
        # ----------------------------------------------------------------
        # _messages: The FIFO queue, stored as a list of (type, data) tuples.
        #
        # We use a plain list rather than collections.deque because we need
        # random access for type-filtered receives (we scan for the first
        # message matching a given type and remove it, leaving others in
        # place). A deque only supports efficient removal from the ends.
        #
        # Performance note: In a real OS kernel, the message queue would be
        # a linked list with O(1) removal. Our list-based approach is O(n)
        # for filtered receives, but n <= max_messages (256), so it is fine
        # for a simulation.
        # ----------------------------------------------------------------
        self._messages: list[tuple[int, bytes]] = []
        self._max_messages = max_messages
        self._max_message_size = max_message_size

    # ====================================================================
    # Send
    # ====================================================================

    def send(self, msg_type: int, data: bytes) -> bool:
        """Add a message to the queue.

        Parameters
        ----------
        msg_type : int
            A positive integer identifying the message kind. Receivers can
            filter by this value.
        data : bytes
            The message payload.

        Returns
        -------
        bool
            True if the message was enqueued successfully. False if:
            - The queue is full (at max_messages capacity).
            - The message body exceeds max_message_size.
            - msg_type is not a positive integer.

        In a real OS, the "queue full" case would block the sender until
        another process calls receive(). In our simulation we return False.
        """
        # ----- Validate message type -----
        if msg_type <= 0:
            return False

        # ----- Validate message size -----
        if len(data) > self._max_message_size:
            return False

        # ----- Check capacity -----
        if len(self._messages) >= self._max_messages:
            return False

        # ----- Enqueue -----
        self._messages.append((msg_type, data))
        return True

    # ====================================================================
    # Receive
    # ====================================================================

    def receive(self, msg_type: int = 0) -> tuple[int, bytes] | None:
        """Receive a message from the queue.

        Parameters
        ----------
        msg_type : int
            Which message to receive:
            - 0 (default): receive the oldest message of ANY type.
            - >0: receive the oldest message of exactly this type,
              skipping (and preserving) non-matching messages.

        Returns
        -------
        tuple[int, bytes] | None
            A (type, data) tuple if a matching message was found, or
            None if the queue is empty or has no matching messages.

        Example of type filtering
        -------------------------
        Queue contents (oldest first):
            (1, b"req1"), (2, b"status"), (1, b"req2")

        receive(msg_type=2) returns (2, b"status").
        Queue is now:
            (1, b"req1"), (1, b"req2")

        The type-2 message was plucked from the middle; the type-1
        messages were not disturbed.
        """
        if not self._messages:
            return None

        if msg_type == 0:
            # ----- Any type: dequeue the oldest message -----
            return self._messages.pop(0)

        # ----- Filtered receive: find first matching type -----
        # Scan the queue for the first message with the requested type.
        # This is like sorting through your mailbox looking for a specific
        # label -- you flip past envelopes that don't match and grab the
        # first one that does.
        for i, (mtype, _mdata) in enumerate(self._messages):
            if mtype == msg_type:
                return self._messages.pop(i)

        # No matching message found.
        return None

    # ====================================================================
    # Properties
    # ====================================================================

    @property
    def message_count(self) -> int:
        """Number of messages currently in the queue."""
        return len(self._messages)

    @property
    def is_empty(self) -> bool:
        """True if the queue has no messages."""
        return len(self._messages) == 0

    @property
    def is_full(self) -> bool:
        """True if the queue is at max_messages capacity."""
        return len(self._messages) >= self._max_messages

    @property
    def max_messages(self) -> int:
        """The maximum number of messages this queue can hold."""
        return self._max_messages

    @property
    def max_message_size(self) -> int:
        """The maximum size of a single message body in bytes."""
        return self._max_message_size
