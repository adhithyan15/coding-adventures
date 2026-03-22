# frozen_string_literal: true

require_relative "test_helper"

# = Inode Tests
#
# The inode is the core data structure — every file and directory has one.
# These tests verify:
#   1. Default state: a new inode is free (FILE_TYPE_NONE)
#   2. Type predicates: free?, directory?, regular?
#   3. Direct blocks are initialized to nil
#   4. Timestamps are set on creation

class TestInode < Minitest::Test
  include CodingAdventures::FileSystem

  def test_default_inode_is_free
    inode = Inode.new(0)
    assert inode.free?, "New inode should be free"
    assert_equal FILE_TYPE_NONE, inode.file_type
  end

  def test_directory_inode
    inode = Inode.new(5, FILE_TYPE_DIRECTORY)
    assert inode.directory?
    refute inode.regular?
    refute inode.free?
  end

  def test_regular_file_inode
    inode = Inode.new(10, FILE_TYPE_REGULAR)
    assert inode.regular?
    refute inode.directory?
    refute inode.free?
  end

  def test_default_fields
    inode = Inode.new(7)
    assert_equal 7, inode.inode_number
    assert_equal 0, inode.size
    assert_equal 0o755, inode.permissions
    assert_equal 0, inode.owner_pid
    assert_equal 0, inode.link_count
    assert_equal DIRECT_BLOCKS, inode.direct_blocks.length
    assert_nil inode.indirect_block
  end

  def test_direct_blocks_initialized_to_nil
    inode = Inode.new(0)
    inode.direct_blocks.each do |block|
      assert_nil block, "Direct blocks should all be nil initially"
    end
  end

  def test_timestamps_set_on_creation
    before = Time.now
    inode = Inode.new(0)
    after = Time.now

    assert inode.created_at >= before
    assert inode.created_at <= after
    assert inode.modified_at >= before
    assert inode.accessed_at >= before
  end

  def test_inode_number_stored
    inode = Inode.new(42, FILE_TYPE_REGULAR)
    assert_equal 42, inode.inode_number
  end
end
