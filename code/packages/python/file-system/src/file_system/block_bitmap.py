"""BlockBitmap --- tracking free and used blocks.

A block bitmap is one of the simplest and most elegant data structures in
file system design. It uses a single bit to represent the state of each
data block on disk:

    0 = free (available for allocation)
    1 = used (contains file data, directory entries, or indirect pointers)

With 512 data blocks, we need 512 bits = 64 bytes for the entire bitmap.
That fits comfortably in a single 512-byte block on disk.

Visual representation:
    Bit index:  0   1   2   3   4   5   6   7   8   9  ...
    Value:      1   1   1   0   0   1   0   0   0   0  ...
                ^   ^   ^           ^
                |   |   |           |
              used used used      used        (rest are free)

Why a bitmap? Because it is:
    1. Space-efficient: 1 bit per block vs. e.g., 4 bytes for a free list entry.
    2. Fast allocation: scanning for a 0 bit is O(n) in the worst case, but
       with simple optimizations (remember where the last allocation was) it
       approaches O(1) for sequential allocation.
    3. Easy to understand and implement correctly.

Alternative: A free list (linked list of free blocks). Ext2 uses bitmaps;
FAT uses a table that doubles as both a free list and a linked list of
block chains. We use bitmaps because they are simpler.

Implementation note: We use a bytearray instead of actual bits for clarity.
Each element is 0 or 1. A production file system would pack 8 blocks into
each byte, but our approach makes the code much more readable.
"""


class BlockBitmap:
    """Tracks which data blocks are free (0) vs. allocated (1).

    The bitmap is initialized with all blocks free. The ``allocate()`` method
    finds the first free block, marks it as used, and returns its index. The
    ``free()`` method marks a block as available again.

    Parameters
    ----------
    total_blocks : int
        The number of data blocks to track. Each block gets one entry in the
        bitmap (0 = free, 1 = used).
    """

    def __init__(self, total_blocks: int) -> None:
        self._bitmap = bytearray(total_blocks)  # All zeros = all free
        self._total_blocks = total_blocks

    def allocate(self) -> int | None:
        """Find the first free block, mark it as used, and return its index.

        Scans the bitmap from index 0 upward, looking for the first 0 entry.
        When found, sets it to 1 (used) and returns the block index.

        Returns
        -------
        int or None
            The index of the newly allocated block, or None if the disk is
            full (all blocks are used).

        Example:
            >>> bm = BlockBitmap(10)
            >>> bm.allocate()
            0
            >>> bm.allocate()
            1
            >>> bm.free(0)
            >>> bm.allocate()  # reuses block 0
            0
        """
        for i in range(self._total_blocks):
            if self._bitmap[i] == 0:
                self._bitmap[i] = 1
                return i
        return None  # Disk full

    def free(self, block_num: int) -> None:
        """Mark a block as free (available for reuse).

        Parameters
        ----------
        block_num : int
            The block index to free. Must be in range [0, total_blocks).

        Raises
        ------
        ValueError
            If block_num is out of range.
        """
        if block_num < 0 or block_num >= self._total_blocks:
            raise ValueError(
                f"Block number {block_num} out of range [0, {self._total_blocks})"
            )
        self._bitmap[block_num] = 0

    def is_free(self, block_num: int) -> bool:
        """Check whether a block is free (unallocated).

        Parameters
        ----------
        block_num : int
            The block index to check.

        Returns
        -------
        bool
            True if the block is free, False if it is allocated.
        """
        if block_num < 0 or block_num >= self._total_blocks:
            raise ValueError(
                f"Block number {block_num} out of range [0, {self._total_blocks})"
            )
        return self._bitmap[block_num] == 0

    def free_count(self) -> int:
        """Count the number of free (unallocated) blocks.

        Returns
        -------
        int
            The number of blocks whose bitmap entry is 0 (free).
        """
        return self._bitmap.count(0)

    @property
    def total_blocks(self) -> int:
        """The total number of blocks tracked by this bitmap."""
        return self._total_blocks
