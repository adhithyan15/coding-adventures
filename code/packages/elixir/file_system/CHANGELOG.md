# Changelog

## 0.1.0 — 2026-03-21

### Added

- Initial implementation of inode-based file system (ext2-inspired)
- `FileSystem.format/0` to initialize a blank file system
- `FileSystem.open/4`, `close/3`, `read/4`, `write/4`, `lseek/5` for file I/O
- `FileSystem.stat/2`, `mkdir/2`, `readdir/2`, `unlink/2` for file management
- `FileSystem.dup/3` and `dup2/4` for file descriptor duplication
- `FileSystem.resolve_path/2` for path resolution from root inode
- Immutable state threading (functional style, `{result, new_state}` return pattern)
- Support for direct blocks (12 slots) and single indirect block
- Open flags: `:rdonly`, `:wronly`, `:rdwr`, `:creat`, `:trunc`, `:append`
- Seek modes: `:set`, `:cur`, `:seek_end` (avoiding reserved word `end`)
- File types: `:regular`, `:directory`, `:symlink`, `:char_device`, `:block_device`, `:pipe`, `:socket`
- Directory support with "." and ".." entries
- Multi-process support with independent fd tables per PID
- Comprehensive test suite with >80% coverage
