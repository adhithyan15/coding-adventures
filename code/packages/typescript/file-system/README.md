# @coding-adventures/file-system

A simplified inode-based file system (ext2-inspired) with VFS, directories, file descriptors, and block I/O.

## Overview

This package implements a Virtual File System (VFS) that simulates a Unix-style inode-based file system entirely in memory. It demonstrates the core concepts behind real file systems like ext2: inodes, block allocation bitmaps, directory entries, path resolution, and file descriptors.

## Where It Fits

```
User Program
│   open("/data/log.txt", O_RDWR)
│   write(fd, "hello", 5)
│   close(fd)
▼
OS Kernel — Syscall Dispatcher
▼
Virtual File System (VFS) ← THIS PACKAGE
│   ├── Path Resolution
│   ├── Inode Table
│   ├── Block Bitmap
│   ├── Open File Table
│   ├── FD Table
│   └── Superblock
▼
Block Device (simulated in-memory disk)
```

## Key Types

- **Superblock**: File system metadata (magic number, sizes, free counts)
- **Inode**: File metadata (type, size, permissions, block pointers)
- **DirectoryEntry**: Name-to-inode mapping
- **BlockBitmap**: Tracks free/used data blocks
- **InodeTable**: Manages all inodes
- **OpenFileTable**: System-wide table of open files
- **FileDescriptorTable**: Per-process fd-to-open-file mapping
- **VFS**: The central orchestrator tying everything together

## Usage

```typescript
import { VFS, O_RDWR, O_CREAT, SEEK_SET } from "@coding-adventures/file-system";

const vfs = new VFS();
vfs.format();

// Create directories
vfs.mkdir("/home");
vfs.mkdir("/home/alice");

// Write a file
const fd = vfs.open("/home/alice/notes.txt", O_RDWR | O_CREAT)!;
vfs.write(fd, new TextEncoder().encode("Hello, world!"));
vfs.close(fd);

// Read it back
const fd2 = vfs.open("/home/alice/notes.txt", O_RDWR)!;
const data = vfs.read(fd2, 100)!;
console.log(new TextDecoder().decode(data)); // "Hello, world!"
vfs.close(fd2);
```

## Running Tests

```bash
npm install
npx vitest run --coverage
```
