/**
 * # File System — An Inode-Based File System (ext2-Inspired)
 *
 * ## What Is a File System?
 *
 * A file system is the abstraction that turns a raw disk — billions of identical
 * bytes with no structure — into the familiar world of files and directories.
 * Without a file system, every program would need to remember "my data starts at
 * byte 4,194,304 and is 8,192 bytes long." With a file system, you just say
 * `open("/home/alice/notes.txt")` and the OS figures out the rest.
 *
 * **Analogy:** Think of a library. The *disk* is the building full of shelves.
 * The *file system* is the cataloging system — the card catalog (inode table),
 * the Dewey Decimal numbers (block pointers), the shelf labels (directories),
 * and the checkout desk (file descriptors). Without the cataloging system, you
 * would have a warehouse of unlabeled books with no way to find anything.
 *
 * ## Architecture
 *
 * ```
 * VFS (Virtual File System)
 * ├── Path Resolution    — "/" → inode 0 → "data" → inode 5 → "log.txt" → inode 23
 * ├── Inode Table        — metadata for every file/directory
 * ├── Block Bitmap       — which disk blocks are free/used
 * ├── Open File Table    — system-wide table of open files
 * ├── FD Table           — per-process mapping: local fd → open file entry
 * └── Superblock         — file system metadata (sizes, counts, magic number)
 * ```
 */

// ============================================================================
// Constants
// ============================================================================

/**
 * ## Disk Geometry Constants
 *
 * These constants define the physical layout of our simulated disk. We use
 * 512-byte blocks (the traditional hard disk sector size) and a total disk
 * size of 512 blocks = 256 KB. This is tiny by modern standards but large
 * enough to demonstrate every file system concept.
 *
 * DIRECT_BLOCKS is 12, matching ext2. A file can address up to 12 blocks
 * (6,144 bytes) without needing an indirect block.
 *
 * MAX_INODES is 128 — the maximum number of files and directories that can
 * exist simultaneously.
 *
 * ROOT_INODE is always 0 — the root directory "/" lives at inode 0, and this
 * is a fixed convention that the OS relies on to bootstrap path resolution.
 */
export const BLOCK_SIZE = 512;
export const MAX_BLOCKS = 512;
export const MAX_INODES = 128;
export const DIRECT_BLOCKS = 12;
export const ROOT_INODE = 0;

/**
 * The unallocated sentinel. When a direct_blocks slot or indirect_block field
 * contains -1, it means "no block has been allocated here yet." This is
 * analogous to a null pointer — it tells the file system that attempting to
 * read from this slot would be reading uninitialized data.
 */
const UNALLOCATED = -1;

/**
 * The superblock magic number: 0x45585432, which is the ASCII encoding of
 * "EXT2". When mounting a disk, the OS reads block 0 and checks this magic
 * number to confirm the disk actually contains our file system format (and
 * not random garbage or a different file system).
 */
const MAGIC = 0x45585432;

// ============================================================================
// File Types
// ============================================================================

/**
 * ## FileType — What Kind of Object Does an Inode Represent?
 *
 * In Unix-style file systems, "everything is a file" — but not all files are
 * created equal. The FileType enum distinguishes between ordinary files,
 * directories, symbolic links, device nodes, pipes, and sockets. Each type
 * has different semantics for read/write/open operations.
 *
 * ```
 * FileType Values
 * ═══════════════
 *   REGULAR (1)      — ordinary file containing user data (text, binary, etc.)
 *   DIRECTORY (2)    — contains directory entries (name → inode mappings)
 *   SYMLINK (3)      — symbolic link (stores a path string as its data)
 *   CHAR_DEVICE (4)  — character device (e.g., terminal, serial port)
 *   BLOCK_DEVICE (5) — block device (e.g., disk drive)
 *   PIPE (6)         — named pipe / FIFO for inter-process communication
 *   SOCKET (7)       — Unix domain socket
 * ```
 */
export enum FileType {
  REGULAR = 1,
  DIRECTORY = 2,
  SYMLINK = 3,
  CHAR_DEVICE = 4,
  BLOCK_DEVICE = 5,
  PIPE = 6,
  SOCKET = 7,
}

// ============================================================================
// Open Flags and Seek Modes
// ============================================================================

/**
 * ## Open Flags — How Should the File Be Opened?
 *
 * When a process calls `open()`, it passes flags that control how the file
 * can be accessed. These are bitmask values that can be combined with bitwise
 * OR. The values match the Linux ABI for educational accuracy.
 *
 * ```
 * Flag Truth Table
 * ════════════════
 *   Flags         │ Read? │ Write? │ Create if missing? │ Truncate? │ Append?
 *   ──────────────┼───────┼────────┼────────────────────┼───────────┼────────
 *   O_RDONLY      │  yes  │   no   │        no          │    no     │   no
 *   O_WRONLY      │  no   │  yes   │        no          │    no     │   no
 *   O_RDWR        │  yes  │  yes   │        no          │    no     │   no
 *   O_WRONLY|CREAT│  no   │  yes   │       yes          │    no     │   no
 *   O_WRONLY|TRUNC│  no   │  yes   │        no          │   yes     │   no
 *   O_WRONLY|APPND│  no   │  yes   │        no          │    no     │  yes
 * ```
 */
export const O_RDONLY = 0;
export const O_WRONLY = 1;
export const O_RDWR = 2;
export const O_CREAT = 64;
export const O_TRUNC = 512;
export const O_APPEND = 1024;

/**
 * ## Seek Modes — Where Should the File Offset Move?
 *
 * The `lseek()` system call repositions the read/write offset within an open
 * file. The `whence` parameter determines how the `offset` argument is
 * interpreted:
 *
 * ```
 * Seek Modes
 * ══════════
 *   SEEK_SET (0) — offset is absolute (from start of file)
 *                  new_offset = offset
 *   SEEK_CUR (1) — offset is relative to current position
 *                  new_offset = current_offset + offset
 *   SEEK_END (2) — offset is relative to end of file
 *                  new_offset = file_size + offset
 *
 * Example with a 100-byte file, currently at offset 50:
 *   lseek(fd, 10, SEEK_SET) → offset becomes 10
 *   lseek(fd, 10, SEEK_CUR) → offset becomes 60 (50 + 10)
 *   lseek(fd, -5, SEEK_END) → offset becomes 95 (100 + (-5))
 * ```
 */
export const SEEK_SET = 0;
export const SEEK_CUR = 1;
export const SEEK_END = 2;

// ============================================================================
// Superblock
// ============================================================================

/**
 * ## Superblock — The File System's Identity Card
 *
 * The superblock is the very first block on disk (block 0). It contains the
 * metadata needed to mount the file system — without it, the OS cannot
 * interpret the rest of the disk. Think of it as the title page of a book:
 * it tells you the book's name (magic), how many pages it has (total_blocks),
 * and how many chapters are indexed (total_inodes).
 *
 * ```
 * Superblock Layout (block 0)
 * ═══════════════════════════
 *   ┌──────────────┬──────────────────────────────────────────┐
 *   │ magic        │ 0x45585432 ("EXT2") — validates format   │
 *   │ block_size   │ 512 bytes per block                      │
 *   │ total_blocks │ 512 blocks total on disk                 │
 *   │ total_inodes │ 128 inodes (max files/directories)       │
 *   │ free_blocks  │ currently unallocated data blocks         │
 *   │ free_inodes  │ currently unallocated inodes              │
 *   └──────────────┴──────────────────────────────────────────┘
 * ```
 */
