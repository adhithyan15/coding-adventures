# File System (Go)

A simplified inode-based file system inspired by ext2, the classic Linux file system. This is Layer 15 of the coding-adventures computing stack.

## What It Does

This package implements a Virtual File System (VFS) that turns raw block storage (a byte slice) into the familiar world of files and directories. It provides:

- **Inodes**: fixed-size records storing file metadata (type, size, permissions, block pointers)
- **Directories**: special files containing name-to-inode mappings
- **Block allocation**: a bitmap tracking which disk blocks are free
- **Path resolution**: turning `/home/alice/notes.txt` into a sequence of inode lookups
- **File descriptors**: system-wide and per-process tables for managing open files
- **VFS API**: `Open`, `Read`, `Write`, `Close`, `Lseek`, `Stat`, `MkDir`, `ReadDir`, `Unlink`

## Where It Fits

```
User Program
|   vfs.Open("/data/log.txt", O_RDWR)
|   vfs.Write(fd, []byte("hello"))
|   vfs.Close(fd)
v
VFS (this package) <-- YOU ARE HERE
|   Path Resolution, Inode Table, Block Bitmap,
|   Open File Table, Superblock
v
In-Memory Block Storage ([]byte)
```

## Usage

```go
import fs "github.com/adhithyan15/coding-adventures/code/packages/go/file-system"

// Create and format a file system
vfs := fs.NewDefaultVFS()
vfs.Format()

// Create a directory
vfs.MkDir("/home", 0o755)

// Create and write a file
fd := vfs.Open("/home/notes.txt", fs.O_RDWR|fs.O_CREAT)
vfs.Write(fd, []byte("Hello, World!"))
vfs.Close(fd)

// Read it back
fd = vfs.Open("/home/notes.txt", fs.O_RDONLY)
data := vfs.Read(fd, 100)
fmt.Println(string(data)) // "Hello, World!"
vfs.Close(fd)
```

## Development

```bash
go test ./... -v -cover
```
