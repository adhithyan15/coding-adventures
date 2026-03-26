"""InodeTable --- the fixed-size array of all inodes.

The inode table is conceptually an array of MAX_INODES slots, each of which
can hold one Inode or be empty (None). When a new file or directory is
created, the file system allocates an inode from this table. When a file is
deleted (and its link_count drops to 0), the inode is freed back to the table.

Analogy: The inode table is like the card catalog in a library. Each drawer
(slot) can hold one catalog card (inode) or be empty. When a new book arrives,
you put a card in the first empty drawer. When a book is discarded, you remove
its card.

Allocation strategy:
    We use first-fit allocation: scan from slot 0 upward and return the first
    empty slot. This is O(n) in the worst case, but with 128 inodes, it is
    effectively instant. Real file systems use more sophisticated strategies
    (e.g., keeping inodes close to their parent directory for better locality).

Important invariant:
    The inode_number stored inside the Inode must always match its index in the
    table. This is enforced by the allocate() method.
"""

from file_system.constants import MAX_INODES
from file_system.inode import FileType, Inode


class InodeTable:
    """Fixed-size table of inodes indexed by inode number.

    Parameters
    ----------
    max_inodes : int
        The maximum number of inodes this table can hold. Each slot is either
        occupied (contains an Inode) or free (None).
    """

    def __init__(self, max_inodes: int = MAX_INODES) -> None:
        self._inodes: list[Inode | None] = [None] * max_inodes
        self._max_inodes = max_inodes

    def allocate(self, file_type: FileType = FileType.REGULAR) -> Inode | None:
        """Allocate a new inode with the given file type.

        Scans the table for the first free slot, creates an Inode with the
        matching inode_number, and stores it.

        Parameters
        ----------
        file_type : FileType
            The type of file this inode represents (REGULAR, DIRECTORY, etc.).

        Returns
        -------
        Inode or None
            The newly allocated inode, or None if all slots are occupied.

        Example:
            >>> table = InodeTable(4)
            >>> inode = table.allocate(FileType.REGULAR)
            >>> inode.inode_number
            0
            >>> inode.file_type
            <FileType.REGULAR: 1>
        """
        for i in range(self._max_inodes):
            if self._inodes[i] is None:
                inode = Inode(inode_number=i, file_type=file_type)
                self._inodes[i] = inode
                return inode
        return None  # All inodes used

    def free(self, inode_num: int) -> None:
        """Free an inode, making its slot available for reuse.

        Parameters
        ----------
        inode_num : int
            The inode number to free. Must be in range [0, max_inodes).

        Raises
        ------
        ValueError
            If inode_num is out of range.
        """
        if inode_num < 0 or inode_num >= self._max_inodes:
            raise ValueError(
                f"Inode number {inode_num} out of range [0, {self._max_inodes})"
            )
        self._inodes[inode_num] = None

    def get(self, inode_num: int) -> Inode | None:
        """Look up an inode by number.

        Parameters
        ----------
        inode_num : int
            The inode number to look up.

        Returns
        -------
        Inode or None
            The inode at that slot, or None if the slot is free.

        Raises
        ------
        ValueError
            If inode_num is out of range.
        """
        if inode_num < 0 or inode_num >= self._max_inodes:
            raise ValueError(
                f"Inode number {inode_num} out of range [0, {self._max_inodes})"
            )
        return self._inodes[inode_num]

    def free_count(self) -> int:
        """Count the number of free (unallocated) inode slots."""
        return sum(1 for inode in self._inodes if inode is None)

    @property
    def max_inodes(self) -> int:
        """The maximum number of inodes this table can hold."""
        return self._max_inodes