export interface Superblock {
  magic: number;
  block_size: number;
  total_blocks: number;
  total_inodes: number;
  free_blocks: number;
  free_inodes: number;
}

// ============================================================================
// Inode
// ============================================================================

/**
 * ## Inode — The Heart of the File System
 *
 * An inode (index node) is a fixed-size record that stores everything about
 * a file *except its name*. This is a crucial insight: in Unix-style file
 * systems, **names live in directories, not in files.** A file can have
 * multiple names (hard links) that all point to the same inode.
 *
 * ```
 * Inode Structure
 * ═══════════════
 *   inode_number   — unique ID (0–127), inode 0 = root directory "/"
 *   file_type      — REGULAR, DIRECTORY, SYMLINK, etc.
 *   size           — file size in bytes
 *   permissions    — octal permission bits (e.g., 0o755)
 *   owner_pid      — PID of the creating process
 *   link_count     — number of directory entries pointing here
 *   direct_blocks  — 12 block numbers for file data (first 6,144 bytes)
 *   indirect_block — block number of an indirect pointer block
 *   created_at     — creation timestamp
 *   modified_at    — last data modification timestamp
 *   accessed_at    — last data access timestamp
 *
 * Block Addressing
 * ════════════════
 *   direct_blocks[0..11] → up to 12 × 512 = 6,144 bytes
 *   indirect_block → holds 128 pointers → 128 × 512 = 65,536 more bytes
 *   Maximum file size: 6,144 + 65,536 = 71,680 bytes
 * ```
 */
export interface Inode {
  inode_number: number;
  file_type: FileType;
  size: number;
  permissions: number;
  owner_pid: number;
  link_count: number;
  direct_blocks: number[];
  indirect_block: number;
  created_at: number;
  modified_at: number;
  accessed_at: number;
}

/**
 * Creates a fresh inode with all block pointers set to UNALLOCATED (-1).
 * The timestamps are set to the current time, permissions default to 0o755
 * (owner can read/write/execute, others can read/execute), and the link
 * count starts at 1 (the directory entry that created this inode).
 */
function createInode(inodeNumber: number, fileType: FileType): Inode {
  const now = Date.now();
  return {
    inode_number: inodeNumber,
    file_type: fileType,
    size: 0,
    permissions: 0o755,
    owner_pid: 0,
    link_count: 1,
    direct_blocks: new Array(DIRECT_BLOCKS).fill(UNALLOCATED),
    indirect_block: UNALLOCATED,
    created_at: now,
    modified_at: now,
    accessed_at: now,
  };
}

// ============================================================================
// Directory Entry
// ============================================================================

/**
 * ## DirectoryEntry — Mapping Names to Inodes
 *
 * A directory is just a file whose data blocks contain a list of these
 * entries. Each entry maps a human-readable name to an inode number.
 *
 * ```
 * Example: root directory "/"
 * ═══════════════════════════
 *   ┌──────────┬─────────────┐
 *   │ name     │ inode_number │
 *   ├──────────┼─────────────┤
 *   │ "."      │ 0           │  ← points to self
 *   │ ".."     │ 0           │  ← points to parent (root is its own parent)
 *   │ "home"   │ 5           │  ← subdirectory
 *   │ "etc"    │ 3           │  ← another subdirectory
 *   └──────────┴─────────────┘
 * ```
 *
 * We serialize entries as "name\0inodeNumber\n" for simplicity. The null
 * byte separates the name from the inode number, and the newline separates
 * entries from each other. This is simpler than ext2's variable-length
 * record format but demonstrates the same concept.
 */
export interface DirectoryEntry {
  name: string;
  inode_number: number;
}

/**
 * Serializes an array of directory entries into a byte buffer. Each entry
 * is encoded as "name\0inodeNumber\n". For example, the entries
 * [{ name: ".", inode_number: 0 }, { name: "..", inode_number: 0 }]
 * become the string ".\00\n..\00\n" encoded as UTF-8 bytes.
 */
function serializeDirectoryEntries(entries: DirectoryEntry[]): Uint8Array {
  const text = entries.map((e) => `${e.name}\0${e.inode_number}\n`).join("");
  return new TextEncoder().encode(text);
}

/**
 * Deserializes a byte buffer back into an array of directory entries.
 * This is the inverse of serializeDirectoryEntries. It splits the decoded
 * text by newlines, then splits each line by the null byte to recover
 * the name and inode number.
 */
function deserializeDirectoryEntries(data: Uint8Array): DirectoryEntry[] {
  const text = new TextDecoder().decode(data);
  const entries: DirectoryEntry[] = [];
  const lines = text.split("\n");
  for (const line of lines) {
    if (line.length === 0) continue;
    const nullIdx = line.indexOf("\0");
    if (nullIdx === -1) continue;
    const name = line.substring(0, nullIdx);
    const inodeNum = parseInt(line.substring(nullIdx + 1), 10);
    if (!isNaN(inodeNum)) {
      entries.push({ name, inode_number: inodeNum });
    }
  }
  return entries;
}

// ============================================================================
// Block Bitmap
// ============================================================================

/**
 * ## BlockBitmap — Tracking Free and Used Blocks
 *
 * The block bitmap uses one bit per data block to track which blocks are
 * free (0) and which are in use (1). This is a simple and efficient data
 * structure for allocation — finding a free block is just scanning for the
 * first zero bit.
 *
 * ```
 * Block Bitmap (one bit per data block)
 * ═════════════════════════════════════
 *   Bit index:   0   1   2   3   4   5   6   7   8   9  ...
 *   Value:       1   1   1   0   0   1   0   0   0   0  ...
 *                ▲   ▲   ▲           ▲
 *              used used used      used         (rest are free)
 *
 * Operations:
 *   allocate() → scan for first 0 bit, set to 1, return block number
 *   free(n)    → set bit n back to 0
 *   is_free(n) → return whether bit n is 0
 * ```
 *
 * **Why a bitmap?** Because it is compact (512 blocks need only 64 bytes)
 * and allocation is O(n) in the worst case — acceptable for our small disk.
 * Real file systems use more sophisticated structures (block groups in ext4,
 * B-trees in XFS) for performance at scale.
 */
export class BlockBitmap {
  /** The bitmap array: true = used, false = free */
  private bitmap: boolean[];
  /** Total number of data blocks tracked by this bitmap */
  private totalBlocks: number;

  constructor(totalBlocks: number) {
    this.totalBlocks = totalBlocks;
    this.bitmap = new Array(totalBlocks).fill(false);
  }

  /**
   * Allocates the first free block. Scans the bitmap from index 0 upward,
   * looking for the first `false` (free) entry. Sets it to `true` (used)
   * and returns the block index. Returns null if the disk is full.
   */
  allocate(): number | null {
    for (let i = 0; i < this.totalBlocks; i++) {
      if (!this.bitmap[i]) {
        this.bitmap[i] = true;
        return i;
      }
    }
    return null;
  }

