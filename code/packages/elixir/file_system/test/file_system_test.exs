defmodule CodingAdventures.FileSystemTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.FileSystem
  alias CodingAdventures.FileSystem.DirectoryEntry

  # ============================================================================
  # Helper
  # ============================================================================

  defp fresh_fs, do: FileSystem.format()

  # ============================================================================
  # Format and Superblock Tests
  # ============================================================================

  describe "format/0" do
    test "initializes superblock with correct magic number" do
      state = fresh_fs()
      sb = FileSystem.get_superblock(state)
      assert sb.magic == 0x45585432
    end

    test "initializes superblock with correct sizes" do
      state = fresh_fs()
      sb = FileSystem.get_superblock(state)
      assert sb.block_size == FileSystem.block_size()
      assert sb.total_blocks == FileSystem.max_blocks()
      assert sb.total_inodes == FileSystem.max_inodes()
    end

    test "has correct free counts after format" do
      state = fresh_fs()
      sb = FileSystem.get_superblock(state)
      assert sb.free_inodes == FileSystem.max_inodes() - 1
    end

    test "creates root directory at inode 0" do
      state = fresh_fs()
      root_stat = FileSystem.stat(state, "/")
      assert root_stat != nil
      assert root_stat.inode_number == FileSystem.root_inode()
      assert root_stat.file_type == :directory
    end

    test "root directory has . and .. entries" do
      state = fresh_fs()
      entries = FileSystem.readdir(state, "/")
      assert entries != nil
      names = Enum.map(entries, & &1.name)
      assert "." in names
      assert ".." in names

      dot = Enum.find(entries, fn entry -> entry.name == "." end)
      dotdot = Enum.find(entries, fn entry -> entry.name == ".." end)
      assert dot.inode_number == FileSystem.root_inode()
      assert dotdot.inode_number == FileSystem.root_inode()
    end
  end

  # ============================================================================
  # Directory Entry Serialization
  # ============================================================================

  describe "serialize/deserialize entries" do
    test "round-trips correctly" do
      entries = [
        %DirectoryEntry{name: ".", inode_number: 0},
        %DirectoryEntry{name: "..", inode_number: 0},
        %DirectoryEntry{name: "hello", inode_number: 5}
      ]

      serialized = FileSystem.serialize_entries(entries)
      deserialized = FileSystem.deserialize_entries(serialized)

      assert length(deserialized) == 3
      assert Enum.at(deserialized, 0).name == "."
      assert Enum.at(deserialized, 2).inode_number == 5
    end

    test "handles empty list" do
      assert FileSystem.deserialize_entries("") == []
    end
  end

  # ============================================================================
  # mkdir and readdir
  # ============================================================================

  describe "mkdir/2" do
    test "creates a directory with . and .. entries" do
      state = fresh_fs()
      {:ok, state} = FileSystem.mkdir(state, "/home")
      entries = FileSystem.readdir(state, "/home")
      assert entries != nil
      names = Enum.map(entries, & &1.name)
      assert "." in names
      assert ".." in names
    end

    test "adds entry in parent directory" do
      state = fresh_fs()
      {:ok, state} = FileSystem.mkdir(state, "/home")
      entries = FileSystem.readdir(state, "/")
      names = Enum.map(entries, & &1.name)
      assert "home" in names
    end

    test "sets .. to point to parent inode" do
      state = fresh_fs()
      {:ok, state} = FileSystem.mkdir(state, "/home")
      entries = FileSystem.readdir(state, "/home")
      dotdot = Enum.find(entries, fn entry -> entry.name == ".." end)
      assert dotdot.inode_number == FileSystem.root_inode()
    end

    test "creates nested directories" do
      state = fresh_fs()
      {:ok, state} = FileSystem.mkdir(state, "/a")
      {:ok, state} = FileSystem.mkdir(state, "/a/b")
      {:ok, state} = FileSystem.mkdir(state, "/a/b/c")

      stat_result = FileSystem.stat(state, "/a/b/c")
      assert stat_result != nil
      assert stat_result.file_type == :directory

      entries = FileSystem.readdir(state, "/a/b/c")
      names = Enum.map(entries, & &1.name)
      assert "." in names
      assert ".." in names
    end

    test "fails when parent does not exist" do
      state = fresh_fs()
      {:error, _state} = FileSystem.mkdir(state, "/nonexistent/child")
    end

    test "fails when name already exists" do
      state = fresh_fs()
      {:ok, state} = FileSystem.mkdir(state, "/home")
      {:error, _state} = FileSystem.mkdir(state, "/home")
    end

    test "returns nil for readdir on non-existent path" do
      state = fresh_fs()
      assert FileSystem.readdir(state, "/no/such/path") == nil
    end
  end

  # ============================================================================
  # Path Resolution
  # ============================================================================

  describe "resolve_path/2" do
    test "resolves root path" do
      state = fresh_fs()
      assert FileSystem.resolve_path(state, "/") == FileSystem.root_inode()
    end

    test "resolves a directory" do
      state = fresh_fs()
      {:ok, state} = FileSystem.mkdir(state, "/home")
      inode_num = FileSystem.resolve_path(state, "/home")
      assert inode_num != nil
      assert inode_num > FileSystem.root_inode()
    end

    test "resolves nested paths" do
      state = fresh_fs()
      {:ok, state} = FileSystem.mkdir(state, "/a")
      {:ok, state} = FileSystem.mkdir(state, "/a/b")
      {:ok, state} = FileSystem.mkdir(state, "/a/b/c")
      assert FileSystem.resolve_path(state, "/a/b/c") != nil
    end

    test "returns nil for non-existent paths" do
      state = fresh_fs()
      assert FileSystem.resolve_path(state, "/does/not/exist") == nil
    end

    test "returns nil when trying to traverse a file" do
      state = fresh_fs()
      {_fd, state} = FileSystem.open(state, "/file.txt", [:wronly, :creat])
      assert FileSystem.resolve_path(state, "/file.txt/child") == nil
    end
  end

  # ============================================================================
  # File I/O
  # ============================================================================

  describe "file I/O" do
    test "creates a file with :creat flag" do
      state = fresh_fs()
      {fd, _state} = FileSystem.open(state, "/test.txt", [:wronly, :creat])
      assert fd != nil
      assert fd >= 3
    end

    test "write and read round trip" do
      state = fresh_fs()
      {fd, state} = FileSystem.open(state, "/test.txt", [:rdwr, :creat])
      {_bytes, state} = FileSystem.write(state, fd, "Hello, file system!")
      {_pos, state} = FileSystem.lseek(state, fd, 0, :set)
      {data, _state} = FileSystem.read(state, fd, 100)
      assert data == "Hello, file system!"
    end

    test "handles multiple writes" do
      state = fresh_fs()
      {fd, state} = FileSystem.open(state, "/test.txt", [:rdwr, :creat])
      {_bytes, state} = FileSystem.write(state, fd, "Hello")
      {_bytes, state} = FileSystem.write(state, fd, " World")
      {_pos, state} = FileSystem.lseek(state, fd, 0, :set)
      {data, _state} = FileSystem.read(state, fd, 100)
      assert data == "Hello World"
    end

    test "reads only available bytes when count exceeds file size" do
      state = fresh_fs()
      {fd, state} = FileSystem.open(state, "/test.txt", [:rdwr, :creat])
      {_bytes, state} = FileSystem.write(state, fd, "abc")
      {_pos, state} = FileSystem.lseek(state, fd, 0, :set)
      {data, _state} = FileSystem.read(state, fd, 1000)
      assert byte_size(data) == 3
      assert data == "abc"
    end

    test "returns empty binary when reading at end of file" do
      state = fresh_fs()
      {fd, state} = FileSystem.open(state, "/test.txt", [:rdwr, :creat])
      {_bytes, state} = FileSystem.write(state, fd, "data")
      {data, _state} = FileSystem.read(state, fd, 10)
      assert data == ""
    end

    test "fails to read a write-only file" do
      state = fresh_fs()
      {fd, state} = FileSystem.open(state, "/test.txt", [:wronly, :creat])
      {_bytes, state} = FileSystem.write(state, fd, "data")
      {_pos, state} = FileSystem.lseek(state, fd, 0, :set)
      {result, _state} = FileSystem.read(state, fd, 10)
      assert result == nil
    end

    test "fails to write a read-only file" do
      state = fresh_fs()
      {fd1, state} = FileSystem.open(state, "/test.txt", [:wronly, :creat])
      {_bytes, state} = FileSystem.write(state, fd1, "data")
      {:ok, state} = FileSystem.close(state, fd1)
      {fd2, state} = FileSystem.open(state, "/test.txt", [:rdonly])
      {result, _state} = FileSystem.write(state, fd2, "more")
      assert result == nil
    end

    test "returns nil when opening non-existent file without :creat" do
      state = fresh_fs()
      {fd, _state} = FileSystem.open(state, "/no-such-file.txt", [:rdonly])
      assert fd == nil
    end

    test "returns error when closing an invalid fd" do
      state = fresh_fs()
      {:error, _state} = FileSystem.close(state, 999)
    end

    test "handles :trunc flag" do
      state = fresh_fs()
      {fd1, state} = FileSystem.open(state, "/test.txt", [:wronly, :creat])
      {_bytes, state} = FileSystem.write(state, fd1, "Hello World")
      {:ok, state} = FileSystem.close(state, fd1)

      {fd2, state} = FileSystem.open(state, "/test.txt", [:rdwr, :trunc])
      {_bytes, state} = FileSystem.write(state, fd2, "Hi")
      {_pos, state} = FileSystem.lseek(state, fd2, 0, :set)
      {data, _state} = FileSystem.read(state, fd2, 100)
      assert data == "Hi"
    end

    test "handles :append flag" do
      state = fresh_fs()
      {fd1, state} = FileSystem.open(state, "/test.txt", [:wronly, :creat])
      {_bytes, state} = FileSystem.write(state, fd1, "Hello")
      {:ok, state} = FileSystem.close(state, fd1)

      {fd2, state} = FileSystem.open(state, "/test.txt", [:rdwr, :append])
      {_bytes, state} = FileSystem.write(state, fd2, " World")
      {_pos, state} = FileSystem.lseek(state, fd2, 0, :set)
      {data, _state} = FileSystem.read(state, fd2, 100)
      assert data == "Hello World"
    end

    test "handles writes spanning multiple blocks" do
      state = fresh_fs()
      {fd, state} = FileSystem.open(state, "/big.txt", [:rdwr, :creat])
      data = :binary.copy(<<65>>, 1500)
      {_bytes, state} = FileSystem.write(state, fd, data)
      {_pos, state} = FileSystem.lseek(state, fd, 0, :set)
      {result, _state} = FileSystem.read(state, fd, 2000)
      assert byte_size(result) == 1500
      assert result == data
    end

    test "handles writes that use indirect blocks" do
      state = fresh_fs()
      {fd, state} = FileSystem.open(state, "/large.txt", [:rdwr, :creat])
      # Direct blocks cover 12 × 512 = 6144 bytes. Write 7000 to force indirect.
      data = :binary.copy(<<42>>, 7000)
      {_bytes, state} = FileSystem.write(state, fd, data)
      {_pos, state} = FileSystem.lseek(state, fd, 0, :set)
      {result, _state} = FileSystem.read(state, fd, 8000)
      assert byte_size(result) == 7000
      assert result == data
    end

    test "handles file in a subdirectory" do
      state = fresh_fs()
      {:ok, state} = FileSystem.mkdir(state, "/data")
      {fd, state} = FileSystem.open(state, "/data/log.txt", [:rdwr, :creat])
      {_bytes, state} = FileSystem.write(state, fd, "log entry 1")
      {_pos, state} = FileSystem.lseek(state, fd, 0, :set)
      {data, _state} = FileSystem.read(state, fd, 100)
      assert data == "log entry 1"
    end
  end

  # ============================================================================
  # lseek
  # ============================================================================

  describe "lseek/5" do
    test "seek with :set (absolute)" do
      state = fresh_fs()
      {fd, state} = FileSystem.open(state, "/test.txt", [:rdwr, :creat])
      {_bytes, state} = FileSystem.write(state, fd, "Hello World")
      {pos, state} = FileSystem.lseek(state, fd, 5, :set)
      assert pos == 5
      {data, _state} = FileSystem.read(state, fd, 10)
      assert data == " World"
    end

    test "seek with :cur (relative)" do
      state = fresh_fs()
      {fd, state} = FileSystem.open(state, "/test.txt", [:rdwr, :creat])
      {_bytes, state} = FileSystem.write(state, fd, "Hello World")
      {_pos, state} = FileSystem.lseek(state, fd, 0, :set)
      {_data, state} = FileSystem.read(state, fd, 5)
      {pos, state} = FileSystem.lseek(state, fd, 1, :cur)
      assert pos == 6
      {data, _state} = FileSystem.read(state, fd, 10)
      assert data == "World"
    end

    test "seek with :seek_end (from end)" do
      state = fresh_fs()
      {fd, state} = FileSystem.open(state, "/test.txt", [:rdwr, :creat])
      {_bytes, state} = FileSystem.write(state, fd, "Hello World")
      {pos, state} = FileSystem.lseek(state, fd, -5, :seek_end)
      assert pos == 6
      {data, _state} = FileSystem.read(state, fd, 10)
      assert data == "World"
    end

    test "returns nil for negative resulting offset" do
      state = fresh_fs()
      {fd, state} = FileSystem.open(state, "/test.txt", [:rdwr, :creat])
      {_bytes, state} = FileSystem.write(state, fd, "abc")
      {result, _state} = FileSystem.lseek(state, fd, -100, :set)
      assert result == nil
    end

    test "returns nil for invalid whence" do
      state = fresh_fs()
      {fd, state} = FileSystem.open(state, "/test.txt", [:rdwr, :creat])
      {result, _state} = FileSystem.lseek(state, fd, 0, :invalid_whence)
      assert result == nil
    end

    test "returns nil for invalid fd" do
      state = fresh_fs()
      {result, _state} = FileSystem.lseek(state, 999, 0, :set)
      assert result == nil
    end
  end

  # ============================================================================
  # stat
  # ============================================================================

  describe "stat/2" do
    test "returns metadata for root" do
      state = fresh_fs()
      s = FileSystem.stat(state, "/")
      assert s.file_type == :directory
      assert s.inode_number == FileSystem.root_inode()
      assert s.link_count == 2
    end

    test "returns metadata for a file" do
      state = fresh_fs()
      {fd, state} = FileSystem.open(state, "/test.txt", [:wronly, :creat])
      {_bytes, state} = FileSystem.write(state, fd, "12345")
      {:ok, state} = FileSystem.close(state, fd)

      s = FileSystem.stat(state, "/test.txt")
      assert s.file_type == :regular
      assert s.size == 5
      assert s.permissions == 0o755
    end

    test "returns metadata for a directory" do
      state = fresh_fs()
      {:ok, state} = FileSystem.mkdir(state, "/mydir")
      s = FileSystem.stat(state, "/mydir")
      assert s.file_type == :directory
      assert s.link_count == 2
    end

    test "returns nil for non-existent path" do
      state = fresh_fs()
      assert FileSystem.stat(state, "/nope") == nil
    end

    test "updates link_count when adding subdirectories" do
      state = fresh_fs()
      {:ok, state} = FileSystem.mkdir(state, "/parent")
      {:ok, state} = FileSystem.mkdir(state, "/parent/child")
      s = FileSystem.stat(state, "/parent")
      # parent has 3 links: "parent" in root, "." in itself, ".." in child
      assert s.link_count == 3
    end
  end

  # ============================================================================
  # unlink
  # ============================================================================

  describe "unlink/2" do
    test "removes a file" do
      state = fresh_fs()
      {fd, state} = FileSystem.open(state, "/test.txt", [:wronly, :creat])
      {_bytes, state} = FileSystem.write(state, fd, "data")
      {:ok, state} = FileSystem.close(state, fd)

      {:ok, state} = FileSystem.unlink(state, "/test.txt")
      assert FileSystem.stat(state, "/test.txt") == nil
    end

    test "frees inode when link_count reaches 0" do
      state = fresh_fs()
      {fd, state} = FileSystem.open(state, "/test.txt", [:wronly, :creat])
      {_bytes, state} = FileSystem.write(state, fd, "some data")
      {:ok, state} = FileSystem.close(state, fd)

      free_before = FileSystem.get_superblock(state).free_inodes
      {:ok, state} = FileSystem.unlink(state, "/test.txt")
      free_after = FileSystem.get_superblock(state).free_inodes
      assert free_after == free_before + 1
    end

    test "does not unlink directories" do
      state = fresh_fs()
      {:ok, state} = FileSystem.mkdir(state, "/mydir")
      {:error, _state} = FileSystem.unlink(state, "/mydir")
    end

    test "fails for non-existent file" do
      state = fresh_fs()
      {:error, _state} = FileSystem.unlink(state, "/no-such-file")
    end

    test "removes entry from parent directory" do
      state = fresh_fs()
      {_fd, state} = FileSystem.open(state, "/a.txt", [:wronly, :creat])
      {_fd, state} = FileSystem.open(state, "/b.txt", [:wronly, :creat])
      {:ok, state} = FileSystem.unlink(state, "/a.txt")
      entries = FileSystem.readdir(state, "/")
      names = Enum.map(entries, & &1.name)
      refute "a.txt" in names
      assert "b.txt" in names
    end
  end

  # ============================================================================
  # dup / dup2
  # ============================================================================

  describe "dup/dup2" do
    test "duplicates a file descriptor" do
      state = fresh_fs()
      {fd1, state} = FileSystem.open(state, "/test.txt", [:rdwr, :creat])
      {fd2, state} = FileSystem.dup(state, fd1)
      assert fd2 != fd1
      assert fd2 > fd1

      # Both fds share the same offset
      {_bytes, state} = FileSystem.write(state, fd1, "Hello")
      {_bytes, state} = FileSystem.write(state, fd2, " World")

      {_pos, state} = FileSystem.lseek(state, fd1, 0, :set)
      {data, _state} = FileSystem.read(state, fd1, 100)
      assert data == "Hello World"
    end

    test "dup2 to a specific fd number" do
      state = fresh_fs()
      {fd, state} = FileSystem.open(state, "/test.txt", [:rdwr, :creat])
      {result, state} = FileSystem.dup2(state, fd, 10)
      assert result == 10

      {_bytes, state} = FileSystem.write(state, 10, "via dup2")
      {_pos, state} = FileSystem.lseek(state, fd, 0, :set)
      {data, _state} = FileSystem.read(state, fd, 100)
      assert data == "via dup2"
    end

    test "returns nil for dup of invalid fd" do
      state = fresh_fs()
      {result, _state} = FileSystem.dup(state, 999)
      assert result == nil
    end

    test "returns nil for dup2 with invalid old fd" do
      state = fresh_fs()
      {result, _state} = FileSystem.dup2(state, 999, 5)
      assert result == nil
    end
  end

  # ============================================================================
  # Multi-Process
  # ============================================================================

  describe "multi-process" do
    test "maintains independent fd tables per process" do
      state = fresh_fs()
      {fd1, state} = FileSystem.open(state, "/file1.txt", [:wronly, :creat], 1)
      {fd2, state} = FileSystem.open(state, "/file2.txt", [:wronly, :creat], 2)

      assert fd1 == 3
      assert fd2 == 3

      {_bytes, state} = FileSystem.write(state, fd1, "from pid 1", 1)
      {_bytes, state} = FileSystem.write(state, fd2, "from pid 2", 2)
      {:ok, state} = FileSystem.close(state, fd1, 1)
      {:ok, state} = FileSystem.close(state, fd2, 2)

      {r1, state} = FileSystem.open(state, "/file1.txt", [:rdonly], 1)
      {r2, state} = FileSystem.open(state, "/file2.txt", [:rdonly], 2)
      {d1, _state} = FileSystem.read(state, r1, 100, 1)
      {d2, _state} = FileSystem.read(state, r2, 100, 2)
      assert d1 == "from pid 1"
      assert d2 == "from pid 2"
    end
  end

  # ============================================================================
  # Edge Cases
  # ============================================================================

  describe "edge cases" do
    test "handles empty path components gracefully" do
      state = fresh_fs()
      {:ok, state} = FileSystem.mkdir(state, "/data")
      assert FileSystem.resolve_path(state, "/data/") != nil
    end

    test "read returns nil for invalid fd" do
      state = fresh_fs()
      {result, _state} = FileSystem.read(state, 999, 10)
      assert result == nil
    end

    test "write returns nil for invalid fd" do
      state = fresh_fs()
      {result, _state} = FileSystem.write(state, 999, "data")
      assert result == nil
    end

    test "mkdir fails with empty name" do
      state = fresh_fs()
      {:error, _state} = FileSystem.mkdir(state, "/")
    end

    test "unlink fails with empty name" do
      state = fresh_fs()
      {:error, _state} = FileSystem.unlink(state, "/")
    end

    test "readdir returns nil for a file (not directory)" do
      state = fresh_fs()
      {fd, state} = FileSystem.open(state, "/file.txt", [:wronly, :creat])
      {:ok, state} = FileSystem.close(state, fd)
      assert FileSystem.readdir(state, "/file.txt") == nil
    end
  end

  # ============================================================================
  # Integration
  # ============================================================================

  describe "integration" do
    test "full workflow: format → mkdir → write → read → unlink" do
      state = fresh_fs()

      {:ok, state} = FileSystem.mkdir(state, "/home")
      {:ok, state} = FileSystem.mkdir(state, "/home/alice")

      {fd, state} = FileSystem.open(state, "/home/alice/notes.txt", [:rdwr, :creat])
      {_bytes, state} = FileSystem.write(state, fd, "My first note\n")
      {_bytes, state} = FileSystem.write(state, fd, "My second note\n")
      {:ok, state} = FileSystem.close(state, fd)

      {fd2, state} = FileSystem.open(state, "/home/alice/notes.txt", [:rdonly])
      {content, state} = FileSystem.read(state, fd2, 1000)
      assert content == "My first note\nMy second note\n"
      {:ok, state} = FileSystem.close(state, fd2)

      s = FileSystem.stat(state, "/home/alice/notes.txt")
      assert s.file_type == :regular
      assert s.size == 29

      entries = FileSystem.readdir(state, "/home/alice")
      names = Enum.map(entries, & &1.name)
      assert "notes.txt" in names

      {:ok, state} = FileSystem.unlink(state, "/home/alice/notes.txt")
      assert FileSystem.stat(state, "/home/alice/notes.txt") == nil
    end
  end

  # ============================================================================
  # Block Bitmap
  # ============================================================================

  describe "block bitmap" do
    test "is_block_free reports correctly" do
      state = fresh_fs()
      # Block 0 is used by root directory
      assert FileSystem.is_block_free(state, 0) == false
      # Block 1 should be free
      assert FileSystem.is_block_free(state, 1) == true
    end
  end

  # ============================================================================
  # Inode Table
  # ============================================================================

  describe "inode table" do
    test "get_inode returns inode for valid number" do
      state = fresh_fs()
      inode = FileSystem.get_inode(state, 0)
      assert inode != nil
      assert inode.file_type == :directory
    end

    test "get_inode returns nil for free slot" do
      state = fresh_fs()
      assert FileSystem.get_inode(state, 1) == nil
    end
  end

  # ============================================================================
  # Constants
  # ============================================================================

  describe "constants" do
    test "block_size is 512" do
      assert FileSystem.block_size() == 512
    end

    test "max_blocks is 512" do
      assert FileSystem.max_blocks() == 512
    end

    test "max_inodes is 128" do
      assert FileSystem.max_inodes() == 128
    end

    test "direct_blocks_count is 12" do
      assert FileSystem.direct_blocks_count() == 12
    end

    test "root_inode is 0" do
      assert FileSystem.root_inode() == 0
    end
  end
end
