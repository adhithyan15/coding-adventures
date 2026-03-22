# File System (Python)

A simplified inode-based file system inspired by ext2, the classic Linux file system. This is Layer 15 of the coding-adventures computing stack.

## What It Does

This package implements a Virtual File System (VFS) that turns raw block storage (a flat byte array) into the familiar world of files and directories. It provides:

- **Inodes**: fixed-size records storing file metadata (type, size, permissions, block pointers)
- **Directories**: special files containing name-to-inode mappings
- **Block allocation**: a bitmap tracking which disk blocks are free
- **Path resolution**: the algorithm for turning `/home/alice/notes.txt` into a sequence of inode lookups
- **File descriptors**: system-wide and per-process tables for managing open files
- **VFS API**: `open`, `read`, `write`, `close`, `lseek`, `stat`, `mkdir`, `readdir`, `unlink`

## Where It Fits

```
User Program
|   vfs.open("/data/log.txt", O_RDWR)
|   vfs.write(fd, b"hello")
|   vfs.close(fd)
v
VFS (this package) <-- YOU ARE HERE
|   Path Resolution, Inode Table, Block Bitmap,
|   Open File Table, Superblock
v
In-Memory Block Storage (bytearray)
```

## Usage

```python
from file_system import VFS, O_RDWR, O_CREAT, O_RDONLY, SEEK_SET

# Create and format a file system
vfs = VFS()
vfs.format()

# Create a directory
vfs.mkdir("/home")

# Create and write a file
fd = vfs.open("/home/notes.txt", O_RDWR | O_CREAT)
vfs.write(fd, b"Hello, World!")
vfs.close(fd)

# Read it back
fd = vfs.open("/home/notes.txt", O_RDONLY)
data = vfs.read(fd, 100)
print(data)  # b"Hello, World!"
vfs.close(fd)

# List directory contents
entries = vfs.readdir("/home")
for entry in entries:
    print(f"{entry.name} -> inode {entry.inode_number}")
```

## Development

```bash
uv venv
uv pip install -e ".[dev]"
python -m pytest tests/ -v
```