  /**
   * Frees a previously allocated block, making it available for reuse.
   * This is called when a file is deleted or truncated.
   */
  free(blockNum: number): void {
    if (blockNum >= 0 && blockNum < this.totalBlocks) {
      this.bitmap[blockNum] = false;
    }
  }

  /**
   * Checks whether a block is free (available for allocation).
   */
  isFree(blockNum: number): boolean {
    if (blockNum < 0 || blockNum >= this.totalBlocks) return false;
    return !this.bitmap[blockNum];
  }

  /**
   * Returns the count of free (unallocated) blocks.
   */
  freeCount(): number {
    return this.bitmap.filter((used) => !used).length;
  }
}

// ============================================================================
// Inode Table
// ============================================================================

/**
 * ## InodeTable — The Index of All Files
 *
 * The inode table is an array of all inodes on the file system. It provides
 * three operations: allocate (find a free inode and initialize it), free
 * (release an inode for reuse), and get (look up an inode by number).
 *
 * ```
 * Inode Table (128 slots)
 * ═══════════════════════
 *   Index:  0        1        2        3      ...    127
 *           ┌────────┬────────┬────────┬────────┬────────┐
 *           │ root / │ free   │ file   │ dir    │  ...   │
 *           │ DIR    │        │ REG    │ DIR    │        │
 *           └────────┴────────┴────────┴────────┴────────┘
 *
 *   Inode 0 is always the root directory.
 *   Free inodes have file_type = undefined (null in our table).
 * ```
 */
export class InodeTable {
  /** The array of inodes. null = free slot. */
  private inodes: (Inode | null)[];
  /** Total number of inode slots */
  private maxInodes: number;

  constructor(maxInodes: number) {
    this.maxInodes = maxInodes;
    this.inodes = new Array(maxInodes).fill(null);
  }

  /**
   * Allocates the first free inode, initializes it with the given file type,
   * and returns the new Inode. Returns null if all inodes are in use.
   */
  allocate(fileType: FileType): Inode | null {
    for (let i = 0; i < this.maxInodes; i++) {
      if (this.inodes[i] === null) {
        const inode = createInode(i, fileType);
        this.inodes[i] = inode;
        return inode;
      }
    }
    return null;
  }

  /**
   * Frees an inode, making its slot available for reuse.
   */
  free(inodeNumber: number): void {
    if (inodeNumber >= 0 && inodeNumber < this.maxInodes) {
      this.inodes[inodeNumber] = null;
    }
  }

  /**
   * Returns the inode at the given index, or null if the slot is free.
   */
  get(inodeNumber: number): Inode | null {
    if (inodeNumber < 0 || inodeNumber >= this.maxInodes) return null;
    return this.inodes[inodeNumber];
  }
}

// ============================================================================
// Open File Table (System-Wide)
// ============================================================================

/**
 * ## OpenFile — A System-Wide Entry for an Open File
 *
 * When a process calls `open()`, the kernel creates an entry in the
 * system-wide open file table. This entry tracks:
 *
 * - **inode_number**: which file is open
 * - **offset**: current read/write position (each read/write advances it)
 * - **flags**: how the file was opened (read, write, or both)
 * - **ref_count**: how many file descriptors point to this entry
 *
 * Multiple processes can share the same OpenFile entry (after `fork()`),
 * which means they share the same offset — writing in one process advances
 * the offset for the other. This is how pipes and shared file access work.
 */
export interface OpenFile {
  inode_number: number;
  offset: number;
  flags: number;
  ref_count: number;
}

/**
 * ## OpenFileTable — The System-Wide Table of Open Files
 *
 * This table is shared by all processes. Each slot holds an OpenFile entry
 * or null (free). File descriptors in per-process FD tables point into
 * this table by index.
 *
 * ```
 * OpenFileTable (system-wide)
 * ══════════════════════════
 *   Index:  0       1       2       3       4       5      ...
 *           ┌───────┬───────┬───────┬───────┬───────┬───────┐
 *           │stdin  │stdout │stderr │file A │ free  │file B │
 *           │ref=2  │ref=2  │ref=2  │ref=1  │       │ref=1  │
 *           └───────┴───────┴───────┴───────┴───────┴───────┘
 *
 *   Entries 0-2 are pre-allocated for stdin/stdout/stderr.
 *   New opens start searching from index 3.
 * ```
 */
export class OpenFileTable {
  private entries: (OpenFile | null)[];

  constructor() {
    this.entries = [];
  }

  /**
   * Opens a file: creates a new OpenFile entry in the first available slot
   * (starting from index 3, since 0/1/2 are reserved for stdin/stdout/stderr).
   * Returns the table index (which becomes the global file descriptor).
   * Returns null if no slots are available (we cap at 256 entries).
   */
  open(inodeNumber: number, flags: number): number | null {
    /* Find first free slot starting at index 3 */
    for (let i = 3; i < this.entries.length; i++) {
      if (this.entries[i] === null) {
        this.entries[i] = {
          inode_number: inodeNumber,
          offset: 0,
          flags: flags & 3, // mask to access mode bits (O_RDONLY/O_WRONLY/O_RDWR)
          ref_count: 1,
        };
        return i;
      }
    }
    /* No free slot found — extend the table if under the cap */
    if (this.entries.length < 256) {
      const idx = this.entries.length < 3 ? 3 : this.entries.length;
      /* Fill any gap between current length and idx */
      while (this.entries.length < idx) {
        this.entries.push(null);
      }
      this.entries.push({
        inode_number: inodeNumber,
        offset: 0,
        flags: flags & 3,
        ref_count: 1,
      });
      return this.entries.length - 1;
    }
    return null;
  }

  /**
   * Closes an open file entry. Decrements ref_count; if it reaches 0,
   * the slot is freed for reuse.
   */
  close(index: number): void {
    const entry = this.entries[index];
    if (entry) {
      entry.ref_count--;
      if (entry.ref_count <= 0) {
        this.entries[index] = null;
      }
    }
  }

  /**
   * Returns the OpenFile entry at the given index, or null if empty.
   */
  get(index: number): OpenFile | null {
    if (index < 0 || index >= this.entries.length) return null;
    return this.entries[index];
  }

  /**
   * Duplicates an open file entry by incrementing its ref_count.
   * Returns the same index (the caller is responsible for mapping it
   * to a new local fd in the per-process FD table).
   */
  dup(index: number): number | null {
    const entry = this.entries[index];
    if (!entry) return null;
    entry.ref_count++;
    return index;
  }
}

// ============================================================================
// File Descriptor Table (Per-Process)
// ============================================================================

/**
 * ## FileDescriptorTable — Per-Process FD Mapping
 *
 * Each process has its own FileDescriptorTable that maps local file
 * descriptor numbers (0, 1, 2, 3, ...) to indices in the system-wide
 * OpenFileTable. This is why two processes can both have an fd 3 that
 * refers to completely different files.
 *
 * ```
 * Process A's FD Table          System-Wide OpenFileTable
 * ═════════════════════         ═════════════════════════
 *   fd 0 ──────────────────→   entry 0 (stdin)
 *   fd 1 ──────────────────→   entry 1 (stdout)
 *   fd 2 ──────────────────→   entry 2 (stderr)
 *   fd 3 ──────────────────→   entry 5 (some file)
 *   fd 4 ──────────────────→   entry 7 (another file)
 * ```
 *
 * **Standard file descriptors:** By Unix convention, fd 0 = stdin,
 * fd 1 = stdout, fd 2 = stderr. User-opened files start at fd 3.
 */
