# frozen_string_literal: true

require_relative "test_helper"

# = InodeTable Tests
#
# The inode table manages the fixed-size array of inodes. These tests verify:
#   1. Allocation returns inodes with unique numbers
#   2. Freeing an inode makes it available for reuse
#   3. Exhaustion: allocating all inodes, then next returns nil
#   4. get() returns the correct inode or nil for out-of-range

class TestInodeTable < Minitest::Test
  include CodingAdventures::FileSystem

  def setup
    @table = InodeTable.new(5)  # Small table for testing
  end

  def test_allocate_returns_unique_inodes
    i1 = @table.allocate(FILE_TYPE_REGULAR)
    i2 = @table.allocate(FILE_TYPE_DIRECTORY)
    refute_nil i1
    refute_nil i2
    refute_equal i1.inode_number, i2.inode_number
    assert_equal FILE_TYPE_REGULAR, i1.file_type
    assert_equal FILE_TYPE_DIRECTORY, i2.file_type
  end

  def test_allocate_sets_initial_values
    inode = @table.allocate(FILE_TYPE_REGULAR)
    assert_equal 0, inode.size
    assert_equal 0o755, inode.permissions
    assert_equal 0, inode.link_count
    assert_nil inode.indirect_block
  end

  def test_free_makes_inode_available
    inode = @table.allocate(FILE_TYPE_REGULAR)
    num = inode.inode_number
    @table.free(num)
    freed = @table.get(num)
    assert freed.free?, "Freed inode should be free"
  end

  def test_free_reuses_slot
    # Allocate all 5 slots
    5.times { @table.allocate(FILE_TYPE_REGULAR) }
    assert_nil @table.allocate(FILE_TYPE_REGULAR)

    # Free slot 2
    @table.free(2)

    # Should be able to allocate again (gets slot 2)
    inode = @table.allocate(FILE_TYPE_DIRECTORY)
    refute_nil inode
    assert_equal 2, inode.inode_number
  end

  def test_exhaustion_returns_nil
    5.times { @table.allocate(FILE_TYPE_REGULAR) }
    assert_nil @table.allocate(FILE_TYPE_REGULAR)
  end

  def test_get_returns_correct_inode
    inode = @table.allocate(FILE_TYPE_REGULAR)
    retrieved = @table.get(inode.inode_number)
    assert_equal inode.inode_number, retrieved.inode_number
    assert_equal FILE_TYPE_REGULAR, retrieved.file_type
  end

  def test_get_out_of_range_returns_nil
    assert_nil @table.get(-1)
    assert_nil @table.get(5)
    assert_nil @table.get(100)
  end

  def test_free_count
    assert_equal 5, @table.free_count
    @table.allocate(FILE_TYPE_REGULAR)
    assert_equal 4, @table.free_count
    @table.allocate(FILE_TYPE_REGULAR)
    assert_equal 3, @table.free_count
  end

  def test_free_invalid_raises
    assert_raises(ArgumentError) { @table.free(-1) }
    assert_raises(ArgumentError) { @table.free(5) }
  end
end
