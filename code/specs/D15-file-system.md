# D15 — File System

## Overview

A file system is the abstraction that turns a raw disk — billions of identical
bytes with no structure — into the familiar world of files and directories. Without
a file system, every program would need to remember "my data starts at byte
4,194,304 and is 8,192 bytes long." With a file system, you just say
`open("/home/alice/notes.txt")` and the OS figures out the rest.

This package implements a simplified inode-based file system inspired by ext2,
the classic Linux file system. We chose ext2 because it is the simplest real-world
file system that demonstrates all the core concepts:

- **Inodes**: fixed-size records that store file metadata (size, permissions,
  which blocks hold the data)
- **Directories**: special files whose contents are lists of (name, inode) pairs
- **Block allocation**: a bitmap tracking which disk blocks are free
- **Path resolution**: the algorithm for turning "/home/alice/notes.txt" into
  a sequence of inode lookups

**Analogy:** Think of a library. The *disk* is the building full of shelves. The
*file system* is the cataloging system — the card catalog (inode table), the
Dewey Decimal numbers (block pointers), the shelf labels (directories), and the
checkout desk (file descriptors). Without the cataloging system, you would have
a warehouse of unlabeled books with no way to find anything.

## Where It Fits

```
User Program
│   open("/data/log.txt", O_RDWR)
│   write(fd, "hello", 5)
│   close(fd)
▼
OS Kernel — Syscall Dispatcher
│   sys_open(56)  → VFS
│   sys_write(1)  → VFS (for fd >= 3)
│   sys_close(57) → VFS
▼
Virtual File System (VFS) ← YOU ARE HERE
│   ├── Path Resolution    — "/" → inode 0 → "data" → inode 5 → "log.txt" → inode 23
│   ├── Inode Table        — metadata for every file/directory
│   ├── Block Bitmap       — which disk blocks are free/used
│   ├── Open File Table    — system-wide table of open files
│   ├── FD Table           — per-process mapping: local fd → open file entry
│   └── Superblock         — file system metadata (sizes, counts, magic number)
▼
Block Device Interface
│   read_block(n) → [u8; 512]
│   write_block(n, data)
▼
Device Driver Framework (D14)
│   BlockDevice trait
▼
Hardware (simulated disk)
```

**Depends on:** Device Driver Framework (`BlockDevice` trait for raw block I/O)

**Used by:** OS Kernel (syscall handlers), IPC (D16, pipes are file descriptors),
Network Stack (D17, sockets are file descriptors)

## Key Concepts

### What Is Persistent Storage?

Everything we have built so far — registers, caches, DRAM — is **volatile**.
When you turn off the computer, all that data vanishes. Persistent storage
(hard drives, SSDs) keeps data across power cycles. But persistent storage is
*slow* (microseconds to milliseconds, vs nanoseconds for DRAM) and *block-
addressed* (you read/write in fixed-size chunks, not individual bytes).

The file system's job is to hide these ugly details behind a clean abstraction:
named files containing arbitrary-length byte sequences, organized into a
hierarchy of directories.

### Block Devices

A block device is any storage device that reads and writes fixed-size blocks.
Unlike character devices (keyboards, serial ports) that produce a stream of
individual bytes, block devices only operate in chunks:

```
Block Device Interface
══════════════════════

  read_block(block_number) → [u8; BLOCK_SIZE]
      "Give me the 512 bytes stored at block #47"

  write_block(block_number, data: [u8; BLOCK_SIZE])
      "Replace block #47 with these 512 bytes"

  block_size() → usize
      "How big is each block?" → 512

  block_count() → usize
      "How many blocks does this device have?" → 512
```

We use 512-byte blocks (the traditional hard disk sector size) and a total
disk size of 512 blocks = 256 KB. This is tiny by modern standards but large
enough to demonstrate every file system concept.

### Disk Layout

When we "format" a disk, we divide it into regions with specific purposes.
This is the on-disk layout of our file system:

```
Block:  0         1                    N        N+1                   511
        ┌─────────┬────────────────────┬────────┬─────────────────────┐
        │ Super-  │   Inode Table      │ Block  │    Data Blocks      │
        │ block   │   (blocks 1..N)    │ Bitmap │    (user data)      │
        │         │                    │        │                     │
        │ magic   │ inode 0 (root dir) │ 1 bit  │ file contents,      │
        │ counts  │ inode 1            │ per    │ directory entries,   │
        │ sizes   │ inode 2            │ block  │ indirect blocks     │
        │         │ ...                │        │                     │
        │         │ inode 127          │        │                     │
        └─────────┴────────────────────┴────────┴─────────────────────┘

Superblock:     1 block  (block 0)
Inode Table:    N blocks (blocks 1 through N, holding 128 inodes)
Block Bitmap:   1 block  (block N+1, tracks which data blocks are free)
Data Blocks:    remaining blocks (block N+2 through 511)
```

**Why this layout?** The superblock must be at a fixed, known location (block 0)
so the OS can always find it. The inode table comes next because it is the
index to everything else. The bitmap is compact (one bit per block). Data
blocks fill the rest of the disk.

### Inodes: The Heart of the File System

An **inode** (index node) is a fixed-size record that stores everything about
a file *except its name*. This is a crucial insight: in Unix-style file systems,
**names live in directories, not in files.** A file's inode stores:

```
Inode Structure
═══════════════

  ┌──────────────────┬────────────────────────────────────────────────┐
  │ Field            │ Description                                    │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ inode_number     │ Unique ID (0-127). Inode 0 is always the root │
  │                  │ directory "/".                                  │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ file_type        │ What kind of object this inode represents:     │
  │                  │   REGULAR    — ordinary file (text, binary)    │
  │                  │   DIRECTORY  — contains directory entries      │
  │                  │   SYMLINK    — symbolic link (stores a path)   │
  │                  │   CHAR_DEVICE — character device (e.g. tty)   │
  │                  │   BLOCK_DEVICE — block device (e.g. disk)     │
  │                  │   PIPE       — named pipe / FIFO               │
  │                  │   SOCKET     — Unix domain socket              │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ size             │ File size in bytes. For directories, this is   │
  │                  │ the total size of all directory entries.        │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ permissions      │ Octal permission bits (e.g. 0o755).            │
  │                  │   Owner: rwx (read/write/execute)              │
  │                  │   Group: r-x                                    │
  │                  │   Other: r-x                                    │
  │                  │ We store this as a u16.                         │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ owner_pid        │ PID of the process that created this file.     │
  │                  │ In a real OS this would be a user ID (UID),    │
  │                  │ but we use PID for simplicity.                  │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ link_count       │ Number of directory entries pointing to this   │
  │                  │ inode. When it reaches 0, the inode and its    │
  │                  │ data blocks can be freed. This is how hard     │
  │                  │ links work.                                     │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ direct_blocks    │ Array of 12 block numbers. direct_blocks[0]   │
  │ [12]             │ is the first block of file data, [1] is the   │
  │                  │ second, etc. With 512-byte blocks, this lets   │
  │                  │ a file be up to 12 × 512 = 6,144 bytes using  │
  │                  │ direct blocks alone.                            │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ indirect_block   │ Block number of an *indirect block*. The       │
  │                  │ indirect block itself contains more block      │
  │                  │ numbers (pointers). With 512-byte blocks and   │
  │                  │ 4-byte pointers, one indirect block holds 128  │
  │                  │ additional pointers → 128 × 512 = 65,536 more │
  │                  │ bytes. Total max file size: 6,144 + 65,536     │
  │                  │ = 71,680 bytes.                                 │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ created_at       │ Timestamp when the inode was created.          │
  │ modified_at      │ Timestamp when file data was last written.     │
  │ accessed_at      │ Timestamp when file data was last read.        │
  └──────────────────┴────────────────────────────────────────────────┘
```

**Why separate names from metadata?** Because one file can have multiple names
(hard links). If you create a hard link `ln original.txt alias.txt`, both
names point to the same inode. The file's data exists only once on disk. The
`link_count` tracks how many names reference this inode.

### Direct and Indirect Block Pointers

Small files (up to 6,144 bytes with 12 direct pointers) store their block
numbers directly in the inode. But what about larger files? We add one level
of **indirection**:

```
Inode
┌────────────────────────┐
│ direct_blocks[0]  ──────────→ Data Block (bytes 0-511)
│ direct_blocks[1]  ──────────→ Data Block (bytes 512-1023)
│ direct_blocks[2]  ──────────→ Data Block (bytes 1024-1535)
│ ...                    │
│ direct_blocks[11] ──────────→ Data Block (bytes 5632-6143)
│                        │
│ indirect_block ────────────→ ┌──────────────────────┐
│                        │    │ ptr[0] → Data Block   │ (bytes 6144-6655)
│                        │    │ ptr[1] → Data Block   │ (bytes 6656-7167)
│                        │    │ ptr[2] → Data Block   │ (bytes 7168-7679)
│                        │    │ ...                    │
│                        │    │ ptr[127] → Data Block  │ (bytes 71168-71679)
│                        │    └──────────────────────┘
└────────────────────────┘

The indirect block is itself a data block on disk, but instead of holding file
contents, it holds 128 block numbers (pointers to other data blocks).

Real file systems (ext4) use double-indirect and triple-indirect blocks for
even larger files. We keep it to one level for simplicity.
```

### Directories: Files That Contain Names

A directory is an inode with `file_type = DIRECTORY`. Its data blocks contain
a list of **directory entries**, each mapping a name to an inode number:

```
Directory Entry (DirectoryEntry)
════════════════════════════════

  ┌───────────────┬────────────────────────────────────────┐
  │ Field         │ Description                            │
  ├───────────────┼────────────────────────────────────────┤
  │ name          │ File/directory name, up to 255 chars.  │
  │               │ No slashes or null bytes allowed.       │
  ├───────────────┼────────────────────────────────────────┤
  │ inode_number  │ The inode this name refers to.          │
  └───────────────┴────────────────────────────────────────┘
```

Every directory contains at least two entries:
- `.`  (dot) — points to the directory's own inode
- `..` (dotdot) — points to the parent directory's inode

For the root directory (inode 0), both `.` and `..` point to inode 0 (the
root is its own parent).

Example: what does `/home/alice/` look like on disk?

```
Inode 0 (root "/")                  Inode 5 ("/home")
  type: DIRECTORY                     type: DIRECTORY
  data blocks contain:                data blocks contain:
    ┌──────┬───────┐                    ┌──────┬───────┐
    │ "."  │   0   │                    │ "."  │   5   │
    │ ".." │   0   │                    │ ".." │   0   │
    │"home"│   5   │──────────→         │"alice│  12   │───→ Inode 12
    │"etc" │   3   │                    │"bob" │  13   │     ("/home/alice")
    └──────┴───────┘                    └──────┴───────┘
```

### Superblock

The superblock is the first block on disk (block 0). It contains the metadata
needed to mount the file system — without it, the OS cannot interpret the rest
of the disk:

```
Superblock
══════════

  ┌──────────────────┬────────────────────────────────────────────────┐
  │ Field            │ Description                                    │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ magic            │ 0x45585432 — the ASCII bytes "EXT2".           │
  │                  │ Used to verify this disk actually contains     │
  │                  │ our file system (not random garbage).           │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ block_size       │ 512 bytes. Every block on disk is this size.   │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ total_blocks     │ 512. The total number of blocks on the disk.   │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ total_inodes     │ 128. Maximum number of files/directories the   │
  │                  │ file system can hold.                           │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ free_blocks      │ How many data blocks are currently unallocated.│
  ├──────────────────┼────────────────────────────────────────────────┤
  │ free_inodes      │ How many inodes are currently unallocated.     │
  └──────────────────┴────────────────────────────────────────────────┘
```

### Inode Table

The `InodeTable` manages the array of all 128 inodes. It provides three
operations:

- **allocate(file_type) → inode_number**: Find the first free inode, mark it
  as used, initialize it with the given type, and return its number.
- **free(inode_number)**: Mark an inode as available for reuse. Also releases
  all data blocks the inode was pointing to.
- **get(inode_number) → &Inode**: Return a reference to the inode for reading
  or modifying.

### Block Bitmap

The block bitmap tracks which data blocks are free and which are in use. It
uses one bit per block: 0 = free, 1 = used.

```
Block Bitmap (one bit per data block)
═════════════════════════════════════

  Bit index:   0   1   2   3   4   5   6   7   8   9  ...
  Value:       1   1   1   0   0   1   0   0   0   0  ...
               ▲   ▲   ▲           ▲
               │   │   │           │
             used used used      used         (rest are free)
```

Operations:
- **allocate() → block_number**: Find the first free bit (0), set it to 1,
  return the block number.
- **free(block_number)**: Set the bit back to 0.
- **is_free(block_number) → bool**: Check if a block is available.