export class FileDescriptorTable {
  /**
   * Maps local fd number → global OpenFileTable index.
   * null means the fd slot is free.
   */
  private fds: (number | null)[];

  constructor() {
    /* Reserve slots 0, 1, 2 for stdin, stdout, stderr.
     * We do not actually point them anywhere — VFS handles these specially. */
    this.fds = [0, 1, 2];
  }

  /**
   * Allocates the lowest available fd number and maps it to the given
   * global index in the OpenFileTable. Returns the local fd number.
   */
  allocate(globalIndex: number): number {
    /* Search for the lowest free slot starting at fd 3 */
    for (let i = 3; i < this.fds.length; i++) {
      if (this.fds[i] === null) {
        this.fds[i] = globalIndex;
        return i;
      }
    }
    /* No free slot — extend the table */
    this.fds.push(globalIndex);
    return this.fds.length - 1;
  }

  /**
   * Returns the global OpenFileTable index for the given local fd,
   * or null if the fd is not open.
   */
  get(fd: number): number | null {
    if (fd < 0 || fd >= this.fds.length) return null;
    return this.fds[fd] ?? null;
  }

  /**
   * Frees a local fd slot, making it available for reuse.
   */
  free(fd: number): void {
    if (fd >= 0 && fd < this.fds.length) {
      this.fds[fd] = null;
    }
  }

  /**
   * Duplicates a file descriptor: maps `newFd` to the same global index
   * as `oldFd`. If `newFd` is already in use, it is freed first.
   * Returns the new fd number, or null if oldFd is invalid.
   */
  dup2(oldFd: number, newFd: number): number | null {
    const globalIdx = this.get(oldFd);
    if (globalIdx === null) return null;
    /* Extend table if needed */
    while (this.fds.length <= newFd) {
      this.fds.push(null);
    }
    this.fds[newFd] = globalIdx;
    return newFd;
  }

  /**
   * Creates a deep copy of this FD table for `fork()`. The child process
   * gets its own copy of the fd-to-global mapping, but the global entries
   * themselves are shared (ref_count must be incremented by the caller).
   */
  clone(): FileDescriptorTable {
    const copy = new FileDescriptorTable();
    copy.fds = [...this.fds];
    return copy;
  }
}

// ============================================================================
// VFS — Virtual File System
// ============================================================================

/**
 * ## VFS — The Virtual File System
 *
 * The VFS is the central orchestrator of the file system. It ties together
 * the superblock, inode table, block bitmap, open file table, and per-process
 * fd tables into a coherent whole. All file operations — open, close, read,
 * write, mkdir, unlink — go through the VFS.
 *
 * ```
 * Disk Layout (after format)
 * ══════════════════════════
 *   Block 0:      Superblock (magic, counts, sizes)
 *   Blocks 1..N:  Inode table (128 inodes)
 *   Block N+1:    Block bitmap (1 bit per data block)
 *   Blocks N+2+:  Data blocks (user data, directory entries)
 * ```
 *
 * In our implementation, the "disk" is an in-memory array of Uint8Array
 * blocks. This avoids needing actual hardware — we simulate a block device
 * entirely in RAM. The algorithms are identical to what a real file system
 * would do on a physical disk.
 */
export class VFS {
  /** In-memory block storage — our simulated disk */
  private blocks: Uint8Array[];
  /** The superblock metadata */
  private superblock!: Superblock;
  /** Tracks which data blocks are free/used */
  private blockBitmap!: BlockBitmap;
  /** The table of all inodes */
  private inodeTable!: InodeTable;
  /** System-wide table of open files */
  private openFileTable: OpenFileTable;
  /** Per-process file descriptor tables, keyed by PID */
  private fdTables: Map<number, FileDescriptorTable>;

  /**
   * The index of the first data block. Blocks before this are reserved
   * for the superblock, inode table, and block bitmap.
   *
   * Layout:
   *   Block 0 = superblock
   *   Blocks 1..inodeTableBlocks = inode table
   *   Block inodeTableBlocks+1 = block bitmap
   *   Block inodeTableBlocks+2.. = data blocks
   */
  private dataBlockStart!: number;

  constructor() {
    this.blocks = [];
    this.openFileTable = new OpenFileTable();
    this.fdTables = new Map();
  }

  /**
   * ## format() — Initialize a Blank Disk
   *
   * Formatting creates an empty file system on the simulated disk:
   *
   * 1. Allocate MAX_BLOCKS blocks of BLOCK_SIZE bytes each.
   * 2. Write the superblock to block 0.
   * 3. Initialize the inode table (all inodes free except inode 0).
   * 4. Initialize the block bitmap (all data blocks free except one for root).
   * 5. Create the root directory at inode 0 with "." and ".." entries.
   *
   * After formatting, the disk is ready for use — you can create files and
   * directories starting from the root "/".
   */
  format(): void {
    /* Step 1: Allocate the simulated disk — an array of empty blocks */
    this.blocks = [];
    for (let i = 0; i < MAX_BLOCKS; i++) {
      this.blocks.push(new Uint8Array(BLOCK_SIZE));
    }

    /*
     * Calculate disk layout. We need to know how many blocks the inode table
     * occupies. Each inode is a logical record; in our in-memory implementation,
     * the inode table is a separate data structure, but we still reserve disk
     * space for it to maintain a realistic layout.
     *
     * We estimate 64 bytes per inode (conservative), so:
     *   128 inodes × 64 bytes = 8,192 bytes / 512 bytes per block = 16 blocks
     */
    const inodeTableBlocks = Math.ceil((MAX_INODES * 64) / BLOCK_SIZE);
    this.dataBlockStart = 1 + inodeTableBlocks + 1; // superblock + inode table + bitmap

    /* Step 2: Initialize the inode table */
    this.inodeTable = new InodeTable(MAX_INODES);

    /* Calculate how many data blocks are available */
    const totalDataBlocks = MAX_BLOCKS - this.dataBlockStart;

    /* Step 3: Initialize the block bitmap */
    this.blockBitmap = new BlockBitmap(totalDataBlocks);

    /* Step 4: Create the root directory (inode 0) */
    const rootInode = this.inodeTable.allocate(FileType.DIRECTORY)!;
    rootInode.link_count = 2; // "." and ".." both point to root

    /* Allocate a data block for the root directory's entries */
    const rootBlockIdx = this.blockBitmap.allocate()!;
    rootInode.direct_blocks[0] = rootBlockIdx;

    /* Write the "." and ".." entries to the root directory's data block */
    const rootEntries: DirectoryEntry[] = [
      { name: ".", inode_number: ROOT_INODE },
      { name: "..", inode_number: ROOT_INODE },
    ];
    const rootData = serializeDirectoryEntries(rootEntries);
    const blockAbsolute = this.dataBlockStart + rootBlockIdx;
    this.blocks[blockAbsolute].set(rootData);
    rootInode.size = rootData.length;

    /* Step 5: Write the superblock */
    this.superblock = {
      magic: MAGIC,
      block_size: BLOCK_SIZE,
      total_blocks: MAX_BLOCKS,
      total_inodes: MAX_INODES,
      free_blocks: this.blockBitmap.freeCount(),
      free_inodes: MAX_INODES - 1, // inode 0 is used by root
    };

    /* Reset the open file table and fd tables */
    this.openFileTable = new OpenFileTable();
    this.fdTables = new Map();
  }

