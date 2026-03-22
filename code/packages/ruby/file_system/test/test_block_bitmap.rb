# frozen_string_literal: true

require_relative "test_helper"

# = BlockBitmap Tests
#
# The block bitmap tracks which data blocks are free and which are in use.
# These tests verify:
#   1. All blocks start free
#   2. Allocation marks blocks as used and returns sequential numbers
#   3. Freeing a block makes it available again
#   4. Exhaustion: allocating all blocks, then next allocation returns nil
#   5. Invalid block numbers raise errors

class TestBlockBitmap < Minitest::Test
  include CodingAdventures::FileSystem

  def setup
    @bitmap = BlockBitmap.new(10)
  end

  def test_all_blocks_start_free
    assert_equal 10, @bitmap.free_count
    10.times { |i| assert @bitmap.free?(i), "Block #{i} should start free" }
  end

  def test_allocate_returns_sequential_blocks
    first = @bitmap.allocate
    second = @bitmap.allocate
    assert_equal 0, first
    assert_equal 1, second
    refute @bitmap.free?(0)
    refute @bitmap.free?(1)
    assert_equal 8, @bitmap.free_count
  end

  def test_free_makes_block_available
    block = @bitmap.allocate
    refute @bitmap.free?(block)
    @bitmap.free(block)
    assert @bitmap.free?(block)
    assert_equal 10, @bitmap.free_count
  end

  def test_allocate_reuses_freed_blocks
    # Allocate all 10 blocks
    blocks = 10.times.map { @bitmap.allocate }
    assert_equal 0, @bitmap.free_count

    # Free block 3
    @bitmap.free(3)
    assert_equal 1, @bitmap.free_count

    # Next allocation should reuse block 3
    reused = @bitmap.allocate
    assert_equal 3, reused
  end

  def test_exhaustion_returns_nil
    10.times { @bitmap.allocate }
    assert_nil @bitmap.allocate, "Should return nil when all blocks are used"
    assert_equal 0, @bitmap.free_count
  end

  def test_mark_used
    @bitmap.mark_used(5)
    refute @bitmap.free?(5)
    assert_equal 9, @bitmap.free_count
  end

  def test_invalid_block_number_raises
    assert_raises(ArgumentError) { @bitmap.free(-1) }
    assert_raises(ArgumentError) { @bitmap.free(10) }
    assert_raises(ArgumentError) { @bitmap.free?(10) }
  end
end
