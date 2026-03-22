//! # File System (D15)
//!
//! A simplified inode-based file system inspired by ext2, the classic Linux
//! file system. This crate implements the Virtual File System (VFS) layer
//! that sits between user programs and the raw disk, providing the familiar
//! abstraction of files and directories.
//!
//! ## What Is a File System?
//!
//! A file system turns a raw disk — billions of identical bytes with no
//! structure — into the familiar world of files and directories. Without a
//! file system, every program would need to remember "my data starts at byte
//! 4,194,304 and is 8,192 bytes long." With a file system, you just say
//! `open("/home/alice/notes.txt")` and the OS figures out the rest.
//!
//! ## Analogy
//!
//! Think of a library:
//! - The *disk* is the building full of shelves
//! - The *file system* is the cataloging system
//! - The *inode table* is the card catalog
//! - The *block pointers* are the Dewey Decimal numbers
//! - The *directories* are the shelf labels
//! - The *file descriptors* are the checkout desk
//!
//! ## Components
//!
//! - [`Superblock`]: File system metadata (magic number, free counts)
//! - [`Inode`]: File metadata (type, size, permissions, block pointers)
//! - [`DirectoryEntry`]: Name-to-inode mapping
//! - [`BlockBitmap`]: Tracks which blocks are free/used
//! - [`InodeTable`]: Manages the fixed array of inodes
//! - [`OpenFile`] / [`OpenFileTable`]: System-wide open file entries
//! - [`FileDescriptorTable`]: Per-process fd mapping
//! - [`VFS`]: The main interface tying everything together

use std::collections::HashMap;

// ============================================================================
// Constants
// ============================================================================

/// Size of each block in bytes. Every read and write operates on exactly one
/// block. This mirrors the traditional 512-byte disk sector.
pub const BLOCK_SIZE: usize = 512;

/// Total number of blocks on the disk. 512 blocks * 512 bytes = 256 KB.
pub const MAX_BLOCKS: usize = 512;

/// Maximum number of inodes (files + directories) the file system supports.
pub const MAX_INODES: usize = 128;

/// Number of direct block pointers in each inode. Files up to
/// DIRECT_BLOCKS * BLOCK_SIZE = 6,144 bytes need only direct pointers.
pub const DIRECT_BLOCKS: usize = 12;

/// The inode number for the root directory "/". Always 0 by convention.
pub const ROOT_INODE: usize = 0;

/// Maximum length of a file or directory name in bytes.
pub const MAX_NAME_LENGTH: usize = 255;

/// Magic number for the superblock. The ASCII bytes "EXT2" = 0x45585432.
/// Used to verify this disk actually contains our file system.
pub const MAGIC: u32 = 0x45585432;

// === File Types ===
//
// Every inode has a file_type field that tells the kernel what kind of
// object this inode represents. The kernel treats each type differently:
//
//   REGULAR files: data blocks contain file contents
//   DIRECTORIES: data blocks contain DirectoryEntry records
//   SYMLINKS: data blocks contain the target path string

/// No type assigned — the inode is free.
pub const FILE_TYPE_NONE: u8 = 0;
/// A regular file (text, binary, image, executable).
pub const FILE_TYPE_REGULAR: u8 = 1;
/// A directory (data blocks contain name-inode pairs).
pub const FILE_TYPE_DIRECTORY: u8 = 2;
/// A symbolic link (data blocks contain a path string).
pub const FILE_TYPE_SYMLINK: u8 = 3;
/// A character device (e.g., /dev/tty).
pub const FILE_TYPE_CHAR_DEVICE: u8 = 4;
/// A block device (e.g., /dev/sda).
pub const FILE_TYPE_BLOCK_DEVICE: u8 = 5;
/// A named pipe (FIFO).
pub const FILE_TYPE_PIPE: u8 = 6;
/// A Unix domain socket.
pub const FILE_TYPE_SOCKET: u8 = 7;

// === Open Flags ===
//
// When a process calls open(), it passes flags to specify the access mode
// and behavior modifiers. These values match Linux kernel definitions.
//
// Truth table for access mode check:
//   flags & 0x3  | Can read? | Can write?
//   -------------|-----------|----------
//   0 (O_RDONLY) |    yes    |    no
//   1 (O_WRONLY) |    no     |    yes
//   2 (O_RDWR)   |    yes    |    yes

/// Open for reading only.
pub const O_RDONLY: u32 = 0;
/// Open for writing only.
pub const O_WRONLY: u32 = 1;
/// Open for both reading and writing.
pub const O_RDWR: u32 = 2;
/// Create the file if it does not exist.
pub const O_CREAT: u32 = 64;
/// Truncate the file to zero length when opening.
pub const O_TRUNC: u32 = 512;
/// Append: set offset to end before each write.
pub const O_APPEND: u32 = 1024;

// === Seek Whence Constants ===
//
// These control how lseek() interprets the offset argument:
//   SEEK_SET: absolute position
//   SEEK_CUR: relative to current position
//   SEEK_END: relative to end of file

/// Set position to exactly the given offset.
pub const SEEK_SET: u32 = 0;
/// Move position forward (or backward) from current.
pub const SEEK_CUR: u32 = 1;
/// Set position relative to end of file.
pub const SEEK_END: u32 = 2;

// ============================================================================
// Superblock
// ============================================================================

/// The superblock is the first block on disk (block 0). It is the "table of
/// contents" for the entire file system — without it, the OS has no idea how
/// the disk is organized.
///
/// ## Analogy
///
/// Think of the superblock as the cover page of a book's index. It tells you
/// what kind of book this is (magic number), how many chapters there are
/// (total_blocks, total_inodes), and how many blank pages are left
/// (free_blocks, free_inodes).
#[derive(Debug, Clone)]
pub struct Superblock {
    /// Magic number to verify this is our file system (0x45585432 = "EXT2").
    pub magic: u32,
    /// Size of each block in bytes.
    pub block_size: usize,
    /// Total number of blocks on the disk.
    pub total_blocks: usize,
    /// Total number of inodes.
    pub total_inodes: usize,
    /// Number of free data blocks.
    pub free_blocks: usize,
    /// Number of free inodes.
    pub free_inodes: usize,
    /// Inode number of the root directory (always 0).
    pub root_inode: usize,
}

impl Superblock {
    /// Creates a new superblock with default values.
    pub fn new(total_blocks: usize, total_inodes: usize) -> Self {
        Self {
            magic: MAGIC,
            block_size: BLOCK_SIZE,
            total_blocks,
            total_inodes,
            free_blocks: 0,
            free_inodes: total_inodes,
            root_inode: ROOT_INODE,
        }
    }

    /// Validates that this superblock has the correct magic number.
    pub fn is_valid(&self) -> bool {
        self.magic == MAGIC
    }
}

// ============================================================================
// Inode
// ============================================================================

/// An inode (index node) stores everything about a file *except its name*.
/// Names live in directories, not in files. This separation is what makes
/// hard links possible — one file can have multiple names.
///
/// ## Block Pointers
///
/// ```text
/// Inode
/// +---------------------------+
/// | direct_blocks[0]  --------> Data Block (bytes 0..511)
/// | direct_blocks[1]  --------> Data Block (bytes 512..1023)
/// | ...                       |
/// | direct_blocks[11] --------> Data Block (bytes 5632..6143)
/// |                           |
/// | indirect_block    --------> [ptr0, ptr1, ..., ptr127]
/// |                           |     |     |          |
/// +---------------------------+     v     v          v
///                               Data   Data       Data
/// ```
///
/// With 12 direct pointers and one indirect block (128 pointers),
/// max file size = 12*512 + 128*512 = 71,680 bytes.
#[derive(Debug, Clone)]
pub struct Inode {
    /// Unique identifier (0 to MAX_INODES - 1). Inode 0 = root directory.
    pub inode_number: usize,
    /// What kind of object: FILE_TYPE_REGULAR, FILE_TYPE_DIRECTORY, etc.
    pub file_type: u8,
    /// File size in bytes.
    pub size: usize,
    /// Permission bits in octal (e.g., 0o755 = rwxr-xr-x).
    pub permissions: u16,
    /// PID of the creating process (simplified from real UID).
    pub owner_pid: u32,
    /// Number of directory entries pointing to this inode.
    pub link_count: u32,
    /// 12 direct block pointers. None means the slot is unused.
    pub direct_blocks: [Option<usize>; DIRECT_BLOCKS],
    /// Indirect block pointer. The indirect block holds 128 more pointers.
    pub indirect_block: Option<usize>,
    /// Creation timestamp (simplified as counter).
    pub created_at: u64,
    /// Last modification timestamp.
    pub modified_at: u64,
    /// Last access timestamp.
    pub accessed_at: u64,
}

impl Inode {
    /// Creates a new inode with the given number and type.
    pub fn new(inode_number: usize, file_type: u8) -> Self {
        Self {
            inode_number,
            file_type,
            size: 0,
            permissions: 0o755,
            owner_pid: 0,
            link_count: 0,
            direct_blocks: [None; DIRECT_BLOCKS],
            indirect_block: None,
            created_at: 0,
            modified_at: 0,
            accessed_at: 0,
        }
    }

    /// Returns true if this inode is free (no type assigned).
    pub fn is_free(&self) -> bool {
        self.file_type == FILE_TYPE_NONE
    }

    /// Returns true if this inode is a directory.
    pub fn is_directory(&self) -> bool {
        self.file_type == FILE_TYPE_DIRECTORY
    }

    /// Returns true if this inode is a regular file.
    pub fn is_regular(&self) -> bool {
        self.file_type == FILE_TYPE_REGULAR
    }
}

// ============================================================================
// DirectoryEntry
// ============================================================================

