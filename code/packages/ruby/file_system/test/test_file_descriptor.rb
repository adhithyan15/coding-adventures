# frozen_string_literal: true

require_relative "test_helper"

# = File Descriptor Tests
#
# File descriptors are the process's handle to an open file. These tests
# cover all three components:
#   - OpenFile: the system-wide entry for one opening of a file
#   - OpenFileTable: the system-wide table of all open files
#   - FileDescriptorTable: the per-process mapping of fd numbers to entries

class TestOpenFile < Minitest::Test
  include CodingAdventures::FileSystem

  def test_readable_flags
    # O_RDONLY (0) is readable
    assert OpenFile.new(0, O_RDONLY).readable?
    # O_WRONLY (1) is not readable
    refute OpenFile.new(0, O_WRONLY).readable?
    # O_RDWR (2) is readable
    assert OpenFile.new(0, O_RDWR).readable?
  end

  def test_writable_flags
    # O_RDONLY (0) is not writable
    refute OpenFile.new(0, O_RDONLY).writable?
    # O_WRONLY (1) is writable
    assert OpenFile.new(0, O_WRONLY).writable?
    # O_RDWR (2) is writable
    assert OpenFile.new(0, O_RDWR).writable?
  end

  def test_initial_values
    of = OpenFile.new(42, O_RDWR)
    assert_equal 42, of.inode_number
    assert_equal 0, of.offset
    assert_equal O_RDWR, of.flags
    assert_equal 1, of.ref_count
  end
end

class TestOpenFileTable < Minitest::Test
  include CodingAdventures::FileSystem

  def setup
    @table = OpenFileTable.new
  end

  def test_open_returns_unique_indices
    idx1 = @table.open(10, O_RDONLY)
    idx2 = @table.open(20, O_WRONLY)
    refute_equal idx1, idx2
  end

  def test_get_returns_entry
    idx = @table.open(10, O_RDONLY)
    entry = @table.get(idx)
    refute_nil entry
    assert_equal 10, entry.inode_number
    assert_equal O_RDONLY, entry.flags
  end

  def test_close_decrements_ref_count
    idx = @table.open(10, O_RDONLY)
    fully_closed = @table.close(idx)
    assert fully_closed, "Should be fully closed when ref_count hits 0"
    assert_nil @table.get(idx)
  end

  def test_close_with_multiple_refs
    idx = @table.open(10, O_RDONLY)
    @table.dup(idx)  # ref_count = 2
    refute @table.close(idx), "Should not be fully closed with ref_count > 0"
    refute_nil @table.get(idx)
  end

  def test_dup_increments_ref_count
    idx = @table.open(10, O_RDONLY)
    @table.dup(idx)
    entry = @table.get(idx)
    assert_equal 2, entry.ref_count
  end

  def test_slot_reuse_after_close
    idx1 = @table.open(10, O_RDONLY)
    @table.close(idx1)
    idx2 = @table.open(20, O_WRONLY)
    # Should reuse the freed slot
    assert_equal idx1, idx2
  end

  def test_close_invalid_returns_false
    refute @table.close(-1)
    refute @table.close(999)
  end

  def test_get_invalid_returns_nil
    assert_nil @table.get(-1)
    assert_nil @table.get(999)
  end

  def test_dup_invalid_returns_nil
    assert_nil @table.dup(999)
  end
end

class TestFileDescriptorTable < Minitest::Test
  include CodingAdventures::FileSystem

  def setup
    @table = FileDescriptorTable.new
  end

  def test_allocate_returns_sequential_fds
    fd1 = @table.allocate(100)
    fd2 = @table.allocate(200)
    assert_equal 0, fd1
    assert_equal 1, fd2
  end

  def test_get_returns_global_fd
    @table.allocate(100)
    assert_equal 100, @table.get(0)
  end

  def test_close_removes_mapping
    @table.allocate(100)
    @table.close(0)
    assert_nil @table.get(0)
  end

  def test_close_allows_fd_reuse
    @table.allocate(100)
    @table.allocate(200)
    @table.close(0)
    # Next allocate should reuse fd 0
    fd = @table.allocate(300)
    assert_equal 0, fd
  end

  def test_dup_fd
    @table.allocate(100)
    new_fd = @table.dup_fd(0)
    refute_nil new_fd
    refute_equal 0, new_fd
    # Both fds should map to the same global fd
    assert_equal @table.get(0), @table.get(new_fd)
  end

  def test_dup_fd_invalid_returns_nil
    assert_nil @table.dup_fd(999)
  end

  def test_dup2
    @table.allocate(100)
    @table.allocate(200)
    result = @table.dup2(0, 5)
    assert_equal 5, result
    assert_equal @table.get(0), @table.get(5)
  end

  def test_dup2_closes_existing
    @table.allocate(100)
    @table.allocate(200)
    # dup2(0, 1) should close fd 1 and point it to fd 0's entry
    @table.dup2(0, 1)
    assert_equal @table.get(0), @table.get(1)
  end

  def test_dup2_invalid_returns_nil
    assert_nil @table.dup2(999, 5)
  end

  def test_clone_table
    @table.allocate(100)
    @table.allocate(200)
    cloned = @table.clone_table
    assert_equal @table.get(0), cloned.get(0)
    assert_equal @table.get(1), cloned.get(1)
  end

  def test_entries
    @table.allocate(100)
    @table.allocate(200)
    e = @table.entries
    assert_equal({0 => 100, 1 => 200}, e)
  end
end