  /**
   * Returns a copy of the superblock for inspection (e.g., by stat or tests).
   */
  getSuperblock(): Superblock {
    return { ...this.superblock };
  }

  // --------------------------------------------------------------------------
  // Path Resolution
  // --------------------------------------------------------------------------

  /**
   * ## resolve_path() — Turn a Path String into an Inode Number
   *
   * Given a path like "/home/alice/notes.txt", this method walks the
   * directory tree from the root inode, looking up each component in the
   * appropriate directory's entries.
   *
   * ```
   * Algorithm:
   * ══════════
   *   1. Start at inode 0 (root directory).
   *   2. Split path by "/" → ["home", "alice", "notes.txt"]
   *   3. For each component:
   *      a. Verify current inode is a DIRECTORY.
   *      b. Read directory entries from data blocks.
   *      c. Find the entry matching the component name.
   *      d. Follow the entry's inode_number.
   *   4. Return the final inode number.
   *
   * Example trace for "/home/alice":
   *   ┌──────────┬───────────────┬──────────────────────────┐
   *   │Component │ Current Inode │ Action                   │
   *   ├──────────┼───────────────┼──────────────────────────┤
   *   │ (start)  │ 0 (root)      │ Begin at root            │
   *   │ "home"   │ 0 → 5         │ Found "home" → inode 5   │
   *   │ "alice"  │ 5 → 12        │ Found "alice" → inode 12 │
   *   └──────────┴───────────────┴──────────────────────────┘
   * ```
   */
  resolvePath(path: string): number | null {
    /* Handle root path */
    if (path === "/") return ROOT_INODE;

    /* Split the path into components, filtering out empty strings */
    const components = path.split("/").filter((c) => c.length > 0);
    let currentInode = ROOT_INODE;

    for (const component of components) {
      const inode = this.inodeTable.get(currentInode);
      if (!inode || inode.file_type !== FileType.DIRECTORY) return null;

      /* Read this directory's entries */
      const entries = this.readDirectoryEntries(inode);
      const entry = entries.find((e) => e.name === component);
      if (!entry) return null;

      currentInode = entry.inode_number;
    }

    return currentInode;
  }

  // --------------------------------------------------------------------------
  // Directory Operations
  // --------------------------------------------------------------------------

  /**
   * ## mkdir() — Create a New Directory
   *
   * Creates a new directory at the given path. The parent directory must
   * already exist. The new directory is initialized with "." (pointing to
   * itself) and ".." (pointing to its parent).
   *
   * ```
   * mkdir("/home/alice")
   * ════════════════════
   *   1. Resolve parent path "/home" → inode 5
   *   2. Allocate new inode (type=DIRECTORY) → inode 12
   *   3. Allocate data block for new directory
   *   4. Write "." (→12) and ".." (→5) entries to new block
   *   5. Add "alice" (→12) entry to parent directory (inode 5)
   *   6. Increment parent's link_count (because ".." points to it)
   * ```
   */
  mkdir(path: string): boolean {
    const { parentPath, name } = this.splitPath(path);
    if (!name) return false;

    /* Resolve the parent directory */
    const parentInodeNum = this.resolvePath(parentPath);
    if (parentInodeNum === null) return false;

    const parentInode = this.inodeTable.get(parentInodeNum);
    if (!parentInode || parentInode.file_type !== FileType.DIRECTORY)
      return false;

    /* Check the name doesn't already exist in the parent */
    const parentEntries = this.readDirectoryEntries(parentInode);
    if (parentEntries.find((e) => e.name === name)) return false;

    /* Allocate a new inode for the directory */
    const newInode = this.inodeTable.allocate(FileType.DIRECTORY);
    if (!newInode) return false;

    /* Allocate a data block for the directory's entries */
    const blockIdx = this.blockBitmap.allocate();
    if (blockIdx === null) {
      this.inodeTable.free(newInode.inode_number);
      return false;
    }

    /* Initialize the new directory with "." and ".." entries */
    newInode.direct_blocks[0] = blockIdx;
    newInode.link_count = 2; // "." and ".." within this directory

    const entries: DirectoryEntry[] = [
      { name: ".", inode_number: newInode.inode_number },
      { name: "..", inode_number: parentInodeNum },
    ];
    const data = serializeDirectoryEntries(entries);
    const blockAbsolute = this.dataBlockStart + blockIdx;
    this.blocks[blockAbsolute].set(data);
    newInode.size = data.length;

    /* Add entry in the parent directory */
    this.addDirectoryEntry(parentInode, name, newInode.inode_number);

    /* Increment parent's link count (the new ".." points to parent) */
    parentInode.link_count++;

    /* Update superblock */
    this.superblock.free_inodes--;
    this.superblock.free_blocks = this.blockBitmap.freeCount();

    return true;
  }

  /**
   * ## readdir() — List Directory Contents
   *
   * Returns an array of DirectoryEntry records for the given path.
   * This is the equivalent of `ls` — it shows you what names exist
   * in a directory and which inodes they point to.
   */
  readdir(path: string): DirectoryEntry[] | null {
    const inodeNum = this.resolvePath(path);
    if (inodeNum === null) return null;

    const inode = this.inodeTable.get(inodeNum);
    if (!inode || inode.file_type !== FileType.DIRECTORY) return null;

    return this.readDirectoryEntries(inode);
  }

  // --------------------------------------------------------------------------
  // File Operations: open, close, read, write, lseek
  // --------------------------------------------------------------------------

  /**
   * ## open() — Open a File for Reading/Writing
   *
   * This is the gateway to all file I/O. The process specifies a path and
   * flags (read, write, create, etc.), and gets back a file descriptor — a
   * small integer it can use for subsequent read/write/close calls.
   *
   * ```
   * open("/data/log.txt", O_RDWR | O_CREAT)
   * ════════════════════════════════════════
   *   1. Resolve path "/data/log.txt" → inode 23 (or create if O_CREAT)
   *   2. Create OpenFile entry: { inode: 23, offset: 0, flags: RDWR, ref: 1 }
   *   3. Allocate fd in process's FD table → fd 3
   *   4. Return fd 3 to the caller
   * ```
   */
  open(path: string, flags: number, pid: number = 0): number | null {
    let inodeNum = this.resolvePath(path);

    /* If the file doesn't exist and O_CREAT is set, create it */
    if (inodeNum === null && (flags & O_CREAT)) {
      inodeNum = this.createFile(path);
      if (inodeNum === null) return null;
    }

    if (inodeNum === null) return null;

    const inode = this.inodeTable.get(inodeNum);
    if (!inode) return null;

    /* O_TRUNC: truncate the file to zero length */
    if (flags & O_TRUNC) {
      this.truncateFile(inode);
    }

    /* Create a system-wide open file entry */
    const globalIdx = this.openFileTable.open(inodeNum, flags);
    if (globalIdx === null) return null;

    /* If O_APPEND is set, position offset at end of file */
    if (flags & O_APPEND) {
      const entry = this.openFileTable.get(globalIdx)!;
      entry.offset = inode.size;
    }

    /* Allocate a local fd in the process's FD table */
    const fdTable = this.getFdTable(pid);
    const fd = fdTable.allocate(globalIdx);

    return fd;
  }