/// A directory entry maps a name to an inode number. Directories are just
/// files whose data blocks contain lists of these entries.
///
/// ## Analogy
///
/// A directory is like a phone book: each entry has a name ("Alice") and a
/// number (inode 23). The phone book doesn't contain Alice — it just tells
/// you how to find her.
///
/// ## Serialization Format
///
/// ```text
/// [name_length: 1 byte] [name: variable] [inode_number: 4 bytes big-endian]
/// ```
#[derive(Debug, Clone)]
pub struct DirectoryEntry {
    /// The file/directory name (max 255 chars, no '/' or null bytes).
    pub name: String,
    /// The inode this entry points to.
    pub inode_number: usize,
}

impl DirectoryEntry {
    /// Creates a new directory entry.
    pub fn new(name: &str, inode_number: usize) -> Result<Self, &'static str> {
        if name.is_empty() {
            return Err("Name cannot be empty");
        }
        if name.len() > MAX_NAME_LENGTH {
            return Err("Name too long");
        }
        if name.contains('/') {
            return Err("Name cannot contain '/'");
        }
        Ok(Self {
            name: name.to_string(),
            inode_number,
        })
    }

    /// Serializes this entry to bytes for storage in a data block.
    pub fn serialize(&self) -> Vec<u8> {
        let name_bytes = self.name.as_bytes();
        let mut result = Vec::with_capacity(1 + name_bytes.len() + 4);
        result.push(name_bytes.len() as u8);
        result.extend_from_slice(name_bytes);
        result.extend_from_slice(&(self.inode_number as u32).to_be_bytes());
        result
    }

    /// Deserializes a directory entry from bytes at the given offset.
    /// Returns the entry and the next offset, or None if invalid.
    pub fn deserialize(data: &[u8], offset: usize) -> Option<(Self, usize)> {
        if offset >= data.len() {
            return None;
        }
        let name_len = data[offset] as usize;
        if name_len == 0 {
            return None;
        }
        let name_end = offset + 1 + name_len;
        let inode_end = name_end + 4;
        if inode_end > data.len() {
            return None;
        }
        let name = std::str::from_utf8(&data[offset + 1..name_end]).ok()?;
        let inode_bytes: [u8; 4] = data[name_end..inode_end].try_into().ok()?;
        let inode_number = u32::from_be_bytes(inode_bytes) as usize;
        Some((
            Self {
                name: name.to_string(),
                inode_number,
            },
            inode_end,
        ))
    }

    /// Serializes multiple entries into a single byte vector.
    pub fn serialize_all(entries: &[DirectoryEntry]) -> Vec<u8> {
        entries.iter().flat_map(|e| e.serialize()).collect()
    }

    /// Deserializes all entries from a byte slice.
    pub fn deserialize_all(data: &[u8]) -> Vec<DirectoryEntry> {
        let mut entries = Vec::new();
        let mut offset = 0;
        while offset < data.len() {
            match Self::deserialize(data, offset) {
                Some((entry, next)) => {
                    entries.push(entry);
                    offset = next;
                }
                None => break,
            }
        }
        entries
    }
}

// ============================================================================
// BlockBitmap
// ============================================================================

/// The block bitmap tracks which data blocks are free (available for new data)
/// and which are in use. It uses one bit per block: false = free, true = used.
///
/// ## Analogy
///
/// Imagine a parking garage with numbered spaces. The attendant has a board
/// with one light per space: off means open, on means taken. When a car
/// arrives, the attendant scans for the first off light, turns it on, and
/// directs the car there.
pub struct BlockBitmap {
    /// One boolean per block: true = used, false = free.
    bitmap: Vec<bool>,
}

impl BlockBitmap {
    /// Creates a new bitmap with all blocks free.
    pub fn new(total_blocks: usize) -> Self {
        Self {
            bitmap: vec![false; total_blocks],
        }
    }

    /// Finds the first free block, marks it used, returns its number.
    /// Returns None if all blocks are in use (disk full).
    pub fn allocate(&mut self) -> Option<usize> {
        for (i, used) in self.bitmap.iter_mut().enumerate() {
            if !*used {
                *used = true;
                return Some(i);
            }
        }
        None
    }

    /// Marks a block as free.
    pub fn free(&mut self, block_number: usize) {
        if block_number < self.bitmap.len() {
            self.bitmap[block_number] = false;
        }
    }

    /// Checks whether a block is free.
    pub fn is_free(&self, block_number: usize) -> bool {
        block_number < self.bitmap.len() && !self.bitmap[block_number]
    }

    /// Returns the number of free blocks.
    pub fn free_count(&self) -> usize {
        self.bitmap.iter().filter(|&&used| !used).count()
    }

    /// Marks a block as used. Used during formatting.
    pub fn mark_used(&mut self, block_number: usize) {
        if block_number < self.bitmap.len() {
            self.bitmap[block_number] = true;
        }
    }
}

// ============================================================================
// InodeTable
// ============================================================================

/// The inode table is the master index of every file and directory on disk.
/// It holds a fixed-size array of inodes. Each slot is either occupied
/// (holding metadata) or free (available for a new file).
///
/// ## Analogy
///
/// Think of the inode table as a hotel register. There are 128 rooms.
/// When a guest checks in (file created), you assign the first available
/// room. When they check out (file deleted), you mark it vacant.
pub struct InodeTable {
    inodes: Vec<Inode>,
}

impl InodeTable {
    /// Creates a new inode table with all inodes free.
    pub fn new(total_inodes: usize) -> Self {
        let inodes = (0..total_inodes)
            .map(|i| Inode::new(i, FILE_TYPE_NONE))
            .collect();
        Self { inodes }
    }

    /// Finds the first free inode, initializes it, and returns a mutable
    /// reference. Returns None if the table is full.
    pub fn allocate(&mut self, file_type: u8) -> Option<usize> {
        for inode in self.inodes.iter_mut() {
            if inode.is_free() {
                inode.file_type = file_type;
                inode.size = 0;
                inode.permissions = 0o755;
                inode.owner_pid = 0;
                inode.link_count = 0;
                inode.direct_blocks = [None; DIRECT_BLOCKS];
                inode.indirect_block = None;
                return Some(inode.inode_number);
            }
        }
        None
    }

    /// Marks an inode as free, clearing all fields.
    pub fn free(&mut self, inode_number: usize) {
        if let Some(inode) = self.inodes.get_mut(inode_number) {
            inode.file_type = FILE_TYPE_NONE;
            inode.size = 0;
            inode.permissions = 0;
            inode.owner_pid = 0;
            inode.link_count = 0;
            inode.direct_blocks = [None; DIRECT_BLOCKS];
            inode.indirect_block = None;
        }
    }

    /// Returns a reference to the inode at the given slot.
    pub fn get(&self, inode_number: usize) -> Option<&Inode> {
        self.inodes.get(inode_number).filter(|i| !i.is_free())
    }

    /// Returns a mutable reference to the inode at the given slot.
    pub fn get_mut(&mut self, inode_number: usize) -> Option<&mut Inode> {
        self.inodes.get_mut(inode_number)
    }

    /// Returns the number of free inodes.
    pub fn free_count(&self) -> usize {
        self.inodes.iter().filter(|i| i.is_free()).count()
    }
}

// ============================================================================
// OpenFile and OpenFileTable
// ============================================================================

/// An OpenFile represents one *opening* of a file. Multiple file descriptors
/// can point to the same OpenFile, sharing the offset (this happens after
/// fork() or dup()).
///
/// ## Readable/Writable Truth Table
///
/// ```text
/// flags & 0x3  | readable? | writable?
/// -------------|-----------|----------
/// 0 (O_RDONLY) |   true    |   false
/// 1 (O_WRONLY) |   false   |   true
/// 2 (O_RDWR)   |   true    |   true
/// ```
#[derive(Debug, Clone)]
pub struct OpenFile {
    /// Which inode this open file refers to.
    pub inode_number: usize,
    /// Current byte offset within the file.
    pub offset: usize,
    /// How the file was opened (O_RDONLY, O_WRONLY, O_RDWR, plus modifiers).
    pub flags: u32,
    /// Number of file descriptors pointing here.
    pub ref_count: u32,
}

impl OpenFile {
    pub fn new(inode_number: usize, flags: u32) -> Self {
        Self {
            inode_number,
            offset: 0,
            flags,
            ref_count: 1,
        }
    }

    /// Can this open file be read from?
    pub fn is_readable(&self) -> bool {
        (self.flags & 0x3) != O_WRONLY
    }

    /// Can this open file be written to?
    pub fn is_writable(&self) -> bool {
        (self.flags & 0x3) != O_RDONLY
    }
}

/// System-wide table of all open files. Shared across all processes.
pub struct OpenFileTable {
    /// Sparse vector: index is the "global fd", None means free slot.
    entries: Vec<Option<OpenFile>>,
}

impl OpenFileTable {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Opens a file, returning the global fd index.
    pub fn open(&mut self, inode_number: usize, flags: u32) -> usize {
        let entry = OpenFile::new(inode_number, flags);
        // Find first free slot
        for (i, slot) in self.entries.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(entry);
                return i;
            }
        }
        self.entries.push(Some(entry));
        self.entries.len() - 1
    }

    /// Closes an entry by decrementing ref_count. Returns true if fully closed.
    pub fn close(&mut self, index: usize) -> bool {
        if let Some(Some(entry)) = self.entries.get_mut(index) {
            entry.ref_count -= 1;
            if entry.ref_count == 0 {
                self.entries[index] = None;
                return true;
            }
        }
        false
    }

    /// Returns a reference to the entry at the given index.
    pub fn get(&self, index: usize) -> Option<&OpenFile> {
        self.entries.get(index).and_then(|e| e.as_ref())
    }

    /// Returns a mutable reference to the entry at the given index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut OpenFile> {
        self.entries.get_mut(index).and_then(|e| e.as_mut())
    }

    /// Duplicates an entry by incrementing ref_count.
    pub fn dup(&mut self, index: usize) -> Option<usize> {
        if let Some(Some(entry)) = self.entries.get_mut(index) {
            entry.ref_count += 1;
            Some(index)
        } else {
            None
        }
    }
}

