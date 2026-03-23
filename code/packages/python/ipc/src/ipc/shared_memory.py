"""SharedMemoryRegion -- a named shared memory segment.

Pipes and message queues both **copy** data: the sender writes bytes, the
kernel copies them into a buffer, and the receiver copies them out. For
large data transfers, this double-copy is expensive.

Shared memory eliminates copying entirely. Two (or more) processes map the
**same physical pages** into their virtual address spaces. A write by one
process is immediately visible to the other -- no system call, no copy, no
kernel involvement after setup.

Analogy
-------
Imagine two people in adjacent offices with a window between them and a
whiteboard mounted in the window frame. Both can read the whiteboard and
both can write on it. This is the fastest possible communication -- no
passing notes, no mailbox. But there is a catch: if both people write at
the same time, the result is garbled. In real systems, semaphores or
mutexes coordinate access. Our simulation omits synchronization for
simplicity, but the hazard is real.

Memory Layout
-------------
::

    Process A's address space          Process B's address space
    +----------------------------+    +----------------------------+
    |  ...                       |    |  ...                       |
    |  0x8000 +-------------+   |    |  0xC000 +-------------+   |
    |         | Shared Data |<--+----+-------->| Shared Data |   |
    |         | "Hello"     |   |    |         | "Hello"     |   |
    |         +-------------+   |    |         +-------------+   |
    |  ...                       |    |  ...                       |
    +----------------------------+    +----------------------------+
                |                                  |
                +------------+---------------------+
                             |
                      +------v------+
                      | Physical    |
                      | Page Frame  |  <-- same physical memory
                      | #42         |
                      +-------------+

Operations
----------
- **attach(pid)**: Map the shared region into a process's address space.
  Adds the PID to the set of attached processes.
- **detach(pid)**: Unmap the region from a process. Removes the PID.
- **read(offset, count)**: Read bytes directly from the shared data.
- **write(offset, data)**: Write bytes directly into the shared data.

Bounds checking is enforced on every read and write. Attempting to access
beyond the region's size raises a ValueError.
"""


class SharedMemoryRegion:
    """A named shared memory segment accessible by multiple processes.

    Parameters
    ----------
    name : str
        A human-readable name for this segment (used as a lookup key).
    size : int
        Size of the shared region in bytes.
    owner_pid : int
        The PID of the process that created this segment.

    Example
    -------
    >>> shm = SharedMemoryRegion("cache", size=1024, owner_pid=1)
    >>> shm.attach(1)
    True
    >>> shm.write(0, b"shared data")
    11
    >>> shm.read(0, 11)
    b'shared data'
    >>> shm.attach(2)
    True
    >>> shm.read(0, 11)  # Process 2 sees the same data
    b'shared data'
    """

    def __init__(self, name: str, size: int, owner_pid: int = 0) -> None:
        # ----------------------------------------------------------------
        # _name:          Human-readable identifier for lookup.
        # _size:          Fixed size of this segment in bytes.
        # _data:          The shared bytes -- a bytearray that represents
        #                 the underlying physical page(s). Zero-initialized,
        #                 just like freshly allocated memory in a real OS.
        # _owner_pid:     PID of the creator. Used for permission checks in
        #                 a full implementation (we keep it for bookkeeping).
        # _attached_pids: Set of PIDs currently "mapped in" to this segment.
        #                 A process must attach before reading or writing.
        #                 When the last process detaches, the segment can be
        #                 destroyed (if marked for deletion).
        # ----------------------------------------------------------------
        self._name = name
        self._size = size
        self._data = bytearray(size)
        self._owner_pid = owner_pid
        self._attached_pids: set[int] = set()

    # ====================================================================
    # Attach / Detach
    # ====================================================================

    def attach(self, pid: int) -> bool:
        """Attach a process to this shared memory region.

        In a real OS, this modifies the process's page table so that a
        range of virtual addresses points to the shared physical pages.
        In our simulation, we just record the PID.

        Parameters
        ----------
        pid : int
            The process ID to attach.

        Returns
        -------
        bool
            True if the PID was newly attached, False if already attached.
        """
        if pid in self._attached_pids:
            return False
        self._attached_pids.add(pid)
        return True

    def detach(self, pid: int) -> bool:
        """Detach a process from this shared memory region.

        Parameters
        ----------
        pid : int
            The process ID to detach.

        Returns
        -------
        bool
            True if the PID was detached, False if it was not attached.
        """
        if pid not in self._attached_pids:
            return False
        self._attached_pids.discard(pid)
        return True

    # ====================================================================
    # Read / Write
    # ====================================================================

    def read(self, offset: int, count: int) -> bytes:
        """Read bytes from the shared region.

        Parameters
        ----------
        offset : int
            Starting byte offset within the region.
        count : int
            Number of bytes to read.

        Returns
        -------
        bytes
            The data at the given offset.

        Raises
        ------
        ValueError
            If offset + count exceeds the region size, or if offset
            is negative.

        Note
        ----
        In a real OS, the process reads directly from its virtual address
        space (no system call needed after attach). We simulate this with
        an explicit read method for clarity.
        """
        if offset < 0:
            raise ValueError(f"negative offset: {offset}")
        if offset + count > self._size:
            raise ValueError(
                f"read beyond region bounds: offset={offset}, "
                f"count={count}, size={self._size}"
            )
        return bytes(self._data[offset : offset + count])

    def write(self, offset: int, data: bytes) -> int:
        """Write bytes into the shared region.

        Parameters
        ----------
        offset : int
            Starting byte offset within the region.
        data : bytes
            The bytes to write.

        Returns
        -------
        int
            Number of bytes written.

        Raises
        ------
        ValueError
            If offset + len(data) exceeds the region size, or if offset
            is negative.

        Warning
        -------
        Shared memory has NO built-in synchronization. If process A writes
        while process B reads, B may see partially-updated data. Real
        programs use semaphores or mutexes to coordinate access.
        """
        if offset < 0:
            raise ValueError(f"negative offset: {offset}")
        if offset + len(data) > self._size:
            raise ValueError(
                f"write beyond region bounds: offset={offset}, "
                f"len(data)={len(data)}, size={self._size}"
            )
        self._data[offset : offset + len(data)] = data
        return len(data)

    # ====================================================================
    # Properties
    # ====================================================================

    @property
    def name(self) -> str:
        """The human-readable name of this shared memory segment."""
        return self._name

    @property
    def size(self) -> int:
        """The size of this segment in bytes."""
        return self._size

    @property
    def owner_pid(self) -> int:
        """The PID of the process that created this segment."""
        return self._owner_pid

    @property
    def attached_count(self) -> int:
        """Number of processes currently attached to this segment."""
        return len(self._attached_pids)

    def is_attached(self, pid: int) -> bool:
        """Check whether a given PID is currently attached."""
        return pid in self._attached_pids
