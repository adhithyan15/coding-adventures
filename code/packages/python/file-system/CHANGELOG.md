# Changelog

All notable changes to the file-system package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added
- `Superblock` dataclass with magic number validation (0x45585432 = "EXT2")
- `FileType` enum (REGULAR, DIRECTORY, SYMLINK, CHAR_DEVICE, BLOCK_DEVICE, PIPE, SOCKET)
- `Inode` dataclass with 12 direct block pointers and 1 indirect block pointer
- `DirectoryEntry` dataclass with serialization/deserialization
- `BlockBitmap` for tracking free/used data blocks
- `InodeTable` for fixed-size inode allocation and lookup
- `OpenFile`, `OpenFileTable` for system-wide open file management
- `FileDescriptorTable` for per-process fd-to-global-fd mapping with `clone()` for fork
- `VFS` class with full file system API:
  - `format()` --- initialize file system with root directory
  - `open()` --- open files with O_RDONLY, O_WRONLY, O_RDWR, O_CREAT, O_TRUNC, O_APPEND
  - `close()` --- close file descriptors
  - `read()` --- read bytes from open files
  - `write()` --- write bytes to open files, allocating blocks as needed
  - `lseek()` --- seek with SEEK_SET, SEEK_CUR, SEEK_END
  - `stat()` --- get file/directory metadata
  - `mkdir()` --- create directories with `.` and `..` entries
  - `readdir()` --- list directory entries
  - `unlink()` --- remove files, freeing inodes and blocks
  - `resolve_path()` --- walk directory tree from root to find inodes
- Support for indirect block pointers (files larger than 6144 bytes)
- Comprehensive test suite with 80+ test cases