impl Default for OpenFileTable {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// FileDescriptorTable
// ============================================================================

/// Per-process file descriptor table. Maps local fd numbers (0, 1, 2, ...)
/// to indices in the system-wide OpenFileTable.
///
/// Each process gets its own FileDescriptorTable. When a process forks,
/// the child gets a clone of the parent's table but shares the same
/// OpenFileTable entries (and thus the same offsets).
pub struct FileDescriptorTable {
    /// Maps local_fd -> global_fd.
    mapping: HashMap<usize, usize>,
}

impl FileDescriptorTable {
    pub fn new() -> Self {
        Self {
            mapping: HashMap::new(),
        }
    }

    /// Allocates the lowest available local fd.
    pub fn allocate(&mut self, global_fd: usize) -> usize {
        let fd = self.lowest_free_fd();
        self.mapping.insert(fd, global_fd);
        fd
    }

    /// Removes a local fd mapping.
    pub fn close(&mut self, local_fd: usize) -> Option<usize> {
        self.mapping.remove(&local_fd)
    }

    /// Looks up which global fd a local fd maps to.
    pub fn get(&self, local_fd: usize) -> Option<usize> {
        self.mapping.get(&local_fd).copied()
    }

    /// Duplicates a file descriptor to the lowest free fd.
    pub fn dup_fd(&mut self, old_fd: usize) -> Option<usize> {
        let global = self.mapping.get(&old_fd).copied()?;
        let new_fd = self.lowest_free_fd();
        self.mapping.insert(new_fd, global);
        Some(new_fd)
    }

    /// Duplicates to a specific fd number. Closes new_fd if already open.
    pub fn dup2(&mut self, old_fd: usize, new_fd: usize) -> Option<usize> {
        let global = self.mapping.get(&old_fd).copied()?;
        self.mapping.insert(new_fd, global);
        Some(new_fd)
    }

    /// Finds the lowest integer not currently in use.
    fn lowest_free_fd(&self) -> usize {
        let mut fd = 0;
        while self.mapping.contains_key(&fd) {
            fd += 1;
        }
        fd
    }
}

impl Default for FileDescriptorTable {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// VFS (Virtual File System)
// ============================================================================

/// The VFS is the grand unifier — the layer that ties together inodes,
/// directories, block bitmaps, file descriptors, and raw block storage into
/// the familiar file system API.
///
/// ## Analogy
///
/// If the file system were a restaurant:
/// - The *disk* (block storage) is the kitchen pantry
/// - The *block bitmap* is the inventory checklist
/// - The *inode table* is the recipe index
/// - The *directories* are the menu sections
/// - The *file descriptors* are order tickets
/// - The *VFS* is the head chef who coordinates everything
pub struct VFS {
    /// The superblock describing this file system's geometry.
    pub superblock: Option<Superblock>,
    /// The inode table holding all file/directory metadata.
    inode_table: Option<InodeTable>,
    /// The block bitmap tracking free/used data blocks.
    block_bitmap: Option<BlockBitmap>,
    /// The system-wide open file table.
    open_file_table: OpenFileTable,
    /// The per-process file descriptor table.
    fd_table: FileDescriptorTable,
    /// In-memory block storage simulating the raw disk.
    blocks: Vec<Vec<u8>>,
}

impl VFS {
    /// Creates a new VFS instance. Call `format()` to initialize.
    pub fn new() -> Self {
        Self {
            superblock: None,
            inode_table: None,
            block_bitmap: None,
            open_file_table: OpenFileTable::new(),
            fd_table: FileDescriptorTable::new(),
            blocks: Vec::new(),
        }
    }

    // ========================================================================
    // format — Initialize a Blank Disk
    // ========================================================================

    /// Formats the file system, creating an empty disk with a root directory.
    ///
    /// This sets up:
    /// 1. The superblock with file system metadata
    /// 2. The inode table (all free except inode 0 for root)
    /// 3. The block bitmap (all free except root dir's data block)
    /// 4. The root directory with "." and ".." entries
    pub fn format(&mut self, total_blocks: Option<usize>, total_inodes: Option<usize>) {
        let total_blocks = total_blocks.unwrap_or(MAX_BLOCKS);
        let total_inodes = total_inodes.unwrap_or(MAX_INODES);

        // Calculate metadata blocks
        let inode_table_blocks = (total_inodes + (BLOCK_SIZE / 64) - 1) / (BLOCK_SIZE / 64);
        let metadata_blocks = 1 + inode_table_blocks + 1;
        let data_block_count = total_blocks - metadata_blocks;

        // Initialize components
        let mut superblock = Superblock::new(total_blocks, total_inodes);
        let mut inode_table = InodeTable::new(total_inodes);
        let mut block_bitmap = BlockBitmap::new(data_block_count);

        // Initialize in-memory block storage
        self.blocks = vec![vec![0u8; BLOCK_SIZE]; data_block_count];

        // Create root directory at inode 0
        let root_inode_num = inode_table.allocate(FILE_TYPE_DIRECTORY).unwrap();
        {
            let root_inode = inode_table.get_mut(root_inode_num).unwrap();
            root_inode.link_count = 2; // "." and ".."

            // Allocate one data block for root directory entries
            let root_block = block_bitmap.allocate().unwrap();
            root_inode.direct_blocks[0] = Some(root_block);

            // Create "." and ".." entries
            let dot = DirectoryEntry::new(".", ROOT_INODE).unwrap();
            let dotdot = DirectoryEntry::new("..", ROOT_INODE).unwrap();
            let dir_data = DirectoryEntry::serialize_all(&[dot, dotdot]);
            root_inode.size = dir_data.len();

            // Write to data block
            Self::write_block_static(&mut self.blocks, root_block, &dir_data);
        }

        // Update superblock
        superblock.free_blocks = block_bitmap.free_count();
        superblock.free_inodes = inode_table.free_count();

        self.superblock = Some(superblock);
        self.inode_table = Some(inode_table);
        self.block_bitmap = Some(block_bitmap);
    }

    // ========================================================================
    // open — Open a File
    // ========================================================================

    /// Opens a file at the given path. Returns the file descriptor or None.
    ///
    /// If `O_CREAT` is set and the file doesn't exist, creates it.
    /// If `O_TRUNC` is set, truncates the file to zero length.
    /// If `O_APPEND` is set, positions at end of file.
    pub fn open(&mut self, path: &str, flags: u32) -> Option<usize> {
        let mut inode_number = self.resolve_path(path);

        if inode_number.is_none() {
            if (flags & O_CREAT) != 0 {
                inode_number = self.create_file(path, FILE_TYPE_REGULAR);
                if inode_number.is_none() {
                    return None;
                }
            } else {
                return None;
            }
        }

        let ino = inode_number.unwrap();

        // O_TRUNC: truncate file
        if (flags & O_TRUNC) != 0 {
            self.truncate_inode(ino);
        }

        // Create open file table entry
        let global_fd = self.open_file_table.open(ino, flags);

        // O_APPEND: seek to end
        if (flags & O_APPEND) != 0 {
            let size = self.inode_table.as_ref().unwrap().get(ino).map(|i| i.size).unwrap_or(0);
            if let Some(entry) = self.open_file_table.get_mut(global_fd) {
                entry.offset = size;
            }
        }

        let local_fd = self.fd_table.allocate(global_fd);
        Some(local_fd)
    }

    // ========================================================================
    // close — Close a File Descriptor
    // ========================================================================

    /// Closes a file descriptor. Returns 0 on success, -1 on error.
    pub fn close(&mut self, fd: usize) -> i32 {
        let global_fd = match self.fd_table.get(fd) {
            Some(g) => g,
            None => return -1,
        };
        self.fd_table.close(fd);
        self.open_file_table.close(global_fd);
        0
    }

    // ========================================================================
    // read — Read Bytes from an Open File
    // ========================================================================

    /// Reads up to `count` bytes from the file. Returns the bytes read.
    pub fn read(&mut self, fd: usize, count: usize) -> Option<Vec<u8>> {
        let global_fd = self.fd_table.get(fd)?;
        let entry = self.open_file_table.get(global_fd)?;
        if !entry.is_readable() {
            return None;
        }

        let inode_number = entry.inode_number;
        let mut offset = entry.offset;
        let inode_table = self.inode_table.as_ref()?;
        let inode = inode_table.get(inode_number)?;

        let available = if inode.size > offset {
            inode.size - offset
        } else {
            return Some(Vec::new());
        };

        let bytes_to_read = count.min(available);
        let mut result = Vec::with_capacity(bytes_to_read);
        let mut remaining = bytes_to_read;

        // We need to read block pointers from the inode, so we clone the
        // direct_blocks and indirect_block to avoid borrow issues.
        let direct_blocks = inode.direct_blocks;
        let indirect_block = inode.indirect_block;

        while remaining > 0 {
            let block_index = offset / BLOCK_SIZE;
            let offset_in_block = offset % BLOCK_SIZE;

            let block_number =
                Self::get_block_number_static(&self.blocks, &direct_blocks, indirect_block, block_index);
            let block_number = match block_number {
                Some(b) => b,
                None => break,
            };

            let block_data = Self::read_block_static(&self.blocks, block_number);
            let bytes_from_block = remaining.min(BLOCK_SIZE - offset_in_block);
            result.extend_from_slice(&block_data[offset_in_block..offset_in_block + bytes_from_block]);

            offset += bytes_from_block;
            remaining -= bytes_from_block;
        }

        // Update offset in the open file entry
        if let Some(entry) = self.open_file_table.get_mut(global_fd) {
            entry.offset = offset;
        }

        Some(result)
    }

