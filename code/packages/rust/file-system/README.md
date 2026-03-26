# File System (Rust)

An inode-based file system inspired by ext2, implementing the Virtual File System (VFS) layer with directories, file descriptors, and block I/O.

## Where It Fits

```
User Program
    open("/data/log.txt", O_RDWR)
    write(fd, "hello", 5)
    close(fd)
        |
        v
VFS (this crate)
    Path Resolution -> Inode Table -> Block Bitmap -> Data Blocks
        |
        v
Block Device (simulated in-memory disk)
```

## Components

| Component | Description |
|-----------|-------------|
| `Superblock` | File system metadata: magic number, block/inode counts |
| `Inode` | File metadata: type, size, permissions, block pointers |
| `DirectoryEntry` | Name-to-inode mapping stored in directory data blocks |
| `BlockBitmap` | Tracks which data blocks are free/used |
| `InodeTable` | Manages the fixed array of 128 inodes |
| `OpenFile` | System-wide entry for one opening of a file |
| `OpenFileTable` | System-wide table of all open files |
| `FileDescriptorTable` | Per-process fd-to-OpenFile mapping |
| `VFS` | Main interface: format, open, read, write, mkdir, etc. |

## Usage

```rust
use file_system::VFS;

let mut vfs = VFS::new();
vfs.format(None, None);

// Create a directory and write a file
vfs.mkdir("/data");
let fd = vfs.open("/data/hello.txt", O_WRONLY | O_CREAT);
vfs.write(fd.unwrap(), b"Hello, world!");
vfs.close(fd.unwrap());

// Read it back
let fd = vfs.open("/data/hello.txt", O_RDONLY).unwrap();
let data = vfs.read(fd, 100).unwrap();
assert_eq!(&data, b"Hello, world!");
```

## Running Tests

```bash
cargo test -p file-system
```
