"""PhysicalFrameAllocator — bitmap-based physical frame manager.

Physical memory (RAM) is divided into fixed-size chunks called FRAMES.
Each frame is the same size as a virtual page: 4 KB (4096 bytes).

For example, a machine with 16 MB of RAM has:
    16 MB / 4 KB = 4096 frames

    Frame 0:    bytes 0x00000000 - 0x00000FFF
    Frame 1:    bytes 0x00001000 - 0x00001FFF
    Frame 2:    bytes 0x00002000 - 0x00002FFF
    ...
    Frame 4095: bytes 0x00FFF000 - 0x00FFFFFF

The frame allocator tracks which frames are free and which are in use.
It uses a BITMAP: an array where each element represents one frame.

    bitmap[i] = 0  ->  frame i is free (available for allocation)
    bitmap[i] = 1  ->  frame i is allocated (in use by some process)

Example bitmap for 16 frames:
    [1, 1, 1, 0, 0, 1, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0]
     ^  ^  ^         ^         ^  ^
     |  |  |         |         |  |
     kernel frames   process A  process B

Allocation scans the bitmap linearly for the first free frame. This is
O(n) in the worst case. Real OS kernels use more sophisticated data
structures (free lists, buddy allocators) for O(1) allocation, but the
bitmap is simple and educational.
"""


class PhysicalFrameAllocator:
    """Bitmap-based physical frame allocator.

    Manages a fixed number of physical frames. Supports allocate (find and
    claim a free frame), free (release a frame), and query operations.

    Example:
        >>> alloc = PhysicalFrameAllocator(total_frames=8)
        >>> alloc.free_count()
        8
        >>> frame = alloc.allocate()
        >>> frame
        0
        >>> alloc.is_allocated(0)
        True
        >>> alloc.free(0)
        >>> alloc.is_allocated(0)
        False
    """

    def __init__(self, total_frames: int) -> None:
        """Initialize the allocator with all frames free.

        Args:
            total_frames: Total number of physical frames to manage.
                For a machine with N bytes of RAM: total_frames = N / 4096.

        Raises:
            ValueError: If total_frames is not positive.
        """
        if total_frames <= 0:
            msg = f"total_frames must be positive, got {total_frames}"
            raise ValueError(msg)

        # The bitmap: 0 = free, 1 = allocated.
        # Using a bytearray gives us mutable, compact storage.
        # Each element is one byte but we only use values 0 and 1.
        # A real OS would use actual bits (1 bit per frame), but bytes
        # are clearer for educational purposes.
        self._bitmap: bytearray = bytearray(total_frames)

        self._total: int = total_frames

    def allocate(self) -> int | None:
        """Find and allocate the first free frame.

        Scans the bitmap from frame 0 upward, looking for a frame marked
        as free (0). When found, marks it as allocated (1) and returns its
        frame number.

        This linear scan is O(n) where n is the total number of frames.
        Real allocators maintain a free list for O(1) allocation, but the
        bitmap scan is simpler to understand.

        Returns:
            The frame number of the newly allocated frame, or None if all
            frames are in use (out of memory).
        """
        for i in range(self._total):
            if self._bitmap[i] == 0:
                self._bitmap[i] = 1
                return i
        return None

    def free(self, frame: int) -> None:
        """Free a previously allocated frame.

        Marks the frame as free (0) in the bitmap, making it available for
        future allocation.

        Args:
            frame: The frame number to free. Must be currently allocated.

        Raises:
            ValueError: If the frame number is out of range.
            RuntimeError: If the frame is already free (double-free bug).
        """
        if frame < 0 or frame >= self._total:
            msg = f"Frame {frame} out of range [0, {self._total})"
            raise ValueError(msg)

        if self._bitmap[frame] == 0:
            msg = (
                f"Frame {frame} is already free — double-free detected! "
                f"Double-free is always a bug: it means some code freed "
                f"a frame that was already returned to the allocator."
            )
            raise RuntimeError(msg)

        self._bitmap[frame] = 0

    def is_allocated(self, frame: int) -> bool:
        """Check whether a frame is currently allocated.

        Args:
            frame: The frame number to check.

        Returns:
            True if the frame is in use, False if it is free.

        Raises:
            ValueError: If the frame number is out of range.
        """
        if frame < 0 or frame >= self._total:
            msg = f"Frame {frame} out of range [0, {self._total})"
            raise ValueError(msg)
        return self._bitmap[frame] == 1

    def free_count(self) -> int:
        """Return the number of free (unallocated) frames.

        Returns:
            Count of frames with bitmap value 0.
        """
        return self._bitmap.count(0)

    def allocated_count(self) -> int:
        """Return the number of allocated (in-use) frames.

        Returns:
            Count of frames with bitmap value 1.
        """
        return self._total - self.free_count()

    @property
    def total_frames(self) -> int:
        """Return the total number of frames managed by this allocator."""
        return self._total