    // ========================================================================
    // write — Write Bytes to an Open File
    // ========================================================================

    /// Writes data to the file. Returns the number of bytes written.
    pub fn write(&mut self, fd: usize, data: &[u8]) -> i32 {
        let global_fd = match self.fd_table.get(fd) {
            Some(g) => g,
            None => return -1,
        };

        // Check writable
        let (inode_number, flags) = match self.open_file_table.get(global_fd) {
            Some(entry) => {
                if !entry.is_writable() {
                    return -1;
                }
                (entry.inode_number, entry.flags)
            }
            None => return -1,
        };

        // Get current offset, handling O_APPEND
        let mut offset = if (flags & O_APPEND) != 0 {
            self.inode_table
                .as_ref()
                .and_then(|t| t.get(inode_number))
                .map(|i| i.size)
                .unwrap_or(0)
        } else {
            self.open_file_table
                .get(global_fd)
                .map(|e| e.offset)
                .unwrap_or(0)
        };

        let mut bytes_written: usize = 0;
        let mut remaining = data.len();

        while remaining > 0 {
            let block_index = offset / BLOCK_SIZE;
            let offset_in_block = offset % BLOCK_SIZE;

            // Get or allocate block
            let block_number = {
                let inode_table = self.inode_table.as_ref().unwrap();
                let inode = match inode_table.get(inode_number) {
                    Some(i) => i,
                    None => return bytes_written as i32,
                };
                Self::get_block_number_static(
                    &self.blocks,
                    &inode.direct_blocks,
                    inode.indirect_block,
                    block_index,
                )
            };

            let block_number = match block_number {
                Some(b) => b,
                None => {
                    // Need to allocate
                    match self.allocate_block_for_inode(inode_number, block_index) {
                        Some(b) => b,
                        None => return bytes_written as i32, // Disk full
                    }
                }
            };

            // Read existing block, modify, write back
            let mut block_data = Self::read_block_static(&self.blocks, block_number);
            let bytes_to_write = remaining.min(BLOCK_SIZE - offset_in_block);
            block_data[offset_in_block..offset_in_block + bytes_to_write]
                .copy_from_slice(&data[bytes_written..bytes_written + bytes_to_write]);
            Self::write_block_static(&mut self.blocks, block_number, &block_data);

            offset += bytes_to_write;
            bytes_written += bytes_to_write;
            remaining -= bytes_to_write;

            // Update inode size
            if let Some(inode_table) = self.inode_table.as_mut() {
                if let Some(inode) = inode_table.get_mut(inode_number) {
                    if offset > inode.size {
                        inode.size = offset;
                    }
                }
            }
        }

        // Update offset in open file entry
        if let Some(entry) = self.open_file_table.get_mut(global_fd) {
            entry.offset = offset;
        }

        // Update superblock
        if let (Some(sb), Some(bm)) = (self.superblock.as_mut(), self.block_bitmap.as_ref()) {
            sb.free_blocks = bm.free_count();
        }

        bytes_written as i32
    }

    // ========================================================================
    // lseek — Reposition the File Offset
    // ========================================================================

    /// Repositions the file offset. Returns the new offset or None on error.
    ///
    /// `whence` controls interpretation:
    /// - `SEEK_SET`: offset is absolute
    /// - `SEEK_CUR`: offset is relative to current position
    /// - `SEEK_END`: offset is relative to end of file
    pub fn lseek(&mut self, fd: usize, offset: i64, whence: u32) -> Option<usize> {
        let global_fd = self.fd_table.get(fd)?;

        let current_offset = self.open_file_table.get(global_fd)?.offset;
        let inode_number = self.open_file_table.get(global_fd)?.inode_number;
        let file_size = self
            .inode_table
            .as_ref()?
            .get(inode_number)
            .map(|i| i.size)
            .unwrap_or(0);

        let new_offset: i64 = match whence {
            SEEK_SET => offset,
            SEEK_CUR => current_offset as i64 + offset,
            SEEK_END => file_size as i64 + offset,
            _ => return None,
        };

        if new_offset < 0 {
            return None;
        }

        let new_offset = new_offset as usize;
        if let Some(entry) = self.open_file_table.get_mut(global_fd) {
            entry.offset = new_offset;
        }
        Some(new_offset)
    }

    // ========================================================================
    // stat — Get File Metadata
    // ========================================================================

    /// Returns a clone of the inode for a given path.
    pub fn stat(&self, path: &str) -> Option<Inode> {
        let inode_number = self.resolve_path(path)?;
        self.inode_table.as_ref()?.get(inode_number).cloned()
    }

    // ========================================================================
    // mkdir — Create a Directory
    // ========================================================================

    /// Creates a directory at the given path. Returns 0 on success, -1 on error.
    pub fn mkdir(&mut self, path: &str) -> i32 {
        let (parent_path, name) = match Self::split_path(path) {
            Some(p) => p,
            None => return -1,
        };

        let parent_inode_num = match self.resolve_path(&parent_path) {
            Some(n) => n,
            None => return -1,
        };

        // Verify parent is a directory
        {
            let inode_table = match self.inode_table.as_ref() {
                Some(t) => t,
                None => return -1,
            };
            match inode_table.get(parent_inode_num) {
                Some(i) if i.is_directory() => {}
                _ => return -1,
            }
        }

        // Check name doesn't already exist
        if self.resolve_in_directory(parent_inode_num, &name).is_some() {
            return -1;
        }

        // Allocate new inode
        let new_inode_num = match self.inode_table.as_mut().unwrap().allocate(FILE_TYPE_DIRECTORY) {
            Some(n) => n,
            None => return -1,
        };

        // Allocate data block
        let block_num = match self.block_bitmap.as_mut().unwrap().allocate() {
            Some(b) => b,
            None => {
                self.inode_table.as_mut().unwrap().free(new_inode_num);
                return -1;
            }
        };

        // Set up the new directory inode
        {
            let inode = self.inode_table.as_mut().unwrap().get_mut(new_inode_num).unwrap();
            inode.link_count = 2;
            inode.direct_blocks[0] = Some(block_num);

            let dot = DirectoryEntry::new(".", new_inode_num).unwrap();
            let dotdot = DirectoryEntry::new("..", parent_inode_num).unwrap();
            let dir_data = DirectoryEntry::serialize_all(&[dot, dotdot]);
            inode.size = dir_data.len();
            Self::write_block_static(&mut self.blocks, block_num, &dir_data);
        }

        // Add entry in parent directory
        self.add_directory_entry(parent_inode_num, &name, new_inode_num);

        // Increment parent link count (because ".." points to parent)
        if let Some(parent_inode) = self.inode_table.as_mut().unwrap().get_mut(parent_inode_num) {
            parent_inode.link_count += 1;
        }

        // Update superblock
        if let (Some(sb), Some(bm), Some(it)) = (
            self.superblock.as_mut(),
            self.block_bitmap.as_ref(),
            self.inode_table.as_ref(),
        ) {
            sb.free_blocks = bm.free_count();
            sb.free_inodes = it.free_count();
        }

        0
    }

    // ========================================================================
    // readdir — List Directory Entries
    // ========================================================================

    /// Returns all directory entries for the given path.
    pub fn readdir(&self, path: &str) -> Option<Vec<DirectoryEntry>> {
        let inode_number = self.resolve_path(path)?;
        let inode_table = self.inode_table.as_ref()?;
        let inode = inode_table.get(inode_number)?;
        if !inode.is_directory() {
            return None;
        }
        Some(self.read_directory_entries(inode))
    }

    // ========================================================================
    // unlink — Remove a Directory Entry
    // ========================================================================

    /// Removes a file at the given path. Returns 0 on success, -1 on error.
    /// Does not remove directories (use rmdir for that).
    pub fn unlink(&mut self, path: &str) -> i32 {
        let (parent_path, name) = match Self::split_path(path) {
            Some(p) => p,
            None => return -1,
        };

        let parent_inode_num = match self.resolve_path(&parent_path) {
            Some(n) => n,
            None => return -1,
        };

        let target_inode_num = match self.resolve_in_directory(parent_inode_num, &name) {
            Some(n) => n,
            None => return -1,
        };

        // Don't unlink directories
        {
            let inode_table = self.inode_table.as_ref().unwrap();
            match inode_table.get(target_inode_num) {
                Some(i) if i.is_directory() => return -1,
                None => return -1,
                _ => {}
            }
        }

        // Remove entry from parent
        self.remove_directory_entry(parent_inode_num, &name);

        // Decrement link count
        let should_free = {
            let inode = self.inode_table.as_mut().unwrap().get_mut(target_inode_num).unwrap();
            if inode.link_count > 0 {
                inode.link_count -= 1;
            }
            inode.link_count == 0
        };

        if should_free {
            self.free_inode_blocks(target_inode_num);
            self.inode_table.as_mut().unwrap().free(target_inode_num);
        }

        // Update superblock
        if let (Some(sb), Some(bm), Some(it)) = (
            self.superblock.as_mut(),
            self.block_bitmap.as_ref(),
            self.inode_table.as_ref(),
        ) {
            sb.free_blocks = bm.free_count();
            sb.free_inodes = it.free_count();
        }

        0
    }

    // ========================================================================
    // resolve_path — Walk the Directory Tree
    // ========================================================================