  /**
   * ## close() — Close a File Descriptor
   *
   * Releases the file descriptor and decrements the ref_count on the
   * underlying OpenFile entry. If no more references exist, the OpenFile
   * entry is freed.
   */
  close(fd: number, pid: number = 0): boolean {
    const fdTable = this.getFdTable(pid);
    const globalIdx = fdTable.get(fd);
    if (globalIdx === null) return false;

    fdTable.free(fd);
    this.openFileTable.close(globalIdx);
    return true;
  }

  /**
   * ## read() — Read Data from an Open File
   *
   * Reads up to `count` bytes from the file starting at the current offset.
   * The offset advances by the number of bytes read.
   *
   * ```
   * Reading Algorithm
   * ═════════════════
   *   1. Look up fd → OpenFile → inode
   *   2. Calculate which block holds the current offset:
   *        block_index = offset / BLOCK_SIZE
   *        byte_within_block = offset % BLOCK_SIZE
   *   3. If block_index < 12 → use direct_blocks[block_index]
   *      If block_index >= 12 → read indirect block, use pointer[block_index-12]
   *   4. Read bytes from block, advance offset
   *   5. Repeat until count bytes read or end-of-file
   * ```
   */
  read(fd: number, count: number, pid: number = 0): Uint8Array | null {
    const fdTable = this.getFdTable(pid);
    const globalIdx = fdTable.get(fd);
    if (globalIdx === null) return null;

    const openFile = this.openFileTable.get(globalIdx);
    if (!openFile) return null;

    /* Check that the file was opened for reading */
    if (openFile.flags === O_WRONLY) return null;

    const inode = this.inodeTable.get(openFile.inode_number);
    if (!inode) return null;

    /* Calculate how many bytes we can actually read */
    const bytesAvailable = inode.size - openFile.offset;
    if (bytesAvailable <= 0) return new Uint8Array(0);

    const bytesToRead = Math.min(count, bytesAvailable);
    const result = new Uint8Array(bytesToRead);
    let bytesRead = 0;

    while (bytesRead < bytesToRead) {
      const blockIndex = Math.floor(
        (openFile.offset + bytesRead) / BLOCK_SIZE,
      );
      const byteInBlock = (openFile.offset + bytesRead) % BLOCK_SIZE;

      /* Resolve the block number (direct or indirect) */
      const blockNum = this.resolveBlockNumber(inode, blockIndex);
      if (blockNum === null) break;

      /* Read from the block */
      const blockAbsolute = this.dataBlockStart + blockNum;
      const block = this.blocks[blockAbsolute];
      const chunkSize = Math.min(
        BLOCK_SIZE - byteInBlock,
        bytesToRead - bytesRead,
      );
      result.set(block.subarray(byteInBlock, byteInBlock + chunkSize), bytesRead);
      bytesRead += chunkSize;
    }

    openFile.offset += bytesRead;
    inode.accessed_at = Date.now();

    return result.subarray(0, bytesRead);
  }

  /**
   * ## write() — Write Data to an Open File
   *
   * Writes `data` to the file starting at the current offset. New blocks
   * are allocated as needed via the BlockBitmap.
   *
   * ```
   * Writing Algorithm
   * ═════════════════
   *   1. Look up fd → OpenFile → inode
   *   2. For each chunk of data:
   *      a. Calculate block_index from offset
   *      b. If block not yet allocated → allocate via BlockBitmap
   *      c. Read existing block (for partial writes)
   *      d. Overwrite relevant bytes
   *      e. Write block back to disk
   *      f. Advance offset
   *   3. Update inode.size if we wrote past the end
   * ```
   */
  write(fd: number, data: Uint8Array, pid: number = 0): number | null {
    const fdTable = this.getFdTable(pid);
    const globalIdx = fdTable.get(fd);
    if (globalIdx === null) return null;

    const openFile = this.openFileTable.get(globalIdx);
    if (!openFile) return null;

    /* Check that the file was opened for writing */
    if (openFile.flags === O_RDONLY) return null;

    const inode = this.inodeTable.get(openFile.inode_number);
    if (!inode) return null;

    let bytesWritten = 0;

    while (bytesWritten < data.length) {
      const blockIndex = Math.floor(
        (openFile.offset + bytesWritten) / BLOCK_SIZE,
      );
      const byteInBlock = (openFile.offset + bytesWritten) % BLOCK_SIZE;

      /* Ensure a block is allocated at this index */
      let blockNum = this.resolveBlockNumber(inode, blockIndex);
      if (blockNum === null) {
        /* Allocate a new block */
        blockNum = this.allocateBlockForInode(inode, blockIndex);
        if (blockNum === null) break; // disk full
      }

      /* Write to the block */
      const blockAbsolute = this.dataBlockStart + blockNum;
      const block = this.blocks[blockAbsolute];
      const chunkSize = Math.min(
        BLOCK_SIZE - byteInBlock,
        data.length - bytesWritten,
      );
      block.set(
        data.subarray(bytesWritten, bytesWritten + chunkSize),
        byteInBlock,
      );
      bytesWritten += chunkSize;
    }

    openFile.offset += bytesWritten;

    /* Update file size if we wrote past the end */
    if (openFile.offset > inode.size) {
      inode.size = openFile.offset;
    }

    inode.modified_at = Date.now();
    this.superblock.free_blocks = this.blockBitmap.freeCount();

    return bytesWritten;
  }

  /**
   * ## lseek() — Reposition the File Offset
   *
   * Moves the read/write position within an open file. The `whence`
   * parameter determines how `offset` is interpreted:
   *
   * ```
   *   SEEK_SET (0): new_offset = offset            (absolute)
   *   SEEK_CUR (1): new_offset = current + offset  (relative)
   *   SEEK_END (2): new_offset = file_size + offset (from end)
   *
   * Example with a 100-byte file at offset 50:
   *   lseek(fd, 10, SEEK_SET) → 10
   *   lseek(fd, 10, SEEK_CUR) → 60
   *   lseek(fd, -5, SEEK_END) → 95
   * ```
   */
  lseek(
    fd: number,
    offset: number,
    whence: number,
    pid: number = 0,
  ): number | null {
    const fdTable = this.getFdTable(pid);
    const globalIdx = fdTable.get(fd);
    if (globalIdx === null) return null;

    const openFile = this.openFileTable.get(globalIdx);
    if (!openFile) return null;

    const inode = this.inodeTable.get(openFile.inode_number);
    if (!inode) return null;

    let newOffset: number;
    switch (whence) {
      case SEEK_SET:
        newOffset = offset;
        break;
      case SEEK_CUR:
        newOffset = openFile.offset + offset;
        break;
      case SEEK_END:
        newOffset = inode.size + offset;
        break;
      default:
        return null;
    }

    if (newOffset < 0) return null;

    openFile.offset = newOffset;
    return newOffset;
  }

