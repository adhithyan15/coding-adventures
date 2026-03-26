"""VFS --- Virtual File System, the main API.

The VFS is the single entry point for all file operations. User programs
never interact with inodes, block bitmaps, or directory entries directly.
Instead, they call VFS methods like ``open()``, ``read()``, ``write()``,
``mkdir()``, and ``unlink()``, and the VFS translates these high-level
requests into low-level block I/O.

Architecture:
    User Program
    |   vfs.open("/data/log.txt", O_RDWR)
    |   vfs.write(fd, b"hello")
    |   vfs.close(fd)
    v
    VFS (this module)
    |   +-- Path Resolution:  "/" -> inode 0 -> "data" -> inode 5 -> ...
    |   +-- Inode Table:      metadata for every file/directory
    |   +-- Block Bitmap:     which data blocks are free/used
    |   +-- Open File Table:  system-wide table of open files
    |   +-- Superblock:       file system metadata
    v
    In-Memory Block Storage (bytearray)
        A flat array of bytes simulating a disk. Divided into BLOCK_SIZE
        chunks. Read/write operations address whole blocks by number.

Flow of a write operation:
    1. Look up fd in the open file table -> get inode number and offset
    2. Get the inode from the inode table -> find which block holds the offset
    3. If the block is not yet allocated, allocate one from the bitmap
    4. Read the existing block (for partial writes)
    5. Overwrite the relevant bytes in the block
    6. Write the block back to storage
    7. Advance the offset and update the inode's size if needed
"""

import struct

from file_system.block_bitmap import BlockBitmap
from file_system.constants import (
    BLOCK_SIZE,
    DIRECT_BLOCKS,
    MAX_BLOCKS,
    MAX_INODES,
    O_APPEND,
    O_CREAT,
    O_RDONLY,
    O_TRUNC,
    O_WRONLY,
    SEEK_CUR,
    SEEK_END,
    SEEK_SET,
)
from file_system.directory import DirectoryEntry
from file_system.file_descriptor import OpenFile, OpenFileTable
from file_system.inode import FileType, Inode
from file_system.inode_table import InodeTable
from file_system.superblock import Superblock


# ---------------------------------------------------------------------------
# How many data blocks are reserved for metadata?
#
# Block 0: superblock
# Blocks 1..N: inode table (we compute N based on inode count)
# Block N+1: block bitmap
# Remaining blocks: data blocks
#
# For simplicity, we keep all metadata in-memory (Superblock, InodeTable,
# BlockBitmap are Python objects) and only use the block storage for actual
# file/directory data. This avoids the complexity of serializing/deserializing
# metadata to specific disk blocks, while still teaching the core concepts.
# ---------------------------------------------------------------------------


