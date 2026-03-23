"""IPCManager -- central coordinator for all IPC mechanisms.

The IPCManager is the kernel component that owns all IPC resources. It is
the single point of creation, lookup, and destruction for pipes, message
queues, and shared memory regions.

In a real OS kernel, the IPC manager maintains global tables:

    +------------------+---------------------------------------------+
    | Table            | Description                                 |
    +==================+=============================================+
    | pipes            | All active pipes, indexed by pipe_id.       |
    +------------------+---------------------------------------------+
    | message_queues   | All message queues, keyed by name (the      |
    |                  | "well-known key" that unrelated processes    |
    |                  | agree on, like a phone number).              |
    +------------------+---------------------------------------------+
    | shared_regions   | All shared memory segments, keyed by name.  |
    +------------------+---------------------------------------------+

Pipe creation returns a triple: (pipe_id, read_fd, write_fd). The read_fd
and write_fd are logical file descriptor numbers. In a full OS, the caller
would map these to real entries in the process's file descriptor table.
Here we simply assign sequential integers.

Lifecycle
---------
::

    create_pipe()          -> (pipe_id, read_fd, write_fd)
    close_pipe_read(id)    -> close the read end
    close_pipe_write(id)   -> close the write end

    create_message_queue(name)  -> MessageQueue
    get_message_queue(name)     -> MessageQueue | None
    delete_message_queue(name)  -> bool

    create_shared_memory(name, size, owner_pid) -> SharedMemoryRegion
    get_shared_memory(name)                     -> SharedMemoryRegion | None
    delete_shared_memory(name)                  -> bool
"""

from ipc.message_queue import MessageQueue
from ipc.pipe import Pipe
from ipc.shared_memory import SharedMemoryRegion