    /// Resolves an absolute path to an inode number by walking from root.
    ///
    /// Algorithm:
    /// 1. Start at root inode (inode 0)
    /// 2. Split path by "/"
    /// 3. For each component, look it up in the current directory
    /// 4. Move to the inode the entry points to
    pub fn resolve_path(&self, path: &str) -> Option<usize> {
        if path == "/" {
            return Some(ROOT_INODE);
        }

        let components: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        if components.is_empty() {
            return Some(ROOT_INODE);
        }

        let mut current = ROOT_INODE;
        for component in &components {
            current = self.resolve_in_directory(current, component)?;
        }
        Some(current)
    }

    // ========================================================================
    // dup / dup2 — Duplicate File Descriptors
    // ========================================================================

    /// Duplicates fd to the lowest free fd number.
    pub fn dup_fd(&mut self, old_fd: usize) -> Option<usize> {
        let global_fd = self.fd_table.get(old_fd)?;
        self.open_file_table.dup(global_fd)?;
        self.fd_table.dup_fd(old_fd)
    }

    /// Duplicates fd to a specific fd number.
    pub fn dup2_fd(&mut self, old_fd: usize, new_fd: usize) -> Option<usize> {
        let global_fd = self.fd_table.get(old_fd)?;

        // Close new_fd if open
        if let Some(existing_global) = self.fd_table.get(new_fd) {
            self.open_file_table.close(existing_global);
        }

        self.open_file_table.dup(global_fd)?;
        self.fd_table.dup2(old_fd, new_fd)
    }

    // ========================================================================
    // Private Helpers
    // ========================================================================

    /// Reads a data block from in-memory storage.
    fn read_block_static(blocks: &[Vec<u8>], block_number: usize) -> Vec<u8> {
        if block_number < blocks.len() {
            blocks[block_number].clone()
        } else {
            vec![0u8; BLOCK_SIZE]
        }
    }

    /// Writes data to a block, padding with zeros if needed.
    fn write_block_static(blocks: &mut [Vec<u8>], block_number: usize, data: &[u8]) {
        if block_number < blocks.len() {
            let mut padded = vec![0u8; BLOCK_SIZE];
            let len = data.len().min(BLOCK_SIZE);
            padded[..len].copy_from_slice(&data[..len]);
            blocks[block_number] = padded;
        }
    }

    /// Splits a path into (parent_path, name).
    fn split_path(path: &str) -> Option<(String, String)> {
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            return None;
        }
        let name = parts.last().unwrap().to_string();
        let parent = if parts.len() == 1 {
            "/".to_string()
        } else {
            format!("/{}", parts[..parts.len() - 1].join("/"))
        };
        Some((parent, name))
    }

    /// Looks up a name in a directory.
    fn resolve_in_directory(&self, dir_inode_num: usize, name: &str) -> Option<usize> {
        let inode_table = self.inode_table.as_ref()?;
        let inode = inode_table.get(dir_inode_num)?;
        if !inode.is_directory() {
            return None;
        }
        let entries = self.read_directory_entries(inode);
        entries.iter().find(|e| e.name == name).map(|e| e.inode_number)
    }

    /// Reads all directory entries from a directory inode.
    fn read_directory_entries(&self, inode: &Inode) -> Vec<DirectoryEntry> {
        let mut data = Vec::new();
        for i in 0..DIRECT_BLOCKS {
            match inode.direct_blocks[i] {
                Some(block_num) => {
                    data.extend_from_slice(&Self::read_block_static(&self.blocks, block_num));
                }
                None => break,
            }
        }
        // Trim to actual size
        if inode.size < data.len() {
            data.truncate(inode.size);
        }
        DirectoryEntry::deserialize_all(&data)
    }

    /// Adds a new entry to a directory.
    fn add_directory_entry(&mut self, dir_inode_num: usize, name: &str, inode_number: usize) {
        let inode_table = self.inode_table.as_ref().unwrap();
        let inode = inode_table.get(dir_inode_num).unwrap();
        let mut entries = self.read_directory_entries(inode);
        entries.push(DirectoryEntry::new(name, inode_number).unwrap());

        let data = DirectoryEntry::serialize_all(&entries);
        let size = data.len();

        // Write back to blocks
        self.write_data_to_inode(dir_inode_num, &data);

        let inode = self.inode_table.as_mut().unwrap().get_mut(dir_inode_num).unwrap();
        inode.size = size;
    }

    /// Removes a named entry from a directory.
    fn remove_directory_entry(&mut self, dir_inode_num: usize, name: &str) {
        let inode_table = self.inode_table.as_ref().unwrap();
        let inode = inode_table.get(dir_inode_num).unwrap();
        let entries: Vec<DirectoryEntry> = self
            .read_directory_entries(inode)
            .into_iter()
            .filter(|e| e.name != name)
            .collect();

        let data = DirectoryEntry::serialize_all(&entries);
        let size = data.len();

        self.write_data_to_inode(dir_inode_num, &data);

        let inode = self.inode_table.as_mut().unwrap().get_mut(dir_inode_num).unwrap();
        inode.size = size;
    }

    /// Writes data across an inode's blocks, allocating as needed.
    fn write_data_to_inode(&mut self, inode_num: usize, data: &[u8]) {
        let mut block_index = 0;
        let mut offset = 0;

        while offset < data.len() {
            let block_num = {
                let inode = self.inode_table.as_ref().unwrap().get(inode_num).unwrap();
                inode.direct_blocks[block_index]
            };

            let block_num = match block_num {
                Some(b) => b,
                None => {
                    let b = match self.block_bitmap.as_mut().unwrap().allocate() {
                        Some(b) => b,
                        None => return,
                    };
                    let inode = self.inode_table.as_mut().unwrap().get_mut(inode_num).unwrap();
                    inode.direct_blocks[block_index] = Some(b);
                    b
                }
            };

            let end = (offset + BLOCK_SIZE).min(data.len());
            Self::write_block_static(&mut self.blocks, block_num, &data[offset..end]);

            offset += BLOCK_SIZE;
            block_index += 1;
        }
    }

    /// Creates a new file at the given path.
    fn create_file(&mut self, path: &str, file_type: u8) -> Option<usize> {
        let (parent_path, name) = Self::split_path(path)?;

        let parent_inode_num = self.resolve_path(&parent_path)?;

        // Verify parent is a directory
        {
            let inode_table = self.inode_table.as_ref()?;
            if !inode_table.get(parent_inode_num)?.is_directory() {
                return None;
            }
        }

        let new_inode_num = self.inode_table.as_mut()?.allocate(file_type)?;
        {
            let inode = self.inode_table.as_mut()?.get_mut(new_inode_num)?;
            inode.link_count = 1;
        }

        self.add_directory_entry(parent_inode_num, &name, new_inode_num);

        if let (Some(sb), Some(it)) = (self.superblock.as_mut(), self.inode_table.as_ref()) {
            sb.free_inodes = it.free_count();
        }

        Some(new_inode_num)
    }

    /// Truncates a file to zero length.
    fn truncate_inode(&mut self, inode_number: usize) {
        self.free_inode_blocks(inode_number);
        if let Some(inode) = self.inode_table.as_mut().unwrap().get_mut(inode_number) {
            inode.size = 0;
        }
        if let (Some(sb), Some(bm)) = (self.superblock.as_mut(), self.block_bitmap.as_ref()) {
            sb.free_blocks = bm.free_count();
        }
    }

    /// Frees all data blocks owned by an inode.
    fn free_inode_blocks(&mut self, inode_number: usize) {
        let (direct_blocks, indirect_block) = {
            let inode = match self.inode_table.as_ref().unwrap().get(inode_number) {
                Some(i) => i,
                None => return,
            };
            (inode.direct_blocks, inode.indirect_block)
        };

        let bitmap = self.block_bitmap.as_mut().unwrap();

        // Free direct blocks
        for block_opt in &direct_blocks {
            if let Some(block_num) = block_opt {
                bitmap.free(*block_num);
            }
        }

        // Free indirect block and its pointers
        if let Some(indirect_num) = indirect_block {
            let indirect_data = Self::read_block_static(&self.blocks, indirect_num);
            for i in 0..(BLOCK_SIZE / 4) {
                let offset = i * 4;
                if offset + 4 > indirect_data.len() {
                    break;
                }
                let ptr = u32::from_be_bytes([
                    indirect_data[offset],
                    indirect_data[offset + 1],
                    indirect_data[offset + 2],
                    indirect_data[offset + 3],
                ]) as usize;
                if ptr > 0 {
                    bitmap.free(ptr);
                }
            }
            bitmap.free(indirect_num);
        }

        // Clear block pointers in inode
        if let Some(inode) = self.inode_table.as_mut().unwrap().get_mut(inode_number) {
            inode.direct_blocks = [None; DIRECT_BLOCKS];
            inode.indirect_block = None;
        }
    }