## File Descriptors: The Process View of Open Files

When a process calls `open("/data/log.txt")`, it does not get back an inode
or a block number. It gets a small integer — a **file descriptor** (fd). File
descriptors are the process's handle to an open file, abstracting away all the
details of inodes and blocks.

There are two levels of indirection:

```
Process A                              System-Wide
┌─────────────────────┐               ┌──────────────────────────────┐
│ FileDescriptorTable │               │ OpenFileTable                │
│ (per-process)       │               │ (shared by all processes)    │
│                     │               │                              │
│ fd 0 ────────────────────────────→  │ entry 0: inode=7 (stdin)     │
│ fd 1 ────────────────────────────→  │ entry 1: inode=8 (stdout)    │
│ fd 2 ────────────────────────────→  │ entry 2: inode=9 (stderr)    │
│ fd 3 ────────────────────────────→  │ entry 5: inode=23, offset=42 │
│ fd 4 ────────────────────────────→  │ entry 7: inode=23, offset=0  │
└─────────────────────┘               └──────────────────────────────┘

Process B
┌─────────────────────┐
│ fd 0 ────────────────────────────→  (same stdin entry)
│ fd 1 ────────────────────────────→  (same stdout entry)
│ fd 2 ────────────────────────────→  (same stderr entry)
│ fd 3 ────────────────────────────→  entry 9: inode=40, offset=100
└─────────────────────┘
```

### OpenFile (System-Wide Entry)

Each entry in the system-wide `OpenFileTable` represents one *opening* of a
file. Multiple processes can share the same entry (after `fork()`), or the
same file can have multiple entries (opened independently by different
processes).

```
OpenFile
════════

  ┌──────────────────┬────────────────────────────────────────────────┐
  │ Field            │ Description                                    │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ inode_number     │ Which file this entry refers to.               │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ offset           │ Current read/write position within the file.   │
  │                  │ Each read/write advances this offset.           │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ flags            │ How the file was opened: READ, WRITE, or       │
  │                  │ READ_WRITE. Determines which operations are    │
  │                  │ permitted.                                      │
  ├──────────────────┼────────────────────────────────────────────────┤
  │ ref_count        │ How many file descriptors (across all          │
  │                  │ processes) point to this entry. When it drops  │
  │                  │ to 0, the entry is freed.                       │
  └──────────────────┴────────────────────────────────────────────────┘
```

### Standard File Descriptors

By convention, every process starts with three open file descriptors:

```
fd 0 = stdin   — standard input  (keyboard by default)
fd 1 = stdout  — standard output (display by default)
fd 2 = stderr  — standard error  (display by default)
fd 3+          — files opened by the process via sys_open
```

These conventions are universal across Unix, Linux, macOS, and even Windows
(which calls them "standard handles"). When you type `echo hello > file.txt`
in a shell, the shell opens `file.txt` and makes fd 1 point to it instead of
the display — this is **I/O redirection**, and it works because programs
write to fd 1 without caring what fd 1 actually points to.

### FileDescriptorTable (Per-Process)

Each process has its own `FileDescriptorTable` that maps local file descriptor
numbers (0, 1, 2, 3, ...) to indices in the system-wide `OpenFileTable`. This
is why two processes can both have an fd 3 that refers to completely different
files.

## Algorithms

### Path Resolution

Given a path like `/home/alice/notes.txt`, how does the file system find the
right inode? The algorithm is:

```
resolve_path("/home/alice/notes.txt")
═════════════════════════════════════

1. Start at the root inode (inode 0).

2. Split the path by "/" → ["home", "alice", "notes.txt"]
   (empty strings from leading "/" are ignored)

3. For each component:
   a. Verify the current inode is a DIRECTORY.
   b. Read the directory's data blocks.
   c. Search through DirectoryEntry records for a matching name.
   d. If found → the entry's inode_number becomes the current inode.
   e. If not found → return error: FILE_NOT_FOUND.

Step-by-step trace:
  ┌──────────────────────────────────────────────────────────┐
  │ Component     │ Current Inode │ Action                   │
  ├───────────────┼───────────────┼──────────────────────────┤
  │ (start)       │ 0 (root)      │ Begin at root            │
  │ "home"        │ 0 → 5         │ Found "home" → inode 5   │
  │ "alice"       │ 5 → 12        │ Found "alice" → inode 12 │
  │ "notes.txt"   │ 12 → 23       │ Found "notes.txt" → 23   │
  └───────────────┴───────────────┴──────────────────────────┘
  Result: inode 23 (the file)
```