class IPCManager:
    """Central coordinator for all IPC mechanisms.

    Example
    -------
    >>> mgr = IPCManager()
    >>> pipe_id, read_fd, write_fd = mgr.create_pipe()
    >>> pipe = mgr.get_pipe(pipe_id)
    >>> pipe.write(b"hello from parent")
    17
    >>> pipe.read(17)
    b'hello from parent'
    """

    def __init__(self) -> None:
        # ----------------------------------------------------------------
        # _pipes:          Map from pipe_id -> Pipe object.
        # _next_pipe_id:   Counter for assigning unique pipe IDs.
        # _next_fd:        Counter for assigning logical file descriptors.
        #                  In a real OS, each process has its own FD table;
        #                  here we use a single global counter for simplicity.
        # _message_queues: Map from name -> MessageQueue.
        # _shared_regions: Map from name -> SharedMemoryRegion.
        # ----------------------------------------------------------------
        self._pipes: dict[int, Pipe] = {}
        self._next_pipe_id: int = 0
        self._next_fd: int = 3  # 0=stdin, 1=stdout, 2=stderr are reserved

        self._message_queues: dict[str, MessageQueue] = {}
        self._shared_regions: dict[str, SharedMemoryRegion] = {}

    # ====================================================================
    # Pipe management
    # ====================================================================

    def create_pipe(self, capacity: int = 4096) -> tuple[int, int, int]:
        """Create a new pipe.

        Returns
        -------
        tuple[int, int, int]
            (pipe_id, read_fd, write_fd)

            - pipe_id: unique identifier for the pipe
            - read_fd: logical file descriptor for the read end
            - write_fd: logical file descriptor for the write end

        In a real OS, after fork(), both parent and child have copies of
        read_fd and write_fd. Typically the parent closes one end and the
        child closes the other to establish a one-way channel.
        """
        pipe_id = self._next_pipe_id
        self._next_pipe_id += 1

        read_fd = self._next_fd
        self._next_fd += 1
        write_fd = self._next_fd
        self._next_fd += 1

        self._pipes[pipe_id] = Pipe(capacity=capacity)
        return (pipe_id, read_fd, write_fd)

    def get_pipe(self, pipe_id: int) -> Pipe | None:
        """Look up a pipe by its ID. Returns None if not found."""
        return self._pipes.get(pipe_id)

    def close_pipe_read(self, pipe_id: int) -> None:
        """Close the read end of a pipe.

        After this, any write to the pipe raises BrokenPipeError.
        """
        pipe = self._pipes.get(pipe_id)
        if pipe is not None:
            pipe.close_read()

    def close_pipe_write(self, pipe_id: int) -> None:
        """Close the write end of a pipe.

        After this, once the buffer drains, readers see EOF.
        """
        pipe = self._pipes.get(pipe_id)
        if pipe is not None:
            pipe.close_write()

    def destroy_pipe(self, pipe_id: int) -> bool:
        """Remove a pipe from the manager entirely.

        Returns True if the pipe existed and was removed.
        """
        if pipe_id in self._pipes:
            del self._pipes[pipe_id]
            return True
        return False

    # ====================================================================
    # Message queue management
    # ====================================================================

    def create_message_queue(
        self,
        name: str,
        max_messages: int = 256,
        max_message_size: int = 4096,
    ) -> MessageQueue:
        """Create a new message queue with the given name.

        If a queue with this name already exists, returns the existing one
        (like msgget with IPC_CREAT in Unix -- idempotent creation).

        Parameters
        ----------
        name : str
            The well-known key that processes use to find this queue.
        max_messages : int
            Maximum number of messages (default 256).
        max_message_size : int
            Maximum single-message size in bytes (default 4096).

        Returns
        -------
        MessageQueue
            The newly created (or existing) message queue.
        """
        if name in self._message_queues:
            return self._message_queues[name]

        mq = MessageQueue(
            max_messages=max_messages,
            max_message_size=max_message_size,
        )
        self._message_queues[name] = mq
        return mq

    def get_message_queue(self, name: str) -> MessageQueue | None:
        """Look up a message queue by name. Returns None if not found."""
        return self._message_queues.get(name)

    def delete_message_queue(self, name: str) -> bool:
        """Delete a message queue.

        Returns True if the queue existed and was deleted.
        Any messages still in the queue are lost.
        """
        if name in self._message_queues:
            del self._message_queues[name]
            return True
        return False

    # ====================================================================
    # Shared memory management
    # ====================================================================

    def create_shared_memory(
        self,
        name: str,
        size: int,
        owner_pid: int = 0,
    ) -> SharedMemoryRegion:
        """Create a new shared memory region.

        If a region with this name already exists, returns the existing one
        (like shmget with IPC_CREAT).

        Parameters
        ----------
        name : str
            The well-known key for this segment.
        size : int
            Size of the region in bytes.
        owner_pid : int
            PID of the creating process.

        Returns
        -------
        SharedMemoryRegion
            The newly created (or existing) shared memory region.
        """
        if name in self._shared_regions:
            return self._shared_regions[name]

        region = SharedMemoryRegion(name=name, size=size, owner_pid=owner_pid)
        self._shared_regions[name] = region
        return region

    def get_shared_memory(self, name: str) -> SharedMemoryRegion | None:
        """Look up a shared memory region by name. Returns None if not found."""
        return self._shared_regions.get(name)

    def delete_shared_memory(self, name: str) -> bool:
        """Delete a shared memory region.

        Returns True if the region existed and was deleted.
        Any data in the region is lost.
        """
        if name in self._shared_regions:
            del self._shared_regions[name]
            return True
        return False

    # ====================================================================
    # Listing operations
    # ====================================================================

    def list_pipes(self) -> list[int]:
        """Return a list of all active pipe IDs."""
        return list(self._pipes.keys())

    def list_message_queues(self) -> list[str]:
        """Return a list of all message queue names."""
        return list(self._message_queues.keys())

    def list_shared_regions(self) -> list[str]:
        """Return a list of all shared memory region names."""
        return list(self._shared_regions.keys())
