# frozen_string_literal: true

require_relative "test_helper"

# = Superblock Tests
#
# The superblock is the "table of contents" for the file system. These tests
# verify that:
#   1. A freshly created superblock has the correct default values
#   2. The magic number validation works (valid vs. corrupted)
#   3. Custom sizes can be specified

class TestSuperblock < Minitest::Test
  include CodingAdventures::FileSystem

  def test_default_values
    sb = Superblock.new
    assert_equal MAGIC, sb.magic
    assert_equal BLOCK_SIZE, sb.block_size
    assert_equal MAX_BLOCKS, sb.total_blocks
    assert_equal MAX_INODES, sb.total_inodes
    assert_equal 0, sb.free_blocks
    assert_equal MAX_INODES, sb.free_inodes
    assert_equal ROOT_INODE, sb.root_inode
  end

  def test_valid_magic
    sb = Superblock.new
    assert sb.valid?, "Superblock with correct magic should be valid"
  end

  def test_invalid_magic
    sb = Superblock.new
    sb.magic = 0xDEADBEEF
    refute sb.valid?, "Superblock with wrong magic should be invalid"
  end

  def test_custom_sizes
    sb = Superblock.new(total_blocks: 1024, total_inodes: 256)
    assert_equal 1024, sb.total_blocks
    assert_equal 256, sb.total_inodes
    assert_equal 256, sb.free_inodes
  end

  def test_free_counts_are_mutable
    sb = Superblock.new
    sb.free_blocks = 400
    sb.free_inodes = 100
    assert_equal 400, sb.free_blocks
    assert_equal 100, sb.free_inodes
  end
end
