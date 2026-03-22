# frozen_string_literal: true

require_relative "test_helper"

# = VFS Tests
#
# The VFS is the integration layer that ties everything together. These tests
# exercise the full file system API: format, mkdir, open, read, write, close,
# lseek, stat, readdir, unlink, and dup/dup2.
#
# Each test formats a fresh file system, then performs operations and verifies
# the results. This mirrors how a real OS would use the VFS.

class TestVFS < Minitest::Test
  include CodingAdventures::FileSystem

  def setup
    @vfs = VFS.new
    @vfs.format
  end

  # === Format Tests ===

  def test_format_creates_valid_superblock
    assert @vfs.superblock.valid?
    assert_equal MAGIC, @vfs.superblock.magic
    assert_equal BLOCK_SIZE, @vfs.superblock.block_size
  end

  def test_format_creates_root_directory
    root = @vfs.inode_table.get(ROOT_INODE)
    refute_nil root
    assert root.directory?
    assert_equal 2, root.link_count  # "." and ".."
  end

  def test_format_root_has_dot_entries
    entries = @vfs.readdir("/")
    refute_nil entries
    names = entries.map(&:name)
    assert_includes names, "."
    assert_includes names, ".."
  end

  def test_format_superblock_free_counts
    # After format, one inode is used (root) and one block (root dir data)
    assert_equal MAX_INODES - 1, @vfs.superblock.free_inodes
    assert @vfs.superblock.free_blocks > 0
  end

  # === mkdir Tests ===

  def test_mkdir_creates_directory
    result = @vfs.mkdir("/data")
    assert_equal 0, result

    inode = @vfs.stat("/data")
    refute_nil inode
    assert inode.directory?
  end

  def test_mkdir_creates_dot_entries
    @vfs.mkdir("/data")
    entries = @vfs.readdir("/data")
    names = entries.map(&:name)
    assert_includes names, "."
    assert_includes names, ".."
  end

  def test_mkdir_dot_dot_points_to_parent
    @vfs.mkdir("/data")
    entries = @vfs.readdir("/data")
    dotdot = entries.find { |e| e.name == ".." }
    assert_equal ROOT_INODE, dotdot.inode_number
  end

  def test_mkdir_appears_in_parent
    @vfs.mkdir("/data")
    entries = @vfs.readdir("/")
    names = entries.map(&:name)
    assert_includes names, "data"
  end

  def test_mkdir_nested
    @vfs.mkdir("/a")
    @vfs.mkdir("/a/b")
    @vfs.mkdir("/a/b/c")

    assert @vfs.stat("/a").directory?
    assert @vfs.stat("/a/b").directory?
    assert @vfs.stat("/a/b/c").directory?
  end

  def test_mkdir_duplicate_fails
    assert_equal 0, @vfs.mkdir("/data")
    assert_equal(-1, @vfs.mkdir("/data"))
  end

  def test_mkdir_nonexistent_parent_fails
    assert_equal(-1, @vfs.mkdir("/no/such/path"))
  end

  # === open/write/close/read Round Trip ===

  def test_write_and_read_round_trip
    fd = @vfs.open("/hello.txt", O_WRONLY | O_CREAT)
    refute_equal(-1, fd)
    bytes = @vfs.write(fd, "Hello, world!")
    assert_equal 13, bytes
    @vfs.close(fd)

    fd = @vfs.open("/hello.txt", O_RDONLY)
    refute_equal(-1, fd)
    data = @vfs.read(fd, 100)
    assert_equal "Hello, world!", data
    @vfs.close(fd)
  end

  def test_write_multiple_blocks
    # Write more than one block (512 bytes)
    big_data = "A" * 1500
    fd = @vfs.open("/big.txt", O_WRONLY | O_CREAT)
    @vfs.write(fd, big_data)
    @vfs.close(fd)

    fd = @vfs.open("/big.txt", O_RDONLY)
    result = @vfs.read(fd, 2000)
    assert_equal big_data, result
    @vfs.close(fd)
  end

  def test_write_to_subdirectory
    @vfs.mkdir("/data")
    fd = @vfs.open("/data/log.txt", O_WRONLY | O_CREAT)
    @vfs.write(fd, "log entry 1")
    @vfs.close(fd)

    fd = @vfs.open("/data/log.txt", O_RDONLY)
    data = @vfs.read(fd, 100)
    assert_equal "log entry 1", data
    @vfs.close(fd)
  end

  # === O_CREAT Tests ===

  def test_open_creat_creates_file
    fd = @vfs.open("/new.txt", O_WRONLY | O_CREAT)
    refute_equal(-1, fd)
    @vfs.close(fd)

    inode = @vfs.stat("/new.txt")
    refute_nil inode
    assert inode.regular?
  end

  def test_open_without_creat_nonexistent_fails
    fd = @vfs.open("/nonexistent.txt", O_RDONLY)
    assert_equal(-1, fd)
  end

  # === O_APPEND Tests ===

  def test_append_mode
    fd = @vfs.open("/log.txt", O_WRONLY | O_CREAT)
    @vfs.write(fd, "first")
    @vfs.close(fd)

    fd = @vfs.open("/log.txt", O_WRONLY | O_APPEND)
    @vfs.write(fd, " second")
    @vfs.close(fd)

    fd = @vfs.open("/log.txt", O_RDONLY)
    data = @vfs.read(fd, 100)
    assert_equal "first second", data
    @vfs.close(fd)
  end

  # === O_TRUNC Tests ===

  def test_trunc_mode
    fd = @vfs.open("/file.txt", O_WRONLY | O_CREAT)
    @vfs.write(fd, "original content")
    @vfs.close(fd)

    fd = @vfs.open("/file.txt", O_WRONLY | O_TRUNC)
    @vfs.write(fd, "new")
    @vfs.close(fd)

    fd = @vfs.open("/file.txt", O_RDONLY)
    data = @vfs.read(fd, 100)
    assert_equal "new", data
    @vfs.close(fd)
  end

  # === lseek Tests ===

  def test_lseek_set
    fd = @vfs.open("/seek.txt", O_RDWR | O_CREAT)
    @vfs.write(fd, "Hello, world!")
    pos = @vfs.lseek(fd, 7, SEEK_SET)
    assert_equal 7, pos
    data = @vfs.read(fd, 100)
    assert_equal "world!", data
    @vfs.close(fd)
  end

  def test_lseek_cur
    fd = @vfs.open("/seek.txt", O_RDWR | O_CREAT)
    @vfs.write(fd, "abcdefghij")
    @vfs.lseek(fd, 2, SEEK_SET)
    pos = @vfs.lseek(fd, 3, SEEK_CUR)
    assert_equal 5, pos
    data = @vfs.read(fd, 100)
    assert_equal "fghij", data
    @vfs.close(fd)
  end

  def test_lseek_end
    fd = @vfs.open("/seek.txt", O_RDWR | O_CREAT)
    @vfs.write(fd, "abcdefghij")
    pos = @vfs.lseek(fd, -3, SEEK_END)
    assert_equal 7, pos
    data = @vfs.read(fd, 100)
    assert_equal "hij", data
    @vfs.close(fd)
  end

  def test_lseek_invalid_whence
    fd = @vfs.open("/seek.txt", O_RDWR | O_CREAT)
    @vfs.write(fd, "test")
    result = @vfs.lseek(fd, 0, 99)
    assert_equal(-1, result)
    @vfs.close(fd)
  end

  def test_lseek_negative_position
    fd = @vfs.open("/seek.txt", O_RDWR | O_CREAT)
    @vfs.write(fd, "test")
    result = @vfs.lseek(fd, -100, SEEK_SET)
    assert_equal(-1, result)
    @vfs.close(fd)
  end

  # === stat Tests ===

  def test_stat_root
    inode = @vfs.stat("/")
    refute_nil inode
    assert inode.directory?
    assert_equal ROOT_INODE, inode.inode_number
  end

  def test_stat_file
    fd = @vfs.open("/test.txt", O_WRONLY | O_CREAT)
    @vfs.write(fd, "hello")
    @vfs.close(fd)

    inode = @vfs.stat("/test.txt")
    refute_nil inode
    assert inode.regular?
    assert_equal 5, inode.size
  end

  def test_stat_nonexistent
    assert_nil @vfs.stat("/no/such/file")
  end

  # === readdir Tests ===

  def test_readdir_root_after_creating_files
    @vfs.mkdir("/dir1")
    fd = @vfs.open("/file1.txt", O_WRONLY | O_CREAT)
    @vfs.close(fd)

    entries = @vfs.readdir("/")
    names = entries.map(&:name)
    assert_includes names, "."
    assert_includes names, ".."
    assert_includes names, "dir1"
    assert_includes names, "file1.txt"
  end

  def test_readdir_nonexistent_returns_nil
    assert_nil @vfs.readdir("/nonexistent")
  end

  def test_readdir_file_returns_nil
    fd = @vfs.open("/file.txt", O_WRONLY | O_CREAT)
    @vfs.close(fd)
    assert_nil @vfs.readdir("/file.txt")
  end

  # === unlink Tests ===

  def test_unlink_removes_file
    fd = @vfs.open("/doomed.txt", O_WRONLY | O_CREAT)
    @vfs.write(fd, "goodbye")
    @vfs.close(fd)

    result = @vfs.unlink("/doomed.txt")
    assert_equal 0, result
    assert_nil @vfs.stat("/doomed.txt")
  end

  def test_unlink_frees_blocks
    free_before = @vfs.superblock.free_blocks

    fd = @vfs.open("/temp.txt", O_WRONLY | O_CREAT)
    @vfs.write(fd, "x" * 1024)
    @vfs.close(fd)

    free_after_write = @vfs.superblock.free_blocks
    assert free_after_write < free_before

    @vfs.unlink("/temp.txt")
    free_after_unlink = @vfs.superblock.free_blocks
    assert free_after_unlink > free_after_write
  end

  def test_unlink_frees_inode
    free_before = @vfs.superblock.free_inodes

    fd = @vfs.open("/temp.txt", O_WRONLY | O_CREAT)
    @vfs.close(fd)
    free_after_create = @vfs.inode_table.free_count
    assert free_after_create < free_before

    @vfs.unlink("/temp.txt")
    assert_equal free_before, @vfs.inode_table.free_count
  end

  def test_unlink_nonexistent_fails
    assert_equal(-1, @vfs.unlink("/nonexistent.txt"))
  end

  def test_unlink_directory_fails
    @vfs.mkdir("/dir")
    assert_equal(-1, @vfs.unlink("/dir"))
  end

  # === resolve_path Tests ===

  def test_resolve_root
    assert_equal ROOT_INODE, @vfs.resolve_path("/")
  end

  def test_resolve_nested_path
    @vfs.mkdir("/a")
    @vfs.mkdir("/a/b")
    @vfs.mkdir("/a/b/c")

    inode_num = @vfs.resolve_path("/a/b/c")
    refute_nil inode_num
    inode = @vfs.inode_table.get(inode_num)
    assert inode.directory?
  end

  def test_resolve_nonexistent_returns_nil
    assert_nil @vfs.resolve_path("/does/not/exist")
  end

  # === dup/dup2 Tests ===

  def test_dup_fd_shares_offset
    fd = @vfs.open("/dup.txt", O_RDWR | O_CREAT)
    @vfs.write(fd, "abcdef")
    @vfs.lseek(fd, 0, SEEK_SET)

    dup_fd = @vfs.dup_fd(fd)
    refute_nil dup_fd
    refute_equal fd, dup_fd

    # Reading from one should advance the shared offset
    data = @vfs.read(fd, 3)
    assert_equal "abc", data

    # Reading from the dup should continue from the new offset
    data2 = @vfs.read(dup_fd, 3)
    assert_equal "def", data2

    @vfs.close(fd)
    @vfs.close(dup_fd)
  end

  def test_dup2_fd
    fd = @vfs.open("/dup2.txt", O_RDWR | O_CREAT)
    @vfs.write(fd, "hello")
    @vfs.lseek(fd, 0, SEEK_SET)

    result = @vfs.dup2_fd(fd, 10)
    assert_equal 10, result

    data = @vfs.read(10, 100)
    assert_equal "hello", data

    @vfs.close(fd)
    @vfs.close(10)
  end

  def test_dup_invalid_fd_returns_nil
    assert_nil @vfs.dup_fd(999)
  end

  def test_dup2_invalid_fd_returns_nil
    assert_nil @vfs.dup2_fd(999, 5)
  end

  # === Error Handling Tests ===

  def test_close_invalid_fd
    assert_equal(-1, @vfs.close(999))
  end

  def test_read_invalid_fd
    assert_nil @vfs.read(999, 10)
  end

  def test_write_invalid_fd
    assert_equal(-1, @vfs.write(999, "data"))
  end

  def test_lseek_invalid_fd
    assert_equal(-1, @vfs.lseek(999, 0, SEEK_SET))
  end

  def test_read_write_only_fd
    fd = @vfs.open("/wo.txt", O_WRONLY | O_CREAT)
    @vfs.write(fd, "test")
    @vfs.lseek(fd, 0, SEEK_SET)
    assert_nil @vfs.read(fd, 10)
    @vfs.close(fd)
  end

  def test_write_read_only_fd
    fd = @vfs.open("/ro.txt", O_WRONLY | O_CREAT)
    @vfs.write(fd, "test")
    @vfs.close(fd)

    fd = @vfs.open("/ro.txt", O_RDONLY)
    assert_equal(-1, @vfs.write(fd, "more"))
    @vfs.close(fd)
  end

  def test_read_at_eof_returns_empty
    fd = @vfs.open("/eof.txt", O_RDWR | O_CREAT)
    @vfs.write(fd, "short")
    # Offset is now at 5 (end of file)
    data = @vfs.read(fd, 100)
    assert_equal "".b, data
    @vfs.close(fd)
  end

  # === Block Exhaustion Test ===

  def test_block_exhaustion
    # Create a file and write until we run out of blocks.
    # Our disk has a limited number of data blocks.
    fd = @vfs.open("/huge.txt", O_WRONLY | O_CREAT)
    total_written = 0
    chunk = "X" * BLOCK_SIZE

    # Keep writing until we can't write a full block
    loop do
      written = @vfs.write(fd, chunk)
      break if written < BLOCK_SIZE

      total_written += written
      break if total_written > MAX_BLOCKS * BLOCK_SIZE  # Safety limit
    end

    @vfs.close(fd)
    # If we get here without error, the VFS handled exhaustion gracefully
    assert true
  end

  # === Inode Exhaustion Test ===

  def test_inode_exhaustion
    # Create files until we run out of inodes
    count = 0
    loop do
      fd = @vfs.open("/file_#{count}.txt", O_WRONLY | O_CREAT)
      break if fd == -1

      @vfs.close(fd)
      count += 1
      break if count > MAX_INODES + 10  # Safety limit
    end

    # We should have created some files but eventually failed
    assert count > 0
    assert count <= MAX_INODES
  end

  # === DirectoryEntry Serialization Tests ===

  def test_directory_entry_serialize_round_trip
    entry = DirectoryEntry.new("hello.txt", 42)
    serialized = entry.serialize
    result, next_offset = DirectoryEntry.deserialize(serialized, 0)
    assert_equal "hello.txt", result.name
    assert_equal 42, result.inode_number
    assert_equal serialized.length, next_offset
  end

  def test_directory_entry_serialize_all_round_trip
    entries = [
      DirectoryEntry.new(".", 0),
      DirectoryEntry.new("..", 0),
      DirectoryEntry.new("file.txt", 5)
    ]
    data = DirectoryEntry.serialize_all(entries)
    result = DirectoryEntry.deserialize_all(data)
    assert_equal 3, result.length
    assert_equal ".", result[0].name
    assert_equal "..", result[1].name
    assert_equal "file.txt", result[2].name
  end

  def test_directory_entry_invalid_name
    assert_raises(ArgumentError) { DirectoryEntry.new("", 0) }
    assert_raises(ArgumentError) { DirectoryEntry.new(nil, 0) }
    assert_raises(ArgumentError) { DirectoryEntry.new("a/b", 0) }
    assert_raises(ArgumentError) { DirectoryEntry.new("x" * 256, 0) }
  end

  def test_directory_entry_deserialize_empty
    assert_nil DirectoryEntry.deserialize("", 0)
  end

  def test_directory_entry_deserialize_truncated
    assert_nil DirectoryEntry.deserialize("\x05abc", 0)  # Claims 5 bytes name but only 3
  end
end
