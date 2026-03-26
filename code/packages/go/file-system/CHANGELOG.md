# Changelog

All notable changes to the file-system package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added
- `Superblock` struct with magic number validation (0x45585432 = "EXT2")
- `FileType` constants (Regular, Directory, Symlink, CharDevice, BlockDevice, Pipe, Socket)
- `Inode` struct with 12 direct block pointers and 1 indirect block pointer
- `DirectoryEntry` with serialization/deserialization and validation
- `BlockBitmap` for tracking free/used data blocks
- `InodeTable` for fixed-size inode allocation and lookup
- `OpenFile`, `OpenFileTable` for system-wide open file management
- `FileDescriptorTable` for per-process fd-to-global-fd mapping with `Clone()` for fork
- `VFS` struct with full file system API:
  - `Format()` --- initialize file system with root directory
  - `Open()` --- open files with O_RDONLY, O_WRONLY, O_RDWR, O_CREAT, O_TRUNC, O_APPEND
  - `Close()` --- close file descriptors
  - `Read()` --- read bytes from open files
  - `Write()` --- write bytes to open files, allocating blocks as needed
  - `Lseek()` --- seek with SeekSet, SeekCur, SeekEnd
  - `Stat()` --- get file/directory metadata
  - `MkDir()` --- create directories with `.` and `..` entries
  - `ReadDir()` --- list directory entries
  - `Unlink()` --- remove files, freeing inodes and blocks
  - `ResolvePath()` --- walk directory tree from root to find inodes
- Support for indirect block pointers (files larger than 6144 bytes)
- Comprehensive test suite with 80+ test cases
