"""Pipe -- a unidirectional byte stream between two processes.

A pipe is the simplest IPC mechanism. It connects a writer to a reader
through a fixed-size circular buffer. Data written to one end appears at
the other end, in order, exactly once.

Analogy
-------
Think of a pipe as a garden hose: you push water in one end and it flows
out the other. You can't send water backwards, and the hose has a fixed
capacity -- if you try to push more water than it can hold, you have to
wait until some drains out the other end.

Circular Buffer
---------------
The pipe uses a circular (ring) buffer to store data in transit. A circular
buffer is an array that "wraps around": when the write position reaches
the end of the array, it jumps back to index 0.

Why circular? Because it lets us reuse space as soon as the reader consumes
it, without ever shifting elements. Two pointers chase each other around
the ring:

    +---------+
    | a b c . |   read_pos=0, write_pos=3, count=3
    +---------+
      ^     ^
      R     W

After reading 2 bytes ("ab"):

    +---------+
    | . . c . |   read_pos=2, write_pos=3, count=1
    +---------+
          ^ ^
          R W

After writing "defg" (wraps around):

    +---------+
    | f g c d e |   read_pos=2, write_pos=2, count=5
    +---------+   (conceptually -- indices mod capacity)

The key formula:
    - bytes in buffer = count (we track explicitly for clarity)
    - space remaining = capacity - count
    - next write index = write_pos % capacity
    - next read index  = read_pos % capacity

Reference Counts
----------------
A pipe tracks how many readers and writers are attached:

    - When all writers close: the pipe is at EOF. A reader that finds an
      empty buffer knows no more data will ever arrive. This is how
      ``cat file | grep hello`` terminates: when ``cat`` finishes, ``grep``
      sees EOF.
    - When all readers close: the pipe is "broken." A writer that tries to
      push data gets a BrokenPipeError (EPIPE in Unix). There is nobody
      listening, so writing is pointless.
"""