    /// Gets the block number for a logical block index using static refs.
    fn get_block_number_static(
        blocks: &[Vec<u8>],
        direct_blocks: &[Option<usize>; DIRECT_BLOCKS],
        indirect_block: Option<usize>,
        block_index: usize,
    ) -> Option<usize> {
        if block_index < DIRECT_BLOCKS {
            direct_blocks[block_index]
        } else if let Some(indirect_num) = indirect_block {
            let indirect_data = Self::read_block_static(blocks, indirect_num);
            let idx = block_index - DIRECT_BLOCKS;
            let offset = idx * 4;
            if offset + 4 > BLOCK_SIZE {
                return None;
            }
            let ptr = u32::from_be_bytes([
                indirect_data[offset],
                indirect_data[offset + 1],
                indirect_data[offset + 2],
                indirect_data[offset + 3],
            ]) as usize;
            if ptr > 0 {
                Some(ptr)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Allocates a data block for a logical block index in an inode.
    fn allocate_block_for_inode(&mut self, inode_number: usize, block_index: usize) -> Option<usize> {
        if block_index < DIRECT_BLOCKS {
            let block_num = self.block_bitmap.as_mut()?.allocate()?;
            let inode = self.inode_table.as_mut()?.get_mut(inode_number)?;
            inode.direct_blocks[block_index] = Some(block_num);
            Some(block_num)
        } else {
            let indirect_index = block_index - DIRECT_BLOCKS;
            let max_indirect = BLOCK_SIZE / 4;
            if indirect_index >= max_indirect {
                return None; // Exceeded max file size
            }

            // Allocate indirect block if needed
            let indirect_num = {
                let inode = self.inode_table.as_ref()?.get(inode_number)?;
                inode.indirect_block
            };

            let indirect_num = match indirect_num {
                Some(n) => n,
                None => {
                    let n = self.block_bitmap.as_mut()?.allocate()?;
                    Self::write_block_static(&mut self.blocks, n, &vec![0u8; BLOCK_SIZE]);
                    let inode = self.inode_table.as_mut()?.get_mut(inode_number)?;
                    inode.indirect_block = Some(n);
                    n
                }
            };

            // Allocate the data block
            let block_num = self.block_bitmap.as_mut()?.allocate()?;

            // Write pointer into indirect block
            let mut indirect_data = Self::read_block_static(&self.blocks, indirect_num);
            let ptr_offset = indirect_index * 4;
            indirect_data[ptr_offset..ptr_offset + 4]
                .copy_from_slice(&(block_num as u32).to_be_bytes());
            Self::write_block_static(&mut self.blocks, indirect_num, &indirect_data);

            Some(block_num)
        }
    }
}

impl Default for VFS {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // === Superblock Tests ===

    #[test]
    fn test_superblock_default() {
        let sb = Superblock::new(MAX_BLOCKS, MAX_INODES);
        assert_eq!(sb.magic, MAGIC);
        assert_eq!(sb.block_size, BLOCK_SIZE);
        assert_eq!(sb.total_blocks, MAX_BLOCKS);
        assert_eq!(sb.total_inodes, MAX_INODES);
        assert!(sb.is_valid());
    }

    #[test]
    fn test_superblock_invalid_magic() {
        let mut sb = Superblock::new(MAX_BLOCKS, MAX_INODES);
        sb.magic = 0xDEADBEEF;
        assert!(!sb.is_valid());
    }

    #[test]
    fn test_superblock_custom_sizes() {
        let sb = Superblock::new(1024, 256);
        assert_eq!(sb.total_blocks, 1024);
        assert_eq!(sb.total_inodes, 256);
    }

    // === Inode Tests ===

    #[test]
    fn test_inode_free() {
        let inode = Inode::new(0, FILE_TYPE_NONE);
        assert!(inode.is_free());
        assert!(!inode.is_directory());
        assert!(!inode.is_regular());
    }

    #[test]
    fn test_inode_directory() {
        let inode = Inode::new(5, FILE_TYPE_DIRECTORY);
        assert!(inode.is_directory());
        assert!(!inode.is_free());
        assert!(!inode.is_regular());
    }

    #[test]
    fn test_inode_regular() {
        let inode = Inode::new(10, FILE_TYPE_REGULAR);
        assert!(inode.is_regular());
        assert!(!inode.is_directory());
        assert!(!inode.is_free());
    }

    #[test]
    fn test_inode_defaults() {
        let inode = Inode::new(7, FILE_TYPE_REGULAR);
        assert_eq!(inode.inode_number, 7);
        assert_eq!(inode.size, 0);
        assert_eq!(inode.permissions, 0o755);
        assert_eq!(inode.link_count, 0);
        assert!(inode.direct_blocks.iter().all(|b| b.is_none()));
        assert!(inode.indirect_block.is_none());
    }

    // === DirectoryEntry Tests ===

    #[test]
    fn test_directory_entry_create() {
        let entry = DirectoryEntry::new("hello.txt", 42).unwrap();
        assert_eq!(entry.name, "hello.txt");
        assert_eq!(entry.inode_number, 42);
    }

    #[test]
    fn test_directory_entry_invalid() {
        assert!(DirectoryEntry::new("", 0).is_err());
        assert!(DirectoryEntry::new("a/b", 0).is_err());
        assert!(DirectoryEntry::new(&"x".repeat(256), 0).is_err());
    }

    #[test]
    fn test_directory_entry_serialize_roundtrip() {
        let entry = DirectoryEntry::new("test.txt", 99).unwrap();
        let data = entry.serialize();
        let (decoded, next) = DirectoryEntry::deserialize(&data, 0).unwrap();
        assert_eq!(decoded.name, "test.txt");
        assert_eq!(decoded.inode_number, 99);
        assert_eq!(next, data.len());
    }

    #[test]
    fn test_directory_entry_serialize_all_roundtrip() {
        let entries = vec![
            DirectoryEntry::new(".", 0).unwrap(),
            DirectoryEntry::new("..", 0).unwrap(),
            DirectoryEntry::new("file.txt", 5).unwrap(),
        ];
        let data = DirectoryEntry::serialize_all(&entries);
        let decoded = DirectoryEntry::deserialize_all(&data);
        assert_eq!(decoded.len(), 3);
        assert_eq!(decoded[0].name, ".");
        assert_eq!(decoded[1].name, "..");
        assert_eq!(decoded[2].name, "file.txt");
    }

    #[test]
    fn test_directory_entry_deserialize_empty() {
        assert!(DirectoryEntry::deserialize(&[], 0).is_none());
    }

    #[test]
    fn test_directory_entry_deserialize_truncated() {
        // Claims 5-byte name but only has 3
        assert!(DirectoryEntry::deserialize(&[5, b'a', b'b', b'c'], 0).is_none());
    }

    // === BlockBitmap Tests ===

    #[test]
    fn test_bitmap_all_free() {
        let bm = BlockBitmap::new(10);
        assert_eq!(bm.free_count(), 10);
        for i in 0..10 {
            assert!(bm.is_free(i));
        }
    }

    #[test]
    fn test_bitmap_allocate() {
        let mut bm = BlockBitmap::new(10);
        assert_eq!(bm.allocate(), Some(0));
        assert_eq!(bm.allocate(), Some(1));
        assert!(!bm.is_free(0));
        assert!(!bm.is_free(1));
        assert_eq!(bm.free_count(), 8);
    }

    #[test]
    fn test_bitmap_free() {
        let mut bm = BlockBitmap::new(10);
        let b = bm.allocate().unwrap();
        bm.free(b);
        assert!(bm.is_free(b));
        assert_eq!(bm.free_count(), 10);
    }

    #[test]
    fn test_bitmap_reuse() {
        let mut bm = BlockBitmap::new(10);
        for _ in 0..10 {
            bm.allocate();
        }
        bm.free(3);
        assert_eq!(bm.allocate(), Some(3));
    }

    #[test]
    fn test_bitmap_exhaustion() {
        let mut bm = BlockBitmap::new(3);
        assert!(bm.allocate().is_some());
        assert!(bm.allocate().is_some());
        assert!(bm.allocate().is_some());
        assert!(bm.allocate().is_none());
    }

    #[test]
    fn test_bitmap_mark_used() {
        let mut bm = BlockBitmap::new(10);
        bm.mark_used(5);
        assert!(!bm.is_free(5));
        assert_eq!(bm.free_count(), 9);
    }

    // === InodeTable Tests ===

    #[test]
    fn test_inode_table_allocate() {
        let mut table = InodeTable::new(5);
        let i1 = table.allocate(FILE_TYPE_REGULAR).unwrap();
        let i2 = table.allocate(FILE_TYPE_DIRECTORY).unwrap();
        assert_ne!(i1, i2);
    }

    #[test]
    fn test_inode_table_free() {
        let mut table = InodeTable::new(5);
        let num = table.allocate(FILE_TYPE_REGULAR).unwrap();
        table.free(num);
        assert!(table.get(num).is_none()); // get filters out free inodes
    }

    #[test]
    fn test_inode_table_exhaustion() {
        let mut table = InodeTable::new(3);
        assert!(table.allocate(FILE_TYPE_REGULAR).is_some());
        assert!(table.allocate(FILE_TYPE_REGULAR).is_some());
        assert!(table.allocate(FILE_TYPE_REGULAR).is_some());
        assert!(table.allocate(FILE_TYPE_REGULAR).is_none());
    }

    #[test]
    fn test_inode_table_reuse() {
        let mut table = InodeTable::new(3);
        table.allocate(FILE_TYPE_REGULAR);
        table.allocate(FILE_TYPE_REGULAR);
        table.allocate(FILE_TYPE_REGULAR);
        table.free(1);
        let num = table.allocate(FILE_TYPE_DIRECTORY).unwrap();
        assert_eq!(num, 1);
    }

    #[test]
    fn test_inode_table_free_count() {
        let mut table = InodeTable::new(5);
        assert_eq!(table.free_count(), 5);
        table.allocate(FILE_TYPE_REGULAR);
        assert_eq!(table.free_count(), 4);
    }

    #[test]
    fn test_inode_table_get_out_of_range() {
        let table = InodeTable::new(5);
        assert!(table.get(5).is_none());
        assert!(table.get(100).is_none());
    }

    // === OpenFile Tests ===

    #[test]
    fn test_open_file_readable() {
        assert!(OpenFile::new(0, O_RDONLY).is_readable());
        assert!(!OpenFile::new(0, O_WRONLY).is_readable());
        assert!(OpenFile::new(0, O_RDWR).is_readable());
    }

    #[test]
    fn test_open_file_writable() {
        assert!(!OpenFile::new(0, O_RDONLY).is_writable());
        assert!(OpenFile::new(0, O_WRONLY).is_writable());
        assert!(OpenFile::new(0, O_RDWR).is_writable());
    }

    // === OpenFileTable Tests ===

    #[test]
    fn test_open_file_table_open_close() {
        let mut table = OpenFileTable::new();
        let idx = table.open(10, O_RDONLY);
        assert!(table.get(idx).is_some());
        assert!(table.close(idx));
        assert!(table.get(idx).is_none());
    }

    #[test]
    fn test_open_file_table_dup() {
        let mut table = OpenFileTable::new();
        let idx = table.open(10, O_RDONLY);
        table.dup(idx);
        assert_eq!(table.get(idx).unwrap().ref_count, 2);
        assert!(!table.close(idx)); // ref_count goes to 1
        assert!(table.get(idx).is_some());
    }

    #[test]
    fn test_open_file_table_slot_reuse() {
        let mut table = OpenFileTable::new();
        let idx1 = table.open(10, O_RDONLY);
        table.close(idx1);
        let idx2 = table.open(20, O_WRONLY);
        assert_eq!(idx1, idx2);
    }

    // === FileDescriptorTable Tests ===

    #[test]
    fn test_fd_table_allocate() {
        let mut table = FileDescriptorTable::new();
        assert_eq!(table.allocate(100), 0);
        assert_eq!(table.allocate(200), 1);
        assert_eq!(table.get(0), Some(100));
        assert_eq!(table.get(1), Some(200));
    }

    #[test]
    fn test_fd_table_close() {
        let mut table = FileDescriptorTable::new();
        table.allocate(100);
        table.close(0);
        assert_eq!(table.get(0), None);
    }

    #[test]
    fn test_fd_table_reuse() {
        let mut table = FileDescriptorTable::new();
        table.allocate(100);
        table.allocate(200);
        table.close(0);
        assert_eq!(table.allocate(300), 0);
    }

    #[test]
    fn test_fd_table_dup() {
        let mut table = FileDescriptorTable::new();
        table.allocate(100);
        let new_fd = table.dup_fd(0).unwrap();
        assert_ne!(new_fd, 0);
        assert_eq!(table.get(new_fd), table.get(0));
    }

    #[test]
    fn test_fd_table_dup2() {
        let mut table = FileDescriptorTable::new();
        table.allocate(100);
        table.allocate(200);
        table.dup2(0, 5).unwrap();
        assert_eq!(table.get(5), Some(100));
    }

    #[test]
    fn test_fd_table_dup_invalid() {
        let mut table = FileDescriptorTable::new();
        assert!(table.dup_fd(999).is_none());
    }

    // === VFS Tests ===

    fn new_formatted_vfs() -> VFS {
        let mut vfs = VFS::new();
        vfs.format(None, None);
        vfs
    }

    #[test]
    fn test_format_creates_valid_superblock() {
        let vfs = new_formatted_vfs();
        assert!(vfs.superblock.as_ref().unwrap().is_valid());
    }

    #[test]
    fn test_format_creates_root_directory() {
        let vfs = new_formatted_vfs();
        let inode = vfs.stat("/").unwrap();
        assert!(inode.is_directory());
        assert_eq!(inode.link_count, 2);
    }

    #[test]
    fn test_format_root_has_dot_entries() {
        let vfs = new_formatted_vfs();
        let entries = vfs.readdir("/").unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"."));
        assert!(names.contains(&".."));
    }

    #[test]
    fn test_mkdir() {
        let mut vfs = new_formatted_vfs();
        assert_eq!(vfs.mkdir("/data"), 0);
        let inode = vfs.stat("/data").unwrap();
        assert!(inode.is_directory());
    }

    #[test]
    fn test_mkdir_dot_entries() {
        let mut vfs = new_formatted_vfs();
        vfs.mkdir("/data");
        let entries = vfs.readdir("/data").unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"."));
        assert!(names.contains(&".."));
    }