class VFS:
    """Virtual File System --- the main API for file operations.

    Uses in-memory block storage (a bytearray) as the backing store. The
    storage is divided into BLOCK_SIZE chunks, each identified by a block
    number. Metadata (superblock, inode table, block bitmap) is kept as
    Python objects rather than serialized to specific blocks, simplifying
    the implementation while preserving all the important concepts.

    Parameters
    ----------
    total_blocks : int
        Number of data blocks available for file/directory data.
    total_inodes : int
        Maximum number of inodes (files + directories).
    """

    def __init__(
        self,
        total_blocks: int = MAX_BLOCKS,
        total_inodes: int = MAX_INODES,
    ) -> None:
        # The raw block storage --- a flat byte array simulating a disk.
        # Each block is BLOCK_SIZE bytes. Block i starts at offset i * BLOCK_SIZE.
        self._storage = bytearray(total_blocks * BLOCK_SIZE)

        # Metadata structures (kept in memory, not serialized to storage)
        self._superblock = Superblock(
            total_blocks=total_blocks,
            total_inodes=total_inodes,
            free_blocks=total_blocks,
            free_inodes=total_inodes,
        )
        self._inode_table = InodeTable(total_inodes)
        self._block_bitmap = BlockBitmap(total_blocks)
        self._open_file_table = OpenFileTable()

        # Track whether format() has been called
        self._formatted = False

    # =======================================================================
    # Format --- initialize a blank file system
    # =======================================================================

    def format(self) -> None:
        """Initialize the file system: create superblock, root directory.

        This is the equivalent of ``mkfs.ext2`` --- it takes a blank disk and
        writes the data structures needed for a functioning file system.

        Steps:
            1. Allocate inode 0 as the root directory (type = DIRECTORY).
            2. Allocate one data block for the root directory's entries.
            3. Write the initial directory entries ("." and "..") to that block.
            4. Update the superblock's free counts.
        """
        # Step 1: allocate inode 0 for the root directory "/"
        root_inode = self._inode_table.allocate(FileType.DIRECTORY)
        if root_inode is None:
            raise RuntimeError("Failed to allocate root inode")

        # Step 2: allocate a data block for root's directory entries
        root_block = self._block_bitmap.allocate()
        if root_block is None:
            raise RuntimeError("Failed to allocate block for root directory")
        root_inode.direct_blocks[0] = root_block

        # Step 3: create "." and ".." entries (both point to inode 0 for root)
        entries = [
            DirectoryEntry(name=".", inode_number=0),
            DirectoryEntry(name="..", inode_number=0),
        ]
        self._write_dir_entries(root_inode, entries)

        # Step 4: update superblock
        self._superblock.free_blocks = self._block_bitmap.free_count()
        self._superblock.free_inodes = self._inode_table.free_count()

        self._formatted = True

    # =======================================================================
    # Open --- open a file by path
    # =======================================================================

    def open(self, path: str, flags: int = O_RDONLY) -> int:
        """Open a file by path. Create it if O_CREAT is set and it doesn't exist.

        The open operation resolves the path to an inode, creates an OpenFile
        entry in the system-wide table, and returns a file descriptor (fd).

        Parameters
        ----------
        path : str
            Absolute path to the file (e.g., "/data/log.txt").
        flags : int
            Open flags (O_RDONLY, O_WRONLY, O_RDWR, optionally OR'd with
            O_CREAT, O_TRUNC, O_APPEND).

        Returns
        -------
        int
            A file descriptor (fd >= 3) on success, -1 on error.
        """
        inode = self.resolve_path(path)

        if inode is None:
            # File doesn't exist --- create it if O_CREAT is set
            if not (flags & O_CREAT):
                return -1  # File not found and O_CREAT not set

            # Create the file: allocate inode, add entry to parent dir
            parent_path, filename = self._split_path(path)
            if not filename:
                return -1  # Cannot create root or empty name

            parent_inode = self.resolve_path(parent_path)
            if parent_inode is None:
                return -1  # Parent directory doesn't exist
            if parent_inode.file_type != FileType.DIRECTORY:
                return -1  # Parent is not a directory

            # Allocate a new inode for the file
            new_inode = self._inode_table.allocate(FileType.REGULAR)
            if new_inode is None:
                return -1  # No free inodes

            # Add directory entry in parent
            entries = self._read_dir_entries(parent_inode)
            entries.append(
                DirectoryEntry(name=filename, inode_number=new_inode.inode_number)
            )
            self._write_dir_entries(parent_inode, entries)

            self._superblock.free_inodes = self._inode_table.free_count()
            inode = new_inode

        # Handle O_TRUNC: truncate file to zero length
        if flags & O_TRUNC and inode.file_type == FileType.REGULAR:
            self._truncate_inode(inode)

        # Create open file entry
        fd = self._open_file_table.open(inode.inode_number, flags)

        # Handle O_APPEND: set offset to end of file
        if flags & O_APPEND:
            open_file = self._open_file_table.get(fd)
            if open_file is not None:
                open_file.offset = inode.size

        return fd

    # =======================================================================
    # Close --- close a file descriptor
    # =======================================================================

    def close(self, fd: int) -> int:
        """Close a file descriptor.

        Decrements the reference count on the OpenFile entry. When the
        ref_count drops to 0, the entry is freed.

        Parameters
        ----------
        fd : int
            The file descriptor to close.

        Returns
        -------
        int
            0 on success, -1 on error (invalid fd).
        """
        if self._open_file_table.close(fd):
            return 0
        return -1

    # =======================================================================
    # Read --- read bytes from an open file
    # =======================================================================

    def read(self, fd: int, count: int) -> bytes:
        """Read up to ``count`` bytes from the file at the current offset.

        The read operation works by:
            1. Looking up the fd in the open file table to get the inode
               and current offset.
            2. Calculating which block(s) contain the requested bytes.
            3. Reading those blocks from storage.
            4. Extracting the relevant bytes.
            5. Advancing the offset.

        Parameters
        ----------
        fd : int
            An open file descriptor.
        count : int
            Maximum number of bytes to read.

        Returns
        -------
        bytes
            The data read (may be shorter than ``count`` if EOF is reached).
            Empty bytes if fd is invalid or at EOF.
        """
        open_file = self._open_file_table.get(fd)
        if open_file is None:
            return b""

        # Check that reading is permitted
        access_mode = open_file.flags & 0x3  # low 2 bits = access mode
        if access_mode == O_WRONLY:
            return b""  # Cannot read a write-only fd

        inode = self._inode_table.get(open_file.inode_number)
        if inode is None:
            return b""

        # Don't read past end of file
        remaining = inode.size - open_file.offset
        if remaining <= 0:
            return b""
        bytes_to_read = min(count, remaining)

        result = bytearray()
        bytes_read = 0

        while bytes_read < bytes_to_read:
            # Which block and offset within that block?
            block_index = open_file.offset // BLOCK_SIZE
            byte_within_block = open_file.offset % BLOCK_SIZE

            # How many bytes can we read from this block?
            available_in_block = BLOCK_SIZE - byte_within_block
            chunk_size = min(available_in_block, bytes_to_read - bytes_read)

            # Get the actual block number from the inode's pointers
            block_num = self._get_block_number(inode, block_index)
            if block_num is None or block_num == -1:
                break  # No more data

            # Read the block and extract the relevant bytes
            block_data = self._read_block(block_num)
            result.extend(
                block_data[byte_within_block : byte_within_block + chunk_size]
            )

            open_file.offset += chunk_size
            bytes_read += chunk_size

        return bytes(result)

    # =======================================================================
    # Write --- write bytes to an open file
    # =======================================================================

    def write(self, fd: int, data: bytes) -> int:
        """Write data to the file at the current offset.

        The write operation allocates new blocks as needed. If the file
        grows beyond its current allocated blocks, the block bitmap is
        consulted to find free blocks.

        Parameters
        ----------
        fd : int
            An open file descriptor.
        data : bytes
            The data to write.

        Returns
        -------
        int
            Number of bytes written, or -1 on error.
        """
        open_file = self._open_file_table.get(fd)
        if open_file is None:
            return -1

        # Check that writing is permitted
        access_mode = open_file.flags & 0x3
        if access_mode == O_RDONLY:
            return -1  # Cannot write to a read-only fd

        inode = self._inode_table.get(open_file.inode_number)
        if inode is None:
            return -1

        # Handle O_APPEND: always write at end
        if open_file.flags & O_APPEND:
            open_file.offset = inode.size

        bytes_written = 0
        total = len(data)

        while bytes_written < total:
            block_index = open_file.offset // BLOCK_SIZE
            byte_within_block = open_file.offset % BLOCK_SIZE

            # Ensure a block is allocated for this position
            block_num = self._get_block_number(inode, block_index)
            if block_num is None or block_num == -1:
                # Need to allocate a new block
                block_num = self._allocate_block_for_inode(inode, block_index)
                if block_num is None:
                    break  # Disk full

            # Read existing block (for partial overwrites)
            block_data = bytearray(self._read_block(block_num))

            # Calculate how much to write into this block
            available_in_block = BLOCK_SIZE - byte_within_block
            chunk_size = min(available_in_block, total - bytes_written)

            # Write the data into the block
            block_data[byte_within_block : byte_within_block + chunk_size] = data[
                bytes_written : bytes_written + chunk_size
            ]

            # Write the block back to storage
            self._write_block(block_num, bytes(block_data))

            open_file.offset += chunk_size
            bytes_written += chunk_size

            # Update file size if we extended the file
            if open_file.offset > inode.size:
                inode.size = open_file.offset

        self._superblock.free_blocks = self._block_bitmap.free_count()
        return bytes_written

    # =======================================================================
    # Lseek --- reposition the file offset
    # =======================================================================

    def lseek(self, fd: int, offset: int, whence: int = SEEK_SET) -> int:
        """Reposition the read/write offset for an open file.

        Parameters
        ----------
        fd : int
            An open file descriptor.
        offset : int
            The offset value (interpretation depends on ``whence``).
        whence : int
            SEEK_SET (0): set offset to ``offset``.
            SEEK_CUR (1): set offset to current + ``offset``.
            SEEK_END (2): set offset to file_size + ``offset``.

        Returns
        -------
        int
            The new offset on success, -1 on error.
        """
        open_file = self._open_file_table.get(fd)
        if open_file is None:
            return -1

        inode = self._inode_table.get(open_file.inode_number)
        if inode is None:
            return -1

        if whence == SEEK_SET:
            new_offset = offset
        elif whence == SEEK_CUR:
            new_offset = open_file.offset + offset
        elif whence == SEEK_END:
            new_offset = inode.size + offset
        else:
            return -1  # Invalid whence

        if new_offset < 0:
            return -1  # Cannot seek before beginning

        open_file.offset = new_offset
        return new_offset

    # =======================================================================
    # Stat --- get file metadata
    # =======================================================================

    def stat(self, path: str) -> Inode | None:
        """Get the inode (metadata) for a file or directory.

        Parameters
        ----------
        path : str
            Absolute path to the file or directory.

        Returns
        -------
        Inode or None
            The inode if the path exists, None otherwise.
        """
        return self.resolve_path(path)

    # =======================================================================
    # Mkdir --- create a directory
    # =======================================================================

    def mkdir(self, path: str, permissions: int = 0o755) -> int:
        """Create a new directory.

        Steps:
            1. Resolve the parent directory.
            2. Allocate a new inode (type = DIRECTORY).
            3. Allocate a data block for the new directory's entries.
            4. Write "." and ".." entries to the new directory.
            5. Add an entry for the new directory in the parent.
            6. Increment the parent's link_count (because ".." points to it).

        Parameters
        ----------
        path : str
            Absolute path of the directory to create (e.g., "/home/alice").
        permissions : int
            Permission bits (default 0o755).

        Returns
        -------
        int
            0 on success, -1 on error.
        """
        # Check if path already exists
        if self.resolve_path(path) is not None:
            return -1  # Already exists

        parent_path, dirname = self._split_path(path)
        if not dirname:
            return -1  # Cannot create root or empty name

        parent_inode = self.resolve_path(parent_path)
        if parent_inode is None:
            return -1  # Parent doesn't exist
        if parent_inode.file_type != FileType.DIRECTORY:
            return -1  # Parent is not a directory

        # Allocate inode for new directory
        new_inode = self._inode_table.allocate(FileType.DIRECTORY)
        if new_inode is None:
            return -1  # No free inodes
        new_inode.permissions = permissions

        # Allocate a data block for the new directory's entries
        new_block = self._block_bitmap.allocate()
        if new_block is None:
            # Rollback inode allocation
            self._inode_table.free(new_inode.inode_number)
            return -1  # Disk full
        new_inode.direct_blocks[0] = new_block

        # Write "." and ".." entries
        dir_entries = [
            DirectoryEntry(name=".", inode_number=new_inode.inode_number),
            DirectoryEntry(name="..", inode_number=parent_inode.inode_number),
        ]
        self._write_dir_entries(new_inode, dir_entries)

        # The new directory has link_count = 2 ("." from itself and the
        # entry in the parent directory).
        new_inode.link_count = 2

        # Add entry in parent directory
        parent_entries = self._read_dir_entries(parent_inode)
        parent_entries.append(
            DirectoryEntry(name=dirname, inode_number=new_inode.inode_number)
        )
        self._write_dir_entries(parent_inode, parent_entries)

        # Parent gains a link (from the new directory's ".." entry)
        parent_inode.link_count += 1

        # Update superblock
        self._superblock.free_blocks = self._block_bitmap.free_count()
        self._superblock.free_inodes = self._inode_table.free_count()

        return 0

    # =======================================================================
    # Readdir --- list directory entries
    # =======================================================================

    def readdir(self, path: str) -> list[DirectoryEntry]:
        """List the entries in a directory.

        Parameters
        ----------
        path : str
            Absolute path to a directory.

        Returns
        -------
        list[DirectoryEntry]
            The directory entries, or an empty list if the path does not
            exist or is not a directory.
        """
        inode = self.resolve_path(path)
        if inode is None:
            return []
        if inode.file_type != FileType.DIRECTORY:
            return []
        return self._read_dir_entries(inode)

    # =======================================================================
    # Unlink --- remove a file
    # =======================================================================

    def unlink(self, path: str) -> int:
        """Remove a file (directory entry + possibly inode and blocks).

        Steps:
            1. Resolve the parent directory.
            2. Find the entry in the parent and remove it.
            3. Decrement the target inode's link_count.
            4. If link_count reaches 0, free the inode and all its blocks.

        Parameters
        ----------
        path : str
            Absolute path to the file to remove.

        Returns
        -------
        int
            0 on success, -1 on error.
        """
        parent_path, filename = self._split_path(path)
        if not filename:
            return -1  # Cannot unlink root

        parent_inode = self.resolve_path(parent_path)
        if parent_inode is None:
            return -1

        # Find the entry in the parent directory
        entries = self._read_dir_entries(parent_inode)
        target_entry = None
        for entry in entries:
            if entry.name == filename:
                target_entry = entry
                break

        if target_entry is None:
            return -1  # File not found

        target_inode = self._inode_table.get(target_entry.inode_number)
        if target_inode is None:
            return -1

        # Don't allow unlinking directories with unlink (use rmdir instead)
        if target_inode.file_type == FileType.DIRECTORY:
            return -1

        # Remove the entry from the parent
        entries = [e for e in entries if e.name != filename]
        self._write_dir_entries(parent_inode, entries)

        # Decrement link count
        target_inode.link_count -= 1

        if target_inode.link_count <= 0:
            # Free all data blocks
            self._free_inode_blocks(target_inode)
            # Free the inode
            self._inode_table.free(target_inode.inode_number)

        # Update superblock
        self._superblock.free_blocks = self._block_bitmap.free_count()
        self._superblock.free_inodes = self._inode_table.free_count()

        return 0

    # =======================================================================
    # Resolve path --- walk the directory tree
    # =======================================================================

    def resolve_path(self, path: str) -> Inode | None:
        """Resolve an absolute path to its inode.

        Algorithm:
            1. Start at the root inode (inode 0).
            2. Split the path by "/" and iterate over each component.
            3. For each component, verify the current inode is a directory.
            4. Read the directory's entries and search for the component name.
            5. If found, move to that entry's inode and continue.
            6. If not found, return None.

        Example trace for "/home/alice/notes.txt":
            Component    | Current Inode | Action
            (start)      | 0 (root)      | Begin at root
            "home"       | 0 -> 5        | Found "home" -> inode 5
            "alice"      | 5 -> 12       | Found "alice" -> inode 12
            "notes.txt"  | 12 -> 23      | Found "notes.txt" -> inode 23
            Result: inode 23

        Parameters
        ----------
        path : str
            Absolute path (must start with "/").

        Returns
        -------
        Inode or None
            The inode at the end of the path, or None if any component
            is not found.
        """
        if not path or path[0] != "/":
            return None

        # Root directory case
        current_inode = self._inode_table.get(0)
        if current_inode is None:
            return None

        # Split path and filter empty strings (from leading "/" and trailing "/")
        components = [c for c in path.split("/") if c]

        # If path is just "/", return root
        if not components:
            return current_inode

        for component in components:
            # Current inode must be a directory to descend into
            if current_inode.file_type != FileType.DIRECTORY:
                return None

            # Search directory entries for the component
            entries = self._read_dir_entries(current_inode)
            found = False
            for entry in entries:
                if entry.name == component:
                    current_inode = self._inode_table.get(entry.inode_number)
                    if current_inode is None:
                        return None
                    found = True
                    break

            if not found:
                return None

        return current_inode

    # =======================================================================
    # Superblock access
    # =======================================================================

    @property
    def superblock(self) -> Superblock:
        """Access the file system's superblock (read-only metadata)."""
        return self._superblock

    # =======================================================================
    # Internal helpers --- block I/O
    # =======================================================================

    def _read_block(self, block_num: int) -> bytes:
        """Read a single block from storage.

        Parameters
        ----------
        block_num : int
            The block number to read (0-indexed).

        Returns
        -------
        bytes
            Exactly BLOCK_SIZE bytes of data.
        """
        start = block_num * BLOCK_SIZE
        return bytes(self._storage[start : start + BLOCK_SIZE])

    def _write_block(self, block_num: int, data: bytes) -> None:
        """Write data to a single block in storage.

        The data is padded with zeros if shorter than BLOCK_SIZE, or
        truncated if longer.

        Parameters
        ----------
        block_num : int
            The block number to write to.
        data : bytes
            The data to write (will be padded/truncated to BLOCK_SIZE).
        """
        start = block_num * BLOCK_SIZE
        # Pad or truncate to exactly BLOCK_SIZE
        padded = data[:BLOCK_SIZE].ljust(BLOCK_SIZE, b"\x00")
        self._storage[start : start + BLOCK_SIZE] = padded

    # =======================================================================
    # Internal helpers --- directory I/O
    # =======================================================================

    def _read_dir_entries(self, inode: Inode) -> list[DirectoryEntry]:
        """Read all directory entries from an inode's data blocks.

        Directory entries are stored as serialized text in the inode's data
        blocks. This method reads all allocated blocks and parses the entries.

        Parameters
        ----------
        inode : Inode
            The directory inode whose entries to read.

        Returns
        -------
        list[DirectoryEntry]
            The parsed directory entries.
        """
        entries: list[DirectoryEntry] = []

        # Read data from all allocated blocks
        raw_data = bytearray()
        for i in range(DIRECT_BLOCKS):
            block_num = inode.direct_blocks[i]
            if block_num == -1:
                break
            raw_data.extend(self._read_block(block_num))

        # Also check indirect block
        if inode.indirect_block != -1:
            indirect_data = self._read_block(inode.indirect_block)
            for j in range(0, BLOCK_SIZE, 4):
                ptr = struct.unpack("<i", indirect_data[j : j + 4])[0]
                if ptr == -1 or ptr == 0:
                    break
                raw_data.extend(self._read_block(ptr))

        # Parse entries from the raw text data
        text = raw_data.decode("utf-8", errors="replace").rstrip("\x00")
        for line in text.split("\n"):
            line = line.strip()
            if line and ":" in line:
                try:
                    entries.append(DirectoryEntry.deserialize(line))
                except (ValueError, IndexError):
                    continue

        return entries

    def _write_dir_entries(
        self, inode: Inode, entries: list[DirectoryEntry]
    ) -> None:
        """Write directory entries to an inode's data blocks.

        Serializes all entries to text and writes them to the inode's blocks,
        allocating additional blocks if needed.

        Parameters
        ----------
        inode : Inode
            The directory inode to write entries into.
        entries : list[DirectoryEntry]
            The entries to write.
        """
        # Serialize all entries
        data = "".join(entry.serialize() for entry in entries).encode("utf-8")
        inode.size = len(data)

        # Write to blocks
        offset = 0
        block_index = 0
        while offset < len(data):
            chunk = data[offset : offset + BLOCK_SIZE]

            # Get or allocate a block for this index
            block_num = self._get_block_number(inode, block_index)
            if block_num is None or block_num == -1:
                block_num = self._allocate_block_for_inode(inode, block_index)
                if block_num is None:
                    return  # Disk full, partial write

            self._write_block(block_num, chunk)
            offset += BLOCK_SIZE
            block_index += 1

    # =======================================================================
    # Internal helpers --- block allocation and lookup
    # =======================================================================

    def _get_block_number(self, inode: Inode, block_index: int) -> int | None:
        """Get the physical block number for a logical block index.

        Block indices 0-11 map to direct_blocks[0-11].
        Block indices 12+ map to pointers stored in the indirect block.

        Parameters
        ----------
        inode : Inode
            The inode to look up.
        block_index : int
            The logical block index within the file.

        Returns
        -------
        int or None
            The physical block number, -1 if not allocated, or None if the
            index is out of range.
        """
        if block_index < DIRECT_BLOCKS:
            return inode.direct_blocks[block_index]
        elif block_index < DIRECT_BLOCKS + (BLOCK_SIZE // 4):
            # Indirect block
            if inode.indirect_block == -1:
                return -1
            indirect_data = self._read_block(inode.indirect_block)
            ptr_offset = (block_index - DIRECT_BLOCKS) * 4
            ptr = struct.unpack("<i", indirect_data[ptr_offset : ptr_offset + 4])[0]
            return ptr
        else:
            return None  # Beyond our addressing capability

    def _allocate_block_for_inode(
        self, inode: Inode, block_index: int
    ) -> int | None:
        """Allocate a new data block and assign it to the given block index.

        If the block index falls in the direct range (0-11), the block number
        is stored directly in the inode. If it falls in the indirect range
        (12+), the indirect block is allocated if needed, and the pointer is
        stored there.

        Parameters
        ----------
        inode : Inode
            The inode to allocate a block for.
        block_index : int
            The logical block index within the file.

        Returns
        -------
        int or None
            The allocated block number, or None if the disk is full or the
            index is out of range.
        """
        new_block = self._block_bitmap.allocate()
        if new_block is None:
            return None  # Disk full

        if block_index < DIRECT_BLOCKS:
            inode.direct_blocks[block_index] = new_block
            return new_block
        elif block_index < DIRECT_BLOCKS + (BLOCK_SIZE // 4):
            # Need to use indirect block
            if inode.indirect_block == -1:
                # Allocate the indirect block itself
                indirect_block = self._block_bitmap.allocate()
                if indirect_block is None:
                    # Can't allocate indirect block, free the data block
                    self._block_bitmap.free(new_block)
                    return None
                inode.indirect_block = indirect_block
                # Initialize indirect block with -1 pointers
                init_data = struct.pack("<i", -1) * (BLOCK_SIZE // 4)
                self._write_block(indirect_block, init_data)

            # Store the pointer in the indirect block
            indirect_data = bytearray(self._read_block(inode.indirect_block))
            ptr_offset = (block_index - DIRECT_BLOCKS) * 4
            struct.pack_into("<i", indirect_data, ptr_offset, new_block)
            self._write_block(inode.indirect_block, bytes(indirect_data))

            return new_block
        else:
            # Beyond our addressing capability
            self._block_bitmap.free(new_block)
            return None

    def _truncate_inode(self, inode: Inode) -> None:
        """Truncate a file to zero length, freeing all its data blocks.

        Parameters
        ----------
        inode : Inode
            The inode to truncate.
        """
        self._free_inode_blocks(inode)
        inode.size = 0

    def _free_inode_blocks(self, inode: Inode) -> None:
        """Free all data blocks and the indirect block owned by an inode.

        Parameters
        ----------
        inode : Inode
            The inode whose blocks to free.
        """
        # Free direct blocks
        for i in range(DIRECT_BLOCKS):
            if inode.direct_blocks[i] != -1:
                self._block_bitmap.free(inode.direct_blocks[i])
                inode.direct_blocks[i] = -1

        # Free indirect block and its pointers
        if inode.indirect_block != -1:
            indirect_data = self._read_block(inode.indirect_block)
            for j in range(0, BLOCK_SIZE, 4):
                ptr = struct.unpack("<i", indirect_data[j : j + 4])[0]
                if ptr != -1 and ptr != 0:
                    self._block_bitmap.free(ptr)

            self._block_bitmap.free(inode.indirect_block)
            inode.indirect_block = -1

    def _split_path(self, path: str) -> tuple[str, str]:
        """Split an absolute path into (parent_path, basename).

        Examples:
            "/home/alice/notes.txt" -> ("/home/alice", "notes.txt")
            "/home"                -> ("/", "home")
            "/"                    -> ("/", "")

        Parameters
        ----------
        path : str
            An absolute path.

        Returns
        -------
        tuple[str, str]
            (parent_path, basename)
        """
        path = path.rstrip("/")
        if not path or path == "/":
            return "/", ""
        last_slash = path.rfind("/")
        if last_slash == 0:
            return "/", path[1:]
        return path[:last_slash], path[last_slash + 1 :]
