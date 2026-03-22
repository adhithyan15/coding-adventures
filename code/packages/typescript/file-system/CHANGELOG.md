# Changelog

## 0.1.0 — 2026-03-21

### Added

- Initial implementation of inode-based file system (ext2-inspired)
- `VFS` class with format, open, close, read, write, lseek, stat, mkdir, readdir, unlink, dup, dup2
- `BlockBitmap` for tracking free/used data blocks
- `InodeTable` for managing inodes (allocate, free, get)
- `OpenFileTable` for system-wide open file tracking with ref counting
- `FileDescriptorTable` for per-process fd-to-global mapping with clone support
- Support for direct blocks (12 slots, up to 6,144 bytes) and single indirect block (up to 71,680 bytes)
- Open flags: O_RDONLY, O_WRONLY, O_RDWR, O_CREAT, O_TRUNC, O_APPEND
- Seek modes: SEEK_SET, SEEK_CUR, SEEK_END
- File types: REGULAR, DIRECTORY, SYMLINK, CHAR_DEVICE, BLOCK_DEVICE, PIPE, SOCKET
- Directory support with "." and ".." entries
- Path resolution from root inode
- Multi-process support with independent fd tables
- Comprehensive test suite with >80% coverage