    #[test]
    fn test_mkdir_dotdot_points_to_parent() {
        let mut vfs = new_formatted_vfs();
        vfs.mkdir("/data");
        let entries = vfs.readdir("/data").unwrap();
        let dotdot = entries.iter().find(|e| e.name == "..").unwrap();
        assert_eq!(dotdot.inode_number, ROOT_INODE);
    }

    #[test]
    fn test_mkdir_nested() {
        let mut vfs = new_formatted_vfs();
        vfs.mkdir("/a");
        vfs.mkdir("/a/b");
        vfs.mkdir("/a/b/c");
        assert!(vfs.stat("/a/b/c").unwrap().is_directory());
    }

    #[test]
    fn test_mkdir_duplicate_fails() {
        let mut vfs = new_formatted_vfs();
        assert_eq!(vfs.mkdir("/data"), 0);
        assert_eq!(vfs.mkdir("/data"), -1);
    }

    #[test]
    fn test_mkdir_nonexistent_parent_fails() {
        let mut vfs = new_formatted_vfs();
        assert_eq!(vfs.mkdir("/no/such/path"), -1);
    }

    #[test]
    fn test_write_read_roundtrip() {
        let mut vfs = new_formatted_vfs();
        let fd = vfs.open("/hello.txt", O_WRONLY | O_CREAT).unwrap();
        assert_eq!(vfs.write(fd, b"Hello, world!"), 13);
        vfs.close(fd);

        let fd = vfs.open("/hello.txt", O_RDONLY).unwrap();
        let data = vfs.read(fd, 100).unwrap();
        assert_eq!(&data, b"Hello, world!");
        vfs.close(fd);
    }

    #[test]
    fn test_write_multiple_blocks() {
        let mut vfs = new_formatted_vfs();
        let big_data = vec![b'A'; 1500];
        let fd = vfs.open("/big.txt", O_WRONLY | O_CREAT).unwrap();
        vfs.write(fd, &big_data);
        vfs.close(fd);

        let fd = vfs.open("/big.txt", O_RDONLY).unwrap();
        let result = vfs.read(fd, 2000).unwrap();
        assert_eq!(result, big_data);
        vfs.close(fd);
    }

    #[test]
    fn test_write_to_subdirectory() {
        let mut vfs = new_formatted_vfs();
        vfs.mkdir("/data");
        let fd = vfs.open("/data/log.txt", O_WRONLY | O_CREAT).unwrap();
        vfs.write(fd, b"log entry 1");
        vfs.close(fd);

        let fd = vfs.open("/data/log.txt", O_RDONLY).unwrap();
        let data = vfs.read(fd, 100).unwrap();
        assert_eq!(&data, b"log entry 1");
    }

    #[test]
    fn test_open_creat() {
        let mut vfs = new_formatted_vfs();
        let fd = vfs.open("/new.txt", O_WRONLY | O_CREAT).unwrap();
        vfs.close(fd);
        assert!(vfs.stat("/new.txt").unwrap().is_regular());
    }

    #[test]
    fn test_open_nonexistent_no_creat() {
        let mut vfs = new_formatted_vfs();
        assert!(vfs.open("/nonexistent.txt", O_RDONLY).is_none());
    }

    #[test]
    fn test_append_mode() {
        let mut vfs = new_formatted_vfs();
        let fd = vfs.open("/log.txt", O_WRONLY | O_CREAT).unwrap();
        vfs.write(fd, b"first");
        vfs.close(fd);

        let fd = vfs.open("/log.txt", O_WRONLY | O_APPEND).unwrap();
        vfs.write(fd, b" second");
        vfs.close(fd);

        let fd = vfs.open("/log.txt", O_RDONLY).unwrap();
        let data = vfs.read(fd, 100).unwrap();
        assert_eq!(&data, b"first second");
    }

    #[test]
    fn test_trunc_mode() {
        let mut vfs = new_formatted_vfs();
        let fd = vfs.open("/file.txt", O_WRONLY | O_CREAT).unwrap();
        vfs.write(fd, b"original content");
        vfs.close(fd);

        let fd = vfs.open("/file.txt", O_WRONLY | O_TRUNC).unwrap();
        vfs.write(fd, b"new");
        vfs.close(fd);

        let fd = vfs.open("/file.txt", O_RDONLY).unwrap();
        let data = vfs.read(fd, 100).unwrap();
        assert_eq!(&data, b"new");
    }

    #[test]
    fn test_lseek_set() {
        let mut vfs = new_formatted_vfs();
        let fd = vfs.open("/seek.txt", O_RDWR | O_CREAT).unwrap();
        vfs.write(fd, b"Hello, world!");
        let pos = vfs.lseek(fd, 7, SEEK_SET).unwrap();
        assert_eq!(pos, 7);
        let data = vfs.read(fd, 100).unwrap();
        assert_eq!(&data, b"world!");
    }

    #[test]
    fn test_lseek_cur() {
        let mut vfs = new_formatted_vfs();
        let fd = vfs.open("/seek.txt", O_RDWR | O_CREAT).unwrap();
        vfs.write(fd, b"abcdefghij");
        vfs.lseek(fd, 2, SEEK_SET);
        let pos = vfs.lseek(fd, 3, SEEK_CUR).unwrap();
        assert_eq!(pos, 5);
        let data = vfs.read(fd, 100).unwrap();
        assert_eq!(&data, b"fghij");
    }

    #[test]
    fn test_lseek_end() {
        let mut vfs = new_formatted_vfs();
        let fd = vfs.open("/seek.txt", O_RDWR | O_CREAT).unwrap();
        vfs.write(fd, b"abcdefghij");
        let pos = vfs.lseek(fd, -3, SEEK_END).unwrap();
        assert_eq!(pos, 7);
        let data = vfs.read(fd, 100).unwrap();
        assert_eq!(&data, b"hij");
    }

