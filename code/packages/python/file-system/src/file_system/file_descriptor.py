"""File descriptors --- the process's view of open files.

When a process calls open("/data/log.txt"), it does not get back an inode
number or a block pointer. It gets a small integer --- a file descriptor (fd).
File descriptors are the process's *handle* to an open file, abstracting away
all the details of inodes and blocks.

There are two levels of indirection:

    Process A                          System-Wide
    +---------------------+           +------------------------------+
    | FileDescriptorTable |           | OpenFileTable                |
    | (per-process)       |           | (shared by all processes)    |
    |                     |           |                              |
    | fd 0 --------------------------------> entry 0: stdin          |
    | fd 1 --------------------------------> entry 1: stdout         |
    | fd 2 --------------------------------> entry 2: stderr         |
    | fd 3 --------------------------------> entry 5: inode=23       |
    +---------------------+           +------------------------------+

Why two tables?
    Because multiple processes can share the same open file entry (after
    fork()), and the same file can have multiple open file entries (opened
    independently by different processes). The two-level structure supports
    both sharing and independence.

Standard file descriptors:
    fd 0 = stdin   --- standard input  (keyboard by default)
    fd 1 = stdout  --- standard output (display by default)
    fd 2 = stderr  --- standard error  (display by default)
    fd 3+ = files opened by the process via open()

dup() and dup2():
    dup(fd)        --- create a new fd pointing to the same OpenFile entry
    dup2(old, new) --- make new_fd point to the same OpenFile entry as old_fd

    These are essential for I/O redirection in shells:
        echo hello > output.txt
        1. Shell opens output.txt -> fd 3
        2. Shell calls dup2(3, 1) -> fd 1 now points to output.txt
        3. Shell closes fd 3
        4. Child process writes to fd 1 (stdout) -> goes to file
"""

from dataclasses import dataclass

from file_system.constants import O_RDONLY


@dataclass
class OpenFile:
    """A system-wide entry representing one *opening* of a file.

    Multiple file descriptors (across multiple processes) can point to the
    same OpenFile entry. They share the same offset, so if one process reads
    5 bytes, the other process's next read starts 5 bytes later. This is the
    behavior after fork().

    Parameters
    ----------
    inode_number : int
        Which file this entry refers to (index into the inode table).
    offset : int
        Current read/write position within the file, in bytes. Starts at 0
        and advances with each read/write.
    flags : int
        How the file was opened (O_RDONLY, O_WRONLY, O_RDWR, etc.).
    ref_count : int
        How many file descriptors point to this entry. When ref_count drops
        to 0, the entry is removed from the OpenFileTable.
    """

    inode_number: int
    offset: int = 0
    flags: int = O_RDONLY
    ref_count: int = 1


