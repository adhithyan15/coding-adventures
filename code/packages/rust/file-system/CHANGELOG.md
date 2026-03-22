# Changelog

All notable changes to this crate will be documented in this file.

## [0.1.0] - 2026-03-21

### Added
- Initial implementation of inode-based file system (ext2-inspired)
- Superblock with magic number validation
- Inode with direct (12) and indirect block pointers
- DirectoryEntry with binary serialization/deserialization
- BlockBitmap for free/used block tracking
- InodeTable for inode allocation and management
- OpenFile, OpenFileTable for system-wide open file tracking
- FileDescriptorTable for per-process fd mapping with dup/dup2
- VFS with full API: format, open, close, read, write, lseek, stat, mkdir, readdir, unlink
- Path resolution algorithm walking from root inode
- O_CREAT, O_TRUNC, O_APPEND flag support
- SEEK_SET, SEEK_CUR, SEEK_END seek modes
- Comprehensive test suite with 90%+ coverage