    #[test]
    fn test_lseek_invalid_whence() {
        let mut vfs = new_formatted_vfs();
        let fd = vfs.open("/seek.txt", O_RDWR | O_CREAT).unwrap();
        vfs.write(fd, b"test");
        assert!(vfs.lseek(fd, 0, 99).is_none());
    }

    #[test]
    fn test_lseek_negative_position() {
        let mut vfs = new_formatted_vfs();
        let fd = vfs.open("/seek.txt", O_RDWR | O_CREAT).unwrap();
        vfs.write(fd, b"test");
        assert!(vfs.lseek(fd, -100, SEEK_SET).is_none());
    }

    #[test]
    fn test_stat_root() {
        let vfs = new_formatted_vfs();
        let inode = vfs.stat("/").unwrap();
        assert!(inode.is_directory());
        assert_eq!(inode.inode_number, ROOT_INODE);
    }

    #[test]
    fn test_stat_file() {
        let mut vfs = new_formatted_vfs();
        let fd = vfs.open("/test.txt", O_WRONLY | O_CREAT).unwrap();
        vfs.write(fd, b"hello");
        vfs.close(fd);

        let inode = vfs.stat("/test.txt").unwrap();
        assert!(inode.is_regular());
        assert_eq!(inode.size, 5);
    }

    #[test]
    fn test_stat_nonexistent() {
        let vfs = new_formatted_vfs();
        assert!(vfs.stat("/no/such/file").is_none());
    }

    #[test]
    fn test_readdir_entries() {
        let mut vfs = new_formatted_vfs();
        vfs.mkdir("/dir1");
        vfs.open("/file1.txt", O_WRONLY | O_CREAT);

        let entries = vfs.readdir("/").unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"dir1"));
        assert!(names.contains(&"file1.txt"));
    }

    #[test]
    fn test_readdir_nonexistent() {
        let vfs = new_formatted_vfs();
        assert!(vfs.readdir("/nonexistent").is_none());
    }

    #[test]
    fn test_readdir_file() {
        let mut vfs = new_formatted_vfs();
        vfs.open("/file.txt", O_WRONLY | O_CREAT);
        assert!(vfs.readdir("/file.txt").is_none());
    }

    #[test]
    fn test_unlink() {
        let mut vfs = new_formatted_vfs();
        let fd = vfs.open("/doomed.txt", O_WRONLY | O_CREAT).unwrap();
        vfs.write(fd, b"goodbye");
        vfs.close(fd);

        assert_eq!(vfs.unlink("/doomed.txt"), 0);
        assert!(vfs.stat("/doomed.txt").is_none());
    }

    #[test]
    fn test_unlink_frees_blocks() {
        let mut vfs = new_formatted_vfs();
        let free_before = vfs.superblock.as_ref().unwrap().free_blocks;

        let fd = vfs.open("/temp.txt", O_WRONLY | O_CREAT).unwrap();
        vfs.write(fd, &vec![b'x'; 1024]);
        vfs.close(fd);

        let free_after_write = vfs.superblock.as_ref().unwrap().free_blocks;
        assert!(free_after_write < free_before);

        vfs.unlink("/temp.txt");
        let free_after_unlink = vfs.superblock.as_ref().unwrap().free_blocks;
        assert!(free_after_unlink > free_after_write);
    }

    #[test]
    fn test_unlink_nonexistent() {
        let mut vfs = new_formatted_vfs();
        assert_eq!(vfs.unlink("/nonexistent.txt"), -1);
    }

    #[test]
    fn test_unlink_directory_fails() {
        let mut vfs = new_formatted_vfs();
        vfs.mkdir("/dir");
        assert_eq!(vfs.unlink("/dir"), -1);
    }

    #[test]
    fn test_resolve_root() {
        let vfs = new_formatted_vfs();
        assert_eq!(vfs.resolve_path("/"), Some(ROOT_INODE));
    }

    #[test]
    fn test_resolve_nested() {
        let mut vfs = new_formatted_vfs();
        vfs.mkdir("/a");
        vfs.mkdir("/a/b");
        vfs.mkdir("/a/b/c");
        let ino = vfs.resolve_path("/a/b/c").unwrap();
        let inode = vfs.stat("/a/b/c").unwrap();
        assert_eq!(inode.inode_number, ino);
    }

    #[test]
    fn test_resolve_nonexistent() {
        let vfs = new_formatted_vfs();
        assert!(vfs.resolve_path("/does/not/exist").is_none());
    }

    #[test]
    fn test_dup_fd() {
        let mut vfs = new_formatted_vfs();
        let fd = vfs.open("/dup.txt", O_RDWR | O_CREAT).unwrap();
        vfs.write(fd, b"abcdef");
        vfs.lseek(fd, 0, SEEK_SET);

        let dup = vfs.dup_fd(fd).unwrap();
        assert_ne!(fd, dup);

        let data = vfs.read(fd, 3).unwrap();
        assert_eq!(&data, b"abc");

        let data2 = vfs.read(dup, 3).unwrap();
        assert_eq!(&data2, b"def");
    }

    #[test]
    fn test_dup2_fd() {
        let mut vfs = new_formatted_vfs();
        let fd = vfs.open("/dup2.txt", O_RDWR | O_CREAT).unwrap();
        vfs.write(fd, b"hello");
        vfs.lseek(fd, 0, SEEK_SET);

        let result = vfs.dup2_fd(fd, 10).unwrap();
        assert_eq!(result, 10);

        let data = vfs.read(10, 100).unwrap();
        assert_eq!(&data, b"hello");
    }

    #[test]
    fn test_dup_invalid() {
        let mut vfs = new_formatted_vfs();
        assert!(vfs.dup_fd(999).is_none());
    }

    #[test]
    fn test_dup2_invalid() {
        let mut vfs = new_formatted_vfs();
        assert!(vfs.dup2_fd(999, 5).is_none());
    }

    #[test]
    fn test_close_invalid() {
        let mut vfs = new_formatted_vfs();
        assert_eq!(vfs.close(999), -1);
    }

    #[test]
    fn test_read_invalid() {
        let mut vfs = new_formatted_vfs();
        assert!(vfs.read(999, 10).is_none());
    }

    #[test]
    fn test_write_invalid() {
        let mut vfs = new_formatted_vfs();
        assert_eq!(vfs.write(999, b"data"), -1);
    }

    #[test]
    fn test_lseek_invalid() {
        let mut vfs = new_formatted_vfs();
        assert!(vfs.lseek(999, 0, SEEK_SET).is_none());
    }

    #[test]
    fn test_read_write_only() {
        let mut vfs = new_formatted_vfs();
        let fd = vfs.open("/wo.txt", O_WRONLY | O_CREAT).unwrap();
        vfs.write(fd, b"test");
        vfs.lseek(fd, 0, SEEK_SET);
        assert!(vfs.read(fd, 10).is_none());
    }

    #[test]
    fn test_write_read_only() {
        let mut vfs = new_formatted_vfs();
        let fd = vfs.open("/ro.txt", O_WRONLY | O_CREAT).unwrap();
        vfs.write(fd, b"test");
        vfs.close(fd);

        let fd = vfs.open("/ro.txt", O_RDONLY).unwrap();
        assert_eq!(vfs.write(fd, b"more"), -1);
    }

    #[test]
    fn test_read_at_eof() {
        let mut vfs = new_formatted_vfs();
        let fd = vfs.open("/eof.txt", O_RDWR | O_CREAT).unwrap();
        vfs.write(fd, b"short");
        let data = vfs.read(fd, 100).unwrap();
        assert!(data.is_empty());
    }

    #[test]
    fn test_block_exhaustion() {
        let mut vfs = new_formatted_vfs();
        let fd = vfs.open("/huge.txt", O_WRONLY | O_CREAT).unwrap();
        let chunk = vec![b'X'; BLOCK_SIZE];
        let mut total = 0;
        loop {
            let written = vfs.write(fd, &chunk);
            if (written as usize) < BLOCK_SIZE {
                break;
            }
            total += written as usize;
            if total > MAX_BLOCKS * BLOCK_SIZE {
                break;
            }
        }
        vfs.close(fd);
        // If we get here, exhaustion was handled gracefully
    }

    #[test]
    fn test_inode_exhaustion() {
        let mut vfs = new_formatted_vfs();
        let mut count = 0;
        loop {
            let fd = vfs.open(&format!("/file_{count}.txt"), O_WRONLY | O_CREAT);
            match fd {
                Some(fd) => {
                    vfs.close(fd);
                    count += 1;
                }
                None => break,
            }
            if count > MAX_INODES + 10 {
                break;
            }
        }
        assert!(count > 0);
        assert!(count <= MAX_INODES);
    }

    #[test]
    fn test_superblock_free_counts() {
        let vfs = new_formatted_vfs();
        let sb = vfs.superblock.as_ref().unwrap();
        assert_eq!(sb.free_inodes, MAX_INODES - 1);
        assert!(sb.free_blocks > 0);
    }

    #[test]
    fn test_unlink_frees_inode() {
        let mut vfs = new_formatted_vfs();
        let free_before = vfs.superblock.as_ref().unwrap().free_inodes;

        let fd = vfs.open("/temp.txt", O_WRONLY | O_CREAT).unwrap();
        vfs.close(fd);

        vfs.unlink("/temp.txt");
        assert_eq!(vfs.superblock.as_ref().unwrap().free_inodes, free_before);
    }

    #[test]
    fn test_mkdir_appears_in_parent() {
        let mut vfs = new_formatted_vfs();
        vfs.mkdir("/data");
        let entries = vfs.readdir("/").unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"data"));
    }
}