class OpenFileTable:
    """System-wide table of all open files.

    This table is shared by all processes. Each entry is an OpenFile with an
    inode number, current offset, flags, and reference count. The table
    assigns unique integer keys (which we call system-wide file descriptors)
    to each entry.

    File descriptors 0, 1, and 2 are reserved for stdin, stdout, and stderr
    by convention. New entries start at fd 3.
    """

    def __init__(self) -> None:
        self._entries: dict[int, OpenFile] = {}
        self._next_fd: int = 3  # 0=stdin, 1=stdout, 2=stderr reserved

    def open(self, inode_number: int, flags: int) -> int:
        """Create a new open file entry and return its file descriptor.

        Parameters
        ----------
        inode_number : int
            The inode of the file being opened.
        flags : int
            The open flags (O_RDONLY, O_WRONLY, O_RDWR, etc.).

        Returns
        -------
        int
            The file descriptor assigned to this entry.
        """
        fd = self._next_fd
        self._entries[fd] = OpenFile(
            inode_number=inode_number, offset=0, flags=flags
        )
        self._next_fd += 1
        return fd

    def close(self, fd: int) -> bool:
        """Close a file descriptor.

        Decrements the reference count on the OpenFile entry. If ref_count
        drops to 0, the entry is removed entirely.

        Parameters
        ----------
        fd : int
            The file descriptor to close.

        Returns
        -------
        bool
            True if the fd existed and was closed, False if the fd was
            not found.
        """
        entry = self._entries.get(fd)
        if entry is None:
            return False
        entry.ref_count -= 1
        if entry.ref_count <= 0:
            del self._entries[fd]
        return True

    def get(self, fd: int) -> OpenFile | None:
        """Look up an open file entry by file descriptor.

        Parameters
        ----------
        fd : int
            The file descriptor to look up.

        Returns
        -------
        OpenFile or None
            The entry if found, None otherwise.
        """
        return self._entries.get(fd)

    def dup(self, fd: int) -> int | None:
        """Duplicate a file descriptor.

        Creates a new fd that points to the same OpenFile entry as the
        original. The ref_count on the entry is incremented. Both fds share
        the same offset and flags.

        Parameters
        ----------
        fd : int
            The file descriptor to duplicate.

        Returns
        -------
        int or None
            The new file descriptor, or None if fd is not valid.
        """
        entry = self._entries.get(fd)
        if entry is None:
            return None
        entry.ref_count += 1
        new_fd = self._next_fd
        self._entries[new_fd] = entry
        self._next_fd += 1
        return new_fd

    def dup2(self, old_fd: int, new_fd: int) -> int | None:
        """Duplicate a file descriptor to a specific fd number.

        If new_fd is already open, it is closed first. Then new_fd is made
        to point to the same OpenFile entry as old_fd.

        Parameters
        ----------
        old_fd : int
            The source file descriptor.
        new_fd : int
            The target file descriptor number.

        Returns
        -------
        int or None
            new_fd on success, None if old_fd is not valid.
        """
        old_entry = self._entries.get(old_fd)
        if old_entry is None:
            return None

        # If new_fd is already open, close it
        if new_fd in self._entries:
            self.close(new_fd)

        old_entry.ref_count += 1
        self._entries[new_fd] = old_entry
        return new_fd


class FileDescriptorTable:
    """Per-process mapping of local file descriptors to system-wide fds.

    Each process has its own FileDescriptorTable. This allows two processes
    to both have an "fd 3" that refers to completely different files. The
    local fd is mapped to a system-wide fd in the OpenFileTable.

    When a process forks, the child gets a *clone* of the parent's
    FileDescriptorTable. Both tables initially point to the same system-wide
    entries, but the tables themselves are independent --- closing an fd in
    the child does not affect the parent's table.
    """

    def __init__(self) -> None:
        self._mappings: dict[int, int] = {}  # local_fd -> global_fd

    def add(self, local_fd: int, global_fd: int) -> None:
        """Add a mapping from a local fd to a system-wide fd.

        Parameters
        ----------
        local_fd : int
            The process-local file descriptor number.
        global_fd : int
            The system-wide file descriptor in the OpenFileTable.
        """
        self._mappings[local_fd] = global_fd

    def remove(self, local_fd: int) -> int | None:
        """Remove a local fd mapping and return the global fd.

        Parameters
        ----------
        local_fd : int
            The process-local file descriptor to remove.

        Returns
        -------
        int or None
            The global fd that was mapped, or None if no mapping existed.
        """
        return self._mappings.pop(local_fd, None)

    def get_global(self, local_fd: int) -> int | None:
        """Look up the system-wide fd for a local fd.

        Parameters
        ----------
        local_fd : int
            The process-local file descriptor to look up.

        Returns
        -------
        int or None
            The corresponding system-wide fd, or None if no mapping exists.
        """
        return self._mappings.get(local_fd)

    def clone(self) -> "FileDescriptorTable":
        """Create a copy of this table (used during fork).

        The clone is an independent copy --- modifying one does not affect
        the other. However, both tables initially point to the same
        system-wide OpenFile entries, so the ref_counts on those entries
        should be incremented by the caller.

        Returns
        -------
        FileDescriptorTable
            A new table with the same mappings.
        """
        new_table = FileDescriptorTable()
        new_table._mappings = dict(self._mappings)
        return new_table