  // --------------------------------------------------------------------------
  // stat and unlink
  // --------------------------------------------------------------------------

  /**
   * ## stat() — Get File Metadata
   *
   * Returns the inode metadata for the file at the given path. This is
   * how programs learn a file's size, type, permissions, and timestamps
   * without opening it.
   */
  stat(
    path: string,
  ): {
    inode_number: number;
    file_type: FileType;
    size: number;
    permissions: number;
    link_count: number;
    created_at: number;
    modified_at: number;
    accessed_at: number;
  } | null {
    const inodeNum = this.resolvePath(path);
    if (inodeNum === null) return null;

    const inode = this.inodeTable.get(inodeNum);
    if (!inode) return null;

    return {
      inode_number: inode.inode_number,
      file_type: inode.file_type,
      size: inode.size,
      permissions: inode.permissions,
      link_count: inode.link_count,
      created_at: inode.created_at,
      modified_at: inode.modified_at,
      accessed_at: inode.accessed_at,
    };
  }

  /**
   * ## unlink() — Remove a File
   *
   * Removes a directory entry and decrements the inode's link_count.
   * When link_count reaches 0, the inode and all its data blocks are freed.
   *
   * ```
   * unlink("/data/log.txt")
   * ═══════════════════════
   *   1. Resolve parent "/data" → inode 5
   *   2. Find "log.txt" in parent's entries → inode 23
   *   3. Remove "log.txt" entry from parent
   *   4. Decrement inode 23's link_count
   *   5. If link_count == 0:
   *      a. Free all data blocks (direct + indirect)
   *      b. Free the inode
   * ```
   *
   * **Important:** unlink does NOT remove directories. Use rmdir for that.
   * This matches Unix semantics — `unlink()` only works on non-directory
   * files (to prevent accidentally orphaning entire directory trees).
   */
  unlink(path: string): boolean {
    const { parentPath, name } = this.splitPath(path);
    if (!name) return false;

    const parentInodeNum = this.resolvePath(parentPath);
    if (parentInodeNum === null) return false;

    const parentInode = this.inodeTable.get(parentInodeNum);
    if (!parentInode || parentInode.file_type !== FileType.DIRECTORY)
      return false;

    /* Find the entry in the parent directory */
    const entries = this.readDirectoryEntries(parentInode);
    const entryIdx = entries.findIndex((e) => e.name === name);
    if (entryIdx === -1) return false;

    const targetInodeNum = entries[entryIdx].inode_number;
    const targetInode = this.inodeTable.get(targetInodeNum);
    if (!targetInode) return false;

    /* Don't unlink directories */
    if (targetInode.file_type === FileType.DIRECTORY) return false;

    /* Remove the entry from the parent */
    entries.splice(entryIdx, 1);
    this.writeDirectoryEntries(parentInode, entries);

    /* Decrement link count */
    targetInode.link_count--;

    /* If no more links, free the inode and its blocks */
    if (targetInode.link_count <= 0) {
      this.freeInodeBlocks(targetInode);
      this.inodeTable.free(targetInodeNum);
      this.superblock.free_inodes++;
      this.superblock.free_blocks = this.blockBitmap.freeCount();
    }

    return true;
  }

  // --------------------------------------------------------------------------
  // dup / dup2
  // --------------------------------------------------------------------------

  /**
   * ## dup() — Duplicate a File Descriptor
   *
   * Allocates the lowest available fd and points it to the same OpenFile
   * entry as the original. Both fds now share the same offset — advancing
   * one advances the other.
   */
  dup(fd: number, pid: number = 0): number | null {
    const fdTable = this.getFdTable(pid);
    const globalIdx = fdTable.get(fd);
    if (globalIdx === null) return null;

    /* Increment ref count on the open file entry */
    if (this.openFileTable.dup(globalIdx) === null) return null;

    /* Allocate a new local fd pointing to the same global entry */
    return fdTable.allocate(globalIdx);
  }

  /**
   * ## dup2() — Duplicate a File Descriptor to a Specific Number
   *
   * Like dup(), but the caller chooses the new fd number. If newFd is
   * already open, it is closed first. This is the mechanism behind shell
   * I/O redirection: `dup2(file_fd, 1)` redirects stdout to a file.
   */
  dup2(oldFd: number, newFd: number, pid: number = 0): number | null {
    const fdTable = this.getFdTable(pid);
    const globalIdx = fdTable.get(oldFd);
    if (globalIdx === null) return null;

    /* If newFd is already open, close it first */
    const existingGlobal = fdTable.get(newFd);
    if (existingGlobal !== null) {
      fdTable.free(newFd);
      this.openFileTable.close(existingGlobal);
    }

    /* Point newFd to the same OpenFile entry */
    if (this.openFileTable.dup(globalIdx) === null) return null;
    fdTable.dup2(oldFd, newFd);

    return newFd;
  }

  // --------------------------------------------------------------------------
  // Private Helper Methods
  // --------------------------------------------------------------------------

  /**
   * Gets or creates the per-process FD table for the given PID.
   */
  private getFdTable(pid: number): FileDescriptorTable {
    let table = this.fdTables.get(pid);
    if (!table) {
      table = new FileDescriptorTable();
      this.fdTables.set(pid, table);
    }
    return table;
  }

  /**
   * Splits a path into parent path and final component name.
   *
   * Examples:
   *   "/home/alice" → { parentPath: "/home", name: "alice" }
   *   "/file.txt"   → { parentPath: "/", name: "file.txt" }
   *   "/"           → { parentPath: "/", name: "" }
   */
  private splitPath(path: string): { parentPath: string; name: string } {
    const parts = path.split("/").filter((p) => p.length > 0);
    if (parts.length === 0) return { parentPath: "/", name: "" };
    const name = parts.pop()!;
    const parentPath = "/" + parts.join("/");
    return { parentPath: parentPath || "/", name };
  }

  /**
   * Reads all directory entries from an inode's data blocks. Concatenates
   * data from all allocated blocks and deserializes the entries.
   */
  private readDirectoryEntries(inode: Inode): DirectoryEntry[] {
    const data = this.readAllBlocks(inode);
    return deserializeDirectoryEntries(data.subarray(0, inode.size));
  }

  /**
   * Writes directory entries back to an inode's data blocks. If the
   * serialized data exceeds the currently allocated blocks, new blocks
   * are allocated. This handles directory growth.
   */
  private writeDirectoryEntries(
    inode: Inode,
    entries: DirectoryEntry[],
  ): void {
    const data = serializeDirectoryEntries(entries);
    /* Write the data across the inode's blocks */
    let written = 0;
    let blockIndex = 0;
    while (written < data.length) {
      let blockNum = this.resolveBlockNumber(inode, blockIndex);
      if (blockNum === null) {
        blockNum = this.allocateBlockForInode(inode, blockIndex);
        if (blockNum === null) break;
      }
      const blockAbsolute = this.dataBlockStart + blockNum;
      const chunkSize = Math.min(BLOCK_SIZE, data.length - written);
      /* Clear block first */
      this.blocks[blockAbsolute].fill(0);
      this.blocks[blockAbsolute].set(
        data.subarray(written, written + chunkSize),
      );
      written += chunkSize;
      blockIndex++;
    }
    inode.size = data.length;
  }