### Reading a File

When a process calls `read(fd, buffer, count)`:

```
1. Look up fd in the process's FileDescriptorTable → get OpenFile index.
2. Look up the OpenFile entry → get inode_number and offset.
3. If fd is 0 (stdin): read from keyboard buffer (existing behavior).
4. If fd is 1 or 2: error (cannot read from stdout/stderr).
5. For fd >= 3:
   a. Get the inode from the InodeTable.
   b. Calculate which block contains the current offset:
      block_index = offset / block_size
      byte_within_block = offset % block_size
   c. If block_index < 12: use direct_blocks[block_index].
      If block_index >= 12: read the indirect block, then use
        indirect_pointers[block_index - 12].
   d. Read the block from the block device.
   e. Copy bytes from the block into the buffer.
   f. Advance the offset.
   g. Repeat until count bytes read or end-of-file reached.
6. Return the number of bytes actually read.
```

### Writing a File

When a process calls `write(fd, data, count)`:

```
1. Look up fd → OpenFile → inode.
2. If fd is 1 or 2 (stdout/stderr): write to display (existing behavior).
3. If fd is 0: error (cannot write to stdin).
4. For fd >= 3:
   a. Calculate block_index from offset (same as read).
   b. If the block is not yet allocated:
      - Allocate a new data block via BlockBitmap.
      - Store the block number in the inode's direct/indirect pointers.
   c. Read the existing block (for partial writes).
   d. Overwrite the relevant bytes.
   e. Write the block back to the device.
   f. Advance the offset.
   g. Update inode.size if offset > size.
   h. Repeat until all data written.
5. Return the number of bytes actually written.
```

### Formatting a Disk

The `format` operation initializes a blank disk with an empty file system:

```
1. Write the superblock to block 0:
   - magic = 0x45585432
   - block_size = 512
   - total_blocks = 512
   - total_inodes = 128
   - free_blocks = (total data blocks - 1)  (root dir uses one block)
   - free_inodes = 127  (inode 0 is used by root)

2. Initialize the inode table:
   - All inodes start as free (type = NONE).
   - Allocate inode 0 as the root directory.

3. Initialize the block bitmap:
   - All data blocks start as free (0), except one block for root dir's
     directory entries (containing "." and "..").

4. Write root directory data:
   - Create two DirectoryEntry records:
     (".", 0) and ("..", 0)
   - Write them to the root directory's first data block.
```

## VFS Operations

The Virtual File System layer provides these operations, which are called by
the kernel's syscall handlers:

| Operation | Description |
|-----------|-------------|
| `open(path, flags)` | Resolve path to inode, create OpenFile entry, allocate fd. Returns fd. |
| `close(fd)` | Decrement ref_count on OpenFile. If 0, remove entry. Free fd slot. |
| `read(fd, buf, count)` | Read up to `count` bytes from current offset. Advance offset. |
| `write(fd, buf, count)` | Write `count` bytes at current offset. Advance offset. May allocate blocks. |
| `lseek(fd, offset, whence)` | Reposition the file offset. whence: SET (absolute), CUR (relative), END (from end). |
| `stat(path)` | Resolve path to inode, return metadata (type, size, permissions, timestamps). |
| `mkdir(path)` | Create a new directory: allocate inode (DIRECTORY), add `.` and `..` entries, add entry in parent. |
| `readdir(fd)` | Read the next DirectoryEntry from an open directory. |
| `unlink(path)` | Remove a directory entry and decrement link_count. If link_count reaches 0, free inode and blocks. |
| `resolve_path(path)` | Walk the directory tree from root, return the final inode number. |
| `format(device)` | Initialize a blank disk with superblock, inode table, bitmap, root directory. |

## Syscalls

These new syscalls are added to the kernel's syscall dispatch table:

```
Syscall Table Additions
═══════════════════════

  ┌──────────────┬─────────┬──────────────────────────────────────────┐
  │ Name         │ Number  │ Arguments                                │
  ├──────────────┼─────────┼──────────────────────────────────────────┤
  │ sys_open     │ 56      │ (path_ptr, path_len, flags) → fd         │
  │ sys_close    │ 57      │ (fd) → 0 on success                      │
  │ sys_lseek    │ 62      │ (fd, offset, whence) → new_offset         │
  │ sys_stat     │ 80      │ (path_ptr, path_len, buf_ptr) → 0        │
  │ sys_mkdir    │ 83      │ (path_ptr, path_len, mode) → 0           │
  │ sys_getdents │ 78      │ (fd, buf_ptr, buf_len) → bytes_read      │
  │ sys_unlink   │ 87      │ (path_ptr, path_len) → 0                 │
  │ sys_dup      │ 32      │ (old_fd) → new_fd                        │
  │ sys_dup2     │ 33      │ (old_fd, new_fd) → new_fd                │
  └──────────────┴─────────┴──────────────────────────────────────────┘

Evolution of existing syscalls:
  - sys_read (0):  fd 0 → keyboard buffer (unchanged)
                   fd 1,2 → error
                   fd >= 3 → VFS read
  - sys_write (1): fd 1,2 → display driver (unchanged)
                   fd 0 → error
                   fd >= 3 → VFS write
```

### sys_dup and sys_dup2

These syscalls duplicate file descriptors, which is essential for I/O
redirection in shells:

```
sys_dup(old_fd) → new_fd
  Allocate the lowest available fd number.
  Point it to the same OpenFile entry as old_fd.
  Increment ref_count on that OpenFile entry.

sys_dup2(old_fd, new_fd) → new_fd
  If new_fd is already open, close it first.
  Point new_fd to the same OpenFile entry as old_fd.
  Increment ref_count.

Example: shell redirection "echo hello > output.txt"
  1. Shell calls sys_open("output.txt", WRITE) → returns fd 3
  2. Shell calls sys_dup2(3, 1) → fd 1 now points to output.txt
  3. Shell calls sys_close(3)  → fd 3 is freed
  4. Now when the child process writes to fd 1 (stdout), it goes to the file
```

## Dependencies

```
D15 File System
│
├── depends on ──→ Device Driver Framework (D14)
│                   └── BlockDevice trait (read_block, write_block)
│
├── used by ───→ IPC (D16)
│                 └── Pipes are represented as file descriptors
│
└── used by ───→ Network Stack (D17)
                  └── Sockets are represented as file descriptors
```

## Testing Strategy

### Unit Tests

1. **Superblock serialization**: Write superblock to block device, read it back,
   verify all fields (especially magic number).
2. **Inode allocation/free**: Allocate all 128 inodes, verify they get unique
   numbers 0-127. Free some, re-allocate, verify reuse.
3. **Block bitmap**: Allocate blocks, verify `is_free` returns false. Free them,
   verify `is_free` returns true. Allocate all blocks, verify allocation fails.
4. **Directory entries**: Create directory, add entries, read them back. Verify
   `.` and `..` are always present.
5. **Path resolution**: Create nested directories `/a/b/c`, verify
   `resolve_path` returns the correct inode at each level. Verify nonexistent
   paths return FILE_NOT_FOUND.
6. **Direct block read/write**: Create file, write < 6144 bytes, read back,
   verify contents match.
7. **Indirect block read/write**: Write > 6144 bytes, verify indirect block
   pointer is used, read back correctly.
8. **File descriptors**: Open multiple files, verify unique fd numbers. Close
   and reopen, verify fd reuse. Verify fd 0/1/2 routing.
9. **sys_dup/sys_dup2**: Duplicate fd, write to both, verify they share offset.
   dup2 over existing fd, verify old fd is closed.
10. **lseek**: Open file, write data, seek to beginning, read back. Test SET,
    CUR, END modes.
11. **unlink**: Create file, write data, unlink, verify inode is freed and
    blocks are reclaimed.
12. **format**: Format a fresh disk, verify superblock, root directory with
    `.` and `..`, all other inodes/blocks free.

### Integration Tests

13. **Full workflow**: Format → mkdir → create file → write → close → open →
    read → verify contents.
14. **Multiple processes**: Two processes open different files, verify
    independent fd tables and offsets.
15. **Stress test**: Create files until inodes exhausted, verify clean error.
    Fill disk until blocks exhausted, verify clean error.

### Coverage Target

Target 95%+ line coverage. Every VFS operation, every error path (disk full,
inode table full, file not found, permission denied, bad fd), and every
block pointer type (direct, indirect) must be exercised.