class Pipe:
    """Unidirectional byte stream backed by a circular buffer.

    Parameters
    ----------
    capacity : int
        Maximum number of bytes the pipe can hold at once. Defaults to 4096,
        which matches one memory page -- a common convention in Unix systems.

    Example
    -------
    >>> pipe = Pipe(capacity=16)
    >>> pipe.write(b"hello")
    5
    >>> pipe.read(3)
    b'hel'
    >>> pipe.available
    2
    >>> pipe.space
    14
    """

    def __init__(self, capacity: int = 4096) -> None:
        # ----------------------------------------------------------------
        # Internal state
        #
        # _buffer:      The circular byte array. Pre-allocated to `capacity`.
        # _capacity:    Fixed size of the buffer.
        # _read_pos:    Index of the next byte to read (mod capacity).
        # _write_pos:   Index of the next byte to write (mod capacity).
        # _count:       Number of bytes currently in the buffer. We track
        #               this explicitly rather than computing it from the
        #               positions to avoid the ambiguity when read_pos ==
        #               write_pos (which could mean empty OR full).
        # _readers:     Reference count of open read ends.
        # _writers:     Reference count of open write ends.
        # _closed_read: Whether the read end has been closed.
        # _closed_write: Whether the write end has been closed.
        # ----------------------------------------------------------------
        self._buffer = bytearray(capacity)
        self._capacity = capacity
        self._read_pos = 0
        self._write_pos = 0
        self._count = 0
        self._readers = 1
        self._writers = 1
        self._closed_read = False
        self._closed_write = False

    # ====================================================================
    # Write
    # ====================================================================

    def write(self, data: bytes) -> int:
        """Write data into the pipe, returning the number of bytes written.

        Behavior depends on the pipe's state:

        +-------------------+------------------+---------------------------+
        | Readers alive?    | Buffer has space? | Action                    |
        +===================+==================+===========================+
        | No (_readers = 0) | (any)            | Raise BrokenPipeError     |
        | Yes               | Yes              | Write as much as fits     |
        | Yes               | No (full)        | Return 0 (would block)    |
        +-------------------+------------------+---------------------------+

        In a real OS, the "would block" case suspends the process until a
        reader drains some data. In our simulation, we simply write what
        fits and return the count.

        Parameters
        ----------
        data : bytes
            The bytes to write.

        Returns
        -------
        int
            Number of bytes actually written (may be less than len(data)
            if the buffer is nearly full).

        Raises
        ------
        BrokenPipeError
            If no readers are attached (nobody will ever read this data).
        """
        # ----- Guard: broken pipe -----
        if self._closed_read or self._readers <= 0:
            raise BrokenPipeError(
                "write to a pipe with no readers (EPIPE / broken pipe)"
            )

        # ----- Guard: write end closed -----
        if self._closed_write:
            raise BrokenPipeError("write end is closed")

        # ----- Calculate how many bytes we can write -----
        to_write = min(len(data), self.space)
        if to_write == 0:
            return 0

        # ----- Copy bytes into the circular buffer -----
        # We may need to write in two chunks if the data wraps around the
        # end of the buffer:
        #
        #   Case 1 (no wrap):
        #     [....WWWW....]   write_pos is before end, data fits
        #          ^^^^
        #
        #   Case 2 (wrap):
        #     [WW........WW]   write_pos is near end, data wraps to start
        #      ^^        ^^
        for i in range(to_write):
            self._buffer[self._write_pos] = data[i]
            self._write_pos = (self._write_pos + 1) % self._capacity

        self._count += to_write
        return to_write

    # ====================================================================
    # Read
    # ====================================================================

    def read(self, count: int) -> bytes:
        """Read up to ``count`` bytes from the pipe.

        Behavior depends on the pipe's state:

        +-------------------+------------------+---------------------------+
        | Writers alive?    | Buffer has data? | Action                    |
        +===================+==================+===========================+
        | (any)             | Yes              | Read available bytes      |
        | Yes               | No (empty)       | Return b"" (would block)  |
        | No (_writers = 0) | No (empty)       | Return b"" (EOF)          |
        +-------------------+------------------+---------------------------+

        In a real OS, the "would block" case suspends the process. We
        return empty bytes; the caller can check ``is_eof`` to distinguish
        between "no data yet" and "pipe is done."

        Parameters
        ----------
        count : int
            Maximum number of bytes to read.

        Returns
        -------
        bytes
            The data read (may be shorter than ``count``).
        """
        if self._closed_read:
            return b""

        to_read = min(count, self._count)
        if to_read == 0:
            return b""

        # ----- Copy bytes out of the circular buffer -----
        result = bytearray(to_read)
        for i in range(to_read):
            result[i] = self._buffer[self._read_pos]
            self._read_pos = (self._read_pos + 1) % self._capacity

        self._count -= to_read
        return bytes(result)

    # ====================================================================
    # Close operations
    # ====================================================================

    def close_read(self) -> None:
        """Close the read end of the pipe.

        After this call, any attempt to write will raise BrokenPipeError,
        because nobody will ever read the data.
        """
        self._readers = 0
        self._closed_read = True

    def close_write(self) -> None:
        """Close the write end of the pipe.

        After this call, once the buffer drains, readers will see EOF.
        This is how shell pipelines terminate: ``cat`` finishes, closes
        its write end, and ``grep`` eventually reads EOF.
        """
        self._writers = 0
        self._closed_write = True

    # ====================================================================
    # Properties -- buffer state queries
    # ====================================================================

    @property
    def is_empty(self) -> bool:
        """True if no data is in the buffer."""
        return self._count == 0

    @property
    def is_full(self) -> bool:
        """True if the buffer is at capacity."""
        return self._count == self._capacity

    @property
    def available(self) -> int:
        """Number of bytes available to read."""
        return self._count

    @property
    def space(self) -> int:
        """Number of bytes of free space available for writing."""
        return self._capacity - self._count

    @property
    def is_eof(self) -> bool:
        """True if no writers remain and the buffer is empty.

        This is the definitive "pipe is done" signal. In a shell pipeline
        like ``ls | grep foo``, EOF occurs when ``ls`` exits (closing its
        write end) and ``grep`` has read all remaining buffered data.
        """
        return (self._writers <= 0 or self._closed_write) and self._count == 0

    @property
    def capacity(self) -> int:
        """The fixed capacity of this pipe's buffer."""
        return self._capacity