  /**
   * Adds a new directory entry to a directory inode. Reads the existing
   * entries, appends the new one, and writes them all back.
   */
  private addDirectoryEntry(
    dirInode: Inode,
    name: string,
    inodeNumber: number,
  ): void {
    const entries = this.readDirectoryEntries(dirInode);
    entries.push({ name, inode_number: inodeNumber });
    this.writeDirectoryEntries(dirInode, entries);
  }

  /**
   * Reads all data from an inode's blocks, concatenated into a single
   * Uint8Array. Used for directory entry deserialization and file reads.
   */
  private readAllBlocks(inode: Inode): Uint8Array {
    const totalBlocks = Math.ceil(inode.size / BLOCK_SIZE);
    const result = new Uint8Array(totalBlocks * BLOCK_SIZE);
    for (let i = 0; i < totalBlocks; i++) {
      const blockNum = this.resolveBlockNumber(inode, i);
      if (blockNum === null) break;
      const blockAbsolute = this.dataBlockStart + blockNum;
      result.set(this.blocks[blockAbsolute], i * BLOCK_SIZE);
    }
    return result;
  }

  /**
   * ## resolveBlockNumber() — Direct vs. Indirect Block Lookup
   *
   * Given a logical block index within a file, returns the physical block
   * number on disk. The first 12 blocks use direct pointers stored in the
   * inode. Blocks 12+ use the indirect block — a data block that itself
   * contains an array of block numbers.
   *
   * ```
   * Block Index Resolution
   * ══════════════════════
   *   index < 12  → inode.direct_blocks[index]
   *   index >= 12 → read inode.indirect_block → pointers[index - 12]
   * ```
   */
  private resolveBlockNumber(inode: Inode, blockIndex: number): number | null {
    if (blockIndex < DIRECT_BLOCKS) {
      const bn = inode.direct_blocks[blockIndex];
      return bn === UNALLOCATED ? null : bn;
    }

    /* Indirect block lookup */
    if (inode.indirect_block === UNALLOCATED) return null;

    const indirectAbsolute = this.dataBlockStart + inode.indirect_block;
    const indirectData = this.blocks[indirectAbsolute];
    const pointerIndex = blockIndex - DIRECT_BLOCKS;

    /* Each pointer is stored as 4 bytes (little-endian) */
    const offset = pointerIndex * 4;
    if (offset + 4 > BLOCK_SIZE) return null;

    const blockNum =
      indirectData[offset] |
      (indirectData[offset + 1] << 8) |
      (indirectData[offset + 2] << 16) |
      (indirectData[offset + 3] << 24);

    /* A value of 0 in an uninitialized indirect block means no block */
    return blockNum === 0 && pointerIndex > 0 ? null : blockNum === 0 ? null : blockNum;
  }

  /**
   * Allocates a new data block and assigns it to the given logical block
   * index within an inode. Handles both direct and indirect allocation.
   *
   * For indirect blocks: if the indirect block itself hasn't been allocated
   * yet, it is allocated first, then the pointer within it is set.
   */
  private allocateBlockForInode(
    inode: Inode,
    blockIndex: number,
  ): number | null {
    const newBlock = this.blockBitmap.allocate();
    if (newBlock === null) return null;

    if (blockIndex < DIRECT_BLOCKS) {
      inode.direct_blocks[blockIndex] = newBlock;
      return newBlock;
    }

    /* Need indirect block */
    if (inode.indirect_block === UNALLOCATED) {
      /* Allocate the indirect block itself */
      const indirectBlock = this.blockBitmap.allocate();
      if (indirectBlock === null) {
        this.blockBitmap.free(newBlock);
        return null;
      }
      inode.indirect_block = indirectBlock;
      /* Clear the indirect block */
      const blockAbsolute = this.dataBlockStart + indirectBlock;
      this.blocks[blockAbsolute].fill(0);
    }

    /* Write the pointer into the indirect block */
    const indirectAbsolute = this.dataBlockStart + inode.indirect_block;
    const pointerIndex = blockIndex - DIRECT_BLOCKS;
    const offset = pointerIndex * 4;

    const indirectData = this.blocks[indirectAbsolute];
    indirectData[offset] = newBlock & 0xff;
    indirectData[offset + 1] = (newBlock >> 8) & 0xff;
    indirectData[offset + 2] = (newBlock >> 16) & 0xff;
    indirectData[offset + 3] = (newBlock >> 24) & 0xff;

    return newBlock;
  }

  /**
   * Creates a new regular file at the given path. The parent directory
   * must exist. Returns the new inode number, or null on failure.
   */
  private createFile(path: string): number | null {
    const { parentPath, name } = this.splitPath(path);
    if (!name) return null;

    const parentInodeNum = this.resolvePath(parentPath);
    if (parentInodeNum === null) return null;

    const parentInode = this.inodeTable.get(parentInodeNum);
    if (!parentInode || parentInode.file_type !== FileType.DIRECTORY)
      return null;

    /* Allocate a new inode */
    const newInode = this.inodeTable.allocate(FileType.REGULAR);
    if (!newInode) return null;

    /* Add entry in parent directory */
    this.addDirectoryEntry(parentInode, name, newInode.inode_number);

    this.superblock.free_inodes--;

    return newInode.inode_number;
  }

  /**
   * Truncates a file to zero length by freeing all its data blocks.
   */
  private truncateFile(inode: Inode): void {
    this.freeInodeBlocks(inode);
    inode.size = 0;
    this.superblock.free_blocks = this.blockBitmap.freeCount();
  }

  /**
   * Frees all data blocks owned by an inode (both direct and indirect).
   * After this call, the inode has no allocated blocks.
   */
  private freeInodeBlocks(inode: Inode): void {
    /* Free direct blocks */
    for (let i = 0; i < DIRECT_BLOCKS; i++) {
      if (inode.direct_blocks[i] !== UNALLOCATED) {
        this.blockBitmap.free(inode.direct_blocks[i]);
        inode.direct_blocks[i] = UNALLOCATED;
      }
    }

    /* Free indirect block and its pointers */
    if (inode.indirect_block !== UNALLOCATED) {
      const indirectAbsolute = this.dataBlockStart + inode.indirect_block;
      const indirectData = this.blocks[indirectAbsolute];

      /* Each pointer is 4 bytes. BLOCK_SIZE / 4 = 128 pointers max. */
      for (let i = 0; i < BLOCK_SIZE / 4; i++) {
        const offset = i * 4;
        const blockNum =
          indirectData[offset] |
          (indirectData[offset + 1] << 8) |
          (indirectData[offset + 2] << 16) |
          (indirectData[offset + 3] << 24);
        if (blockNum !== 0) {
          this.blockBitmap.free(blockNum);
        }
      }

      /* Free the indirect block itself */
      this.blockBitmap.free(inode.indirect_block);
      inode.indirect_block = UNALLOCATED;
    }
  }
}
