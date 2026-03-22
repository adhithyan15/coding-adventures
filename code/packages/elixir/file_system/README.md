# CodingAdventures.FileSystem

A simplified inode-based file system (ext2-inspired) with VFS, directories, file descriptors, and block I/O.

## Overview

This package implements a Virtual File System (VFS) that simulates a Unix-style inode-based file system entirely in memory. It demonstrates the core concepts behind real file systems like ext2: inodes, block allocation bitmaps, directory entries, path resolution, and file descriptors.

In Elixir, the VFS state is an immutable map threaded through all operations. Each operation returns `{result, new_state}`, following Elixir's functional conventions.

## Where It Fits

```
User Program
│   open("/data/log.txt", [:rdwr])
│   write(state, fd, "hello")
│   close(state, fd)
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

## Usage

```elixir
alias CodingAdventures.FileSystem

# Format a fresh file system
state = FileSystem.format()

# Create directories
{:ok, state} = FileSystem.mkdir(state, "/home")
{:ok, state} = FileSystem.mkdir(state, "/home/alice")

# Write a file
{fd, state} = FileSystem.open(state, "/home/alice/notes.txt", [:rdwr, :creat])
{_bytes, state} = FileSystem.write(state, fd, "Hello, world!")
{:ok, state} = FileSystem.close(state, fd)

# Read it back
{fd, state} = FileSystem.open(state, "/home/alice/notes.txt", [:rdonly])
{data, state} = FileSystem.read(state, fd, 100)
IO.puts(data)  # "Hello, world!"
```

## Running Tests

```bash
mix deps.get
mix test
```
